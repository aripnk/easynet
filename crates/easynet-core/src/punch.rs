//! Raw `PUNCH` frames. The signature covers the bytes before the last 64.

use std::collections::{HashMap, VecDeque};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

const HEADER: usize = 36;
const MIN_ENDPOINT: usize = 7;
const SIG_LEN: usize = 64;
const CACHE_CAP: usize = 4096;
const WINDOW_MS: u64 = 30_000;

/// Flags bit0.
pub const PUNCH_SYN: u8 = 0x01;
/// Flags bit1.
pub const PUNCH_ACK: u8 = 0x02;

/// Why a `PUNCH` frame was dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PunchError {
    /// Shorter than `36 + 7 + 64` bytes.
    Short,
    /// Type byte is not `0x01`.
    BadType,
    /// Version byte is not `1`.
    BadVersion,
    /// Reserved byte is not `0`.
    Reserved,
    /// `sender_node_id` is not the first 16 bytes of `SHA256(ed25519_pub)`.
    NodeMismatch,
    /// Ed25519 signature over the leading bytes failed.
    BadSignature,
    /// `|unix_ms - now_ms| > 30_000`.
    Skew,
    /// `(sender_node_id, nonce)` is still inside the 30 s cache.
    Replay,
}

/// Flags and the packed endpoint from a verified frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPunch {
    pub flags: u8,
    pub endpoint: Vec<u8>,
}

/// LRU of 4096 `(sender_node_id, nonce)` pairs, each kept 30_000 ms.
#[derive(Debug, Default)]
pub struct PunchCache {
    order: VecDeque<([u8; 16], u64)>,
    seen_ms: HashMap<([u8; 16], u64), u64>,
}

impl PunchCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a pair, or reject it for skew or replay. Does not read a clock.
    pub fn admit(
        &mut self,
        now_ms: u64,
        frame_unix_ms: u64,
        sender: [u8; 16],
        nonce: u64,
    ) -> Result<(), PunchError> {
        if now_ms.abs_diff(frame_unix_ms) > WINDOW_MS {
            return Err(PunchError::Skew);
        }
        let key = (sender, nonce);
        if let Some(&seen) = self.seen_ms.get(&key) {
            if now_ms.saturating_sub(seen) <= WINDOW_MS {
                self.touch(key);
                return Err(PunchError::Replay);
            }
            self.remove(key);
        }
        if self.seen_ms.len() == CACHE_CAP {
            if let Some(oldest) = self.order.pop_front() {
                self.seen_ms.remove(&oldest);
            }
        }
        self.order.push_back(key);
        self.seen_ms.insert(key, now_ms);
        Ok(())
    }

    fn touch(&mut self, key: ([u8; 16], u64)) {
        if let Some(pos) = self.order.iter().position(|item| *item == key) {
            self.order.remove(pos);
            self.order.push_back(key);
        }
    }

    fn remove(&mut self, key: ([u8; 16], u64)) {
        self.seen_ms.remove(&key);
        if let Some(pos) = self.order.iter().position(|item| *item == key) {
            self.order.remove(pos);
        }
    }
}

fn read_u64(frame: &[u8], at: usize) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&frame[at..at + 8]);
    u64::from_le_bytes(buf)
}

/// First 16 bytes of `SHA256(ed25519_pub)`.
pub fn node_id(ed25519_pub: &[u8; 32]) -> [u8; 16] {
    let digest = Sha256::digest(ed25519_pub);
    let mut id = [0u8; 16];
    id.copy_from_slice(&digest[..16]);
    id
}

/// Build a signed frame. `flags` is `PUNCH_SYN`, `PUNCH_ACK`, or both.
pub fn encode_punch(
    flags: u8,
    ed25519_secret: &[u8; 32],
    unix_ms: u64,
    nonce: u64,
    endpoint: &[u8],
) -> Vec<u8> {
    let signing = SigningKey::from_bytes(ed25519_secret);
    let sender = node_id(&signing.verifying_key().to_bytes());
    let mut frame = Vec::with_capacity(HEADER + endpoint.len() + SIG_LEN);
    frame.push(0x01);
    frame.push(1);
    frame.push(flags);
    frame.push(0);
    frame.extend_from_slice(&sender);
    frame.extend_from_slice(&unix_ms.to_le_bytes());
    frame.extend_from_slice(&nonce.to_le_bytes());
    frame.extend_from_slice(endpoint);
    let signature = signing.sign(&frame);
    frame.extend_from_slice(&signature.to_bytes());
    frame
}

/// Check length, version, reserved, node id, signature, skew, then replay.
pub fn verify_punch(
    frame: &[u8],
    ed25519_pub: &[u8; 32],
    now_ms: u64,
    cache: &mut PunchCache,
) -> Result<VerifiedPunch, PunchError> {
    if frame.len() < HEADER + MIN_ENDPOINT + SIG_LEN {
        return Err(PunchError::Short);
    }
    if frame[0] != 0x01 {
        return Err(PunchError::BadType);
    }
    if frame[1] != 1 {
        return Err(PunchError::BadVersion);
    }
    if frame[3] != 0 {
        return Err(PunchError::Reserved);
    }
    let mut sender = [0u8; 16];
    sender.copy_from_slice(&frame[4..20]);
    if sender != node_id(ed25519_pub) {
        return Err(PunchError::NodeMismatch);
    }
    let split = frame.len() - SIG_LEN;
    let (message, sig_bytes) = frame.split_at(split);
    let verifying = VerifyingKey::from_bytes(ed25519_pub).map_err(|_| PunchError::BadSignature)?;
    let signature = Signature::from_slice(sig_bytes).map_err(|_| PunchError::BadSignature)?;
    verifying
        .verify(message, &signature)
        .map_err(|_| PunchError::BadSignature)?;
    let unix_ms = read_u64(frame, 20);
    let nonce = read_u64(frame, 28);
    cache.admit(now_ms, unix_ms, sender, nonce)?;
    Ok(VerifiedPunch {
        flags: frame[2],
        endpoint: frame[HEADER..split].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::{PUNCH_SYN, PunchCache, PunchError, encode_punch, node_id, verify_punch};
    use crate::pack_socket_addr;
    use std::net::SocketAddr;

    const SECRET: [u8; 32] = [7u8; 32];

    fn public() -> [u8; 32] {
        ed25519_dalek::SigningKey::from_bytes(&SECRET)
            .verifying_key()
            .to_bytes()
    }

    fn endpoint() -> Vec<u8> {
        let addr: SocketAddr = "203.0.113.5:51820".parse().unwrap();
        pack_socket_addr(addr)
    }

    #[test]
    fn one_millisecond_inside_the_window_is_accepted() {
        let unix_ms = 1_000_000;
        let frame = encode_punch(PUNCH_SYN, &SECRET, unix_ms, 1, &endpoint());
        let mut cache = PunchCache::new();
        let verified = verify_punch(&frame, &public(), unix_ms + 29_999, &mut cache).unwrap();
        assert_eq!(verified.flags, PUNCH_SYN);
        assert_eq!(verified.endpoint, endpoint());
        assert_eq!(node_id(&public()).len(), 16);
    }

    #[test]
    fn thirty_thousand_and_one_milliseconds_is_skew() {
        let unix_ms = 1_000_000;
        let frame = encode_punch(PUNCH_SYN, &SECRET, unix_ms, 1, &endpoint());
        let mut cache = PunchCache::new();
        assert_eq!(
            verify_punch(&frame, &public(), unix_ms + 30_001, &mut cache),
            Err(PunchError::Skew)
        );
    }

    #[test]
    fn same_sender_and_nonce_inside_thirty_seconds_is_replay() {
        let frame = encode_punch(PUNCH_SYN, &SECRET, 5_000, 9, &endpoint());
        let mut cache = PunchCache::new();
        verify_punch(&frame, &public(), 5_000, &mut cache).unwrap();
        assert_eq!(
            verify_punch(&frame, &public(), 5_000 + 30_000, &mut cache),
            Err(PunchError::Replay)
        );
    }

    #[test]
    fn four_thousand_ninety_seventh_pair_evicts_the_oldest() {
        let mut cache = PunchCache::new();
        let sender = [1u8; 16];
        for nonce in 0..4096 {
            cache.admit(0, 0, sender, nonce).unwrap();
        }
        cache.admit(0, 0, sender, 4096).unwrap();
        assert_eq!(cache.admit(0, 0, sender, 4095), Err(PunchError::Replay));
        assert_eq!(cache.admit(0, 0, sender, 0), Ok(()));
    }

    #[test]
    fn short_frame_and_bad_signature_fail() {
        let frame = encode_punch(PUNCH_SYN, &SECRET, 5_000, 9, &endpoint());
        let mut cache = PunchCache::new();
        assert_eq!(
            verify_punch(&frame[..frame.len() - 1], &public(), 5_000, &mut cache),
            Err(PunchError::Short)
        );
        let mut bad = frame.clone();
        let last = bad.len() - 1;
        bad[last] ^= 0xff;
        assert_eq!(
            verify_punch(&bad, &public(), 5_000, &mut cache),
            Err(PunchError::BadSignature)
        );
    }
}
