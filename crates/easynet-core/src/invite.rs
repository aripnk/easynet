//! `easynet://invite/` tokens. Signature is outside the postcard body.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

const PREFIX: &str = "easynet://invite/";
const DOMAIN: &[u8] = b"easynet.invite.v1\0";
const TOKEN_TYPE: u8 = 0x01;
const VERSION: u8 = 0x01;
const MAX_CANDIDATES: usize = 4;

/// Invite body from api §7.1. Field order is the postcard order.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InviteToken {
    pub token_type: u8,
    pub version: u8,
    pub mesh_id: [u8; 8],
    pub root_pub_key: [u8; 32],
    pub peer_ed25519_pub: [u8; 32],
    pub peer_x25519_pub: [u8; 32],
    pub candidates: Vec<Vec<u8>>,
    pub issued_at: u64,
}

/// Why an invite could not be signed or parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteError {
    /// More than four candidates.
    TooManyCandidates,
    /// `peer_ed25519_pub` is not the signing key.
    SignerMismatch,
    /// `token_type` or `version` is not `0x01`, or `mesh_id` does not match the root.
    BadToken,
    /// The URI is not `easynet://invite/…`.
    BadPrefix,
    /// The payload is not unpadded base64url, or it is shorter than a signature.
    BadEncoding,
    /// The Ed25519 signature does not verify.
    BadSignature,
}

/// First 8 bytes of `SHA256(root_pub_key)`.
pub fn mesh_id(root_pub_key: &[u8; 32]) -> [u8; 8] {
    let digest = Sha256::digest(root_pub_key);
    let mut id = [0u8; 8];
    id.copy_from_slice(&digest[..8]);
    id
}

/// Sign `token` and return `easynet://invite/<base64url>`.
pub fn encode_invite(
    token: &InviteToken,
    ed25519_secret: &[u8; 32],
) -> Result<String, InviteError> {
    if token.candidates.len() > MAX_CANDIDATES {
        return Err(InviteError::TooManyCandidates);
    }
    if token.token_type != TOKEN_TYPE || token.version != VERSION {
        return Err(InviteError::BadToken);
    }
    let signing = SigningKey::from_bytes(ed25519_secret);
    if signing.verifying_key().to_bytes() != token.peer_ed25519_pub {
        return Err(InviteError::SignerMismatch);
    }
    let mut body = token.clone();
    body.mesh_id = mesh_id(&body.root_pub_key);
    let payload = sign_body(&body, &signing)?;
    Ok(format!("{PREFIX}{}", b64url_encode(&payload)))
}

/// Parse and verify an invite URI. No expiry check.
pub fn decode_invite(uri: &str) -> Result<InviteToken, InviteError> {
    let Some(encoded) = uri.strip_prefix(PREFIX) else {
        return Err(InviteError::BadPrefix);
    };
    if encoded.is_empty() || encoded.contains('=') {
        return Err(InviteError::BadEncoding);
    }
    let bytes = b64url_decode(encoded).map_err(|_| InviteError::BadEncoding)?;
    let Some(split) = bytes.len().checked_sub(64) else {
        return Err(InviteError::BadEncoding);
    };
    if split == 0 {
        return Err(InviteError::BadEncoding);
    }
    let (body_bytes, sig_bytes) = bytes.split_at(split);
    let token: InviteToken = postcard::from_bytes(body_bytes).map_err(|_| InviteError::BadToken)?;
    if token.token_type != TOKEN_TYPE || token.version != VERSION {
        return Err(InviteError::BadToken);
    }
    if token.candidates.len() > MAX_CANDIDATES {
        return Err(InviteError::TooManyCandidates);
    }
    if token.mesh_id != mesh_id(&token.root_pub_key) {
        return Err(InviteError::BadToken);
    }
    let verifying =
        VerifyingKey::from_bytes(&token.peer_ed25519_pub).map_err(|_| InviteError::BadSignature)?;
    let signature = Signature::from_slice(sig_bytes).map_err(|_| InviteError::BadSignature)?;
    let mut message = Vec::with_capacity(DOMAIN.len() + body_bytes.len());
    message.extend_from_slice(DOMAIN);
    message.extend_from_slice(body_bytes);
    verifying
        .verify(&message, &signature)
        .map_err(|_| InviteError::BadSignature)?;
    Ok(token)
}

fn sign_body(token: &InviteToken, signing: &SigningKey) -> Result<Vec<u8>, InviteError> {
    let body = postcard::to_allocvec(token).map_err(|_| InviteError::BadToken)?;
    let mut message = Vec::with_capacity(DOMAIN.len() + body.len());
    message.extend_from_slice(DOMAIN);
    message.extend_from_slice(&body);
    let signature = signing.sign(&message);
    let mut payload = body;
    payload.extend_from_slice(&signature.to_bytes());
    Ok(payload)
}

fn b64url_encode(data: &[u8]) -> String {
    const ALPH: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = u32::from_be_bytes([0, data[i], data[i + 1], data[i + 2]]);
        for shift in [18, 12, 6, 0] {
            out.push(ALPH[((n >> shift) & 0x3f) as usize] as char);
        }
        i += 3;
    }
    let rest = data.len() - i;
    if rest == 1 {
        let n = u32::from(data[i]) << 16;
        out.push(ALPH[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPH[((n >> 12) & 0x3f) as usize] as char);
    } else if rest == 2 {
        let n = (u32::from(data[i]) << 16) | (u32::from(data[i + 1]) << 8);
        out.push(ALPH[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPH[((n >> 12) & 0x3f) as usize] as char);
        out.push(ALPH[((n >> 6) & 0x3f) as usize] as char);
    }
    out
}

fn b64url_decode(text: &str) -> Result<Vec<u8>, ()> {
    if text.len() % 4 == 1 {
        return Err(());
    }
    let mut bits: u32 = 0;
    let mut nbits: u32 = 0;
    let mut out = Vec::new();
    for byte in text.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return Err(()),
        };
        bits = (bits << 6) | u32::from(value);
        nbits += 6;
        if nbits >= 8 {
            nbits -= 8;
            out.push((bits >> nbits) as u8);
            bits &= (1 << nbits) - 1;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{InviteError, InviteToken, decode_invite, encode_invite};
    use crate::pack_socket_addr;
    use std::net::SocketAddr;
    use x25519_dalek::{PublicKey, StaticSecret};

    const SECRET: [u8; 32] = [7u8; 32];

    fn token() -> InviteToken {
        let signing = ed25519_dalek::SigningKey::from_bytes(&SECRET);
        let x_secret = StaticSecret::from([9u8; 32]);
        let x_public = PublicKey::from(&x_secret);
        let addr: SocketAddr = "203.0.113.5:51820".parse().unwrap();
        InviteToken {
            token_type: 0x01,
            version: 0x01,
            mesh_id: [0; 8],
            root_pub_key: [3u8; 32],
            peer_ed25519_pub: signing.verifying_key().to_bytes(),
            peer_x25519_pub: x_public.to_bytes(),
            candidates: vec![pack_socket_addr(addr)],
            issued_at: 1_700_000_000_000,
        }
    }

    #[test]
    fn ipv4_candidate_round_trips_and_keeps_issued_at() {
        let original = token();
        let uri = encode_invite(&original, &SECRET).unwrap();
        let decoded = decode_invite(&uri).unwrap();
        assert_eq!(decoded.issued_at, original.issued_at);
        assert_eq!(decoded.candidates, original.candidates);
        assert_eq!(decoded.peer_x25519_pub, original.peer_x25519_pub);
        let addr = crate::unpack_socket_addr(&decoded.candidates[0]).unwrap();
        assert_eq!(addr, "203.0.113.5:51820".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn five_candidates_fail() {
        let mut original = token();
        original.candidates = vec![original.candidates[0].clone(); 5];
        assert_eq!(
            encode_invite(&original, &SECRET),
            Err(InviteError::TooManyCandidates)
        );
        original.mesh_id = super::mesh_id(&original.root_pub_key);
        let signing = ed25519_dalek::SigningKey::from_bytes(&SECRET);
        let payload = super::sign_body(&original, &signing).unwrap();
        let uri = format!("easynet://invite/{}", super::b64url_encode(&payload));
        assert_eq!(decode_invite(&uri), Err(InviteError::TooManyCandidates));
    }

    #[test]
    fn flipped_signature_byte_fails() {
        let uri = encode_invite(&token(), &SECRET).unwrap();
        let encoded = uri.strip_prefix("easynet://invite/").unwrap();
        let mut bytes = super::b64url_decode(encoded).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        let uri = format!("easynet://invite/{}", super::b64url_encode(&bytes));
        assert_eq!(decode_invite(&uri), Err(InviteError::BadSignature));
    }

    #[test]
    fn non_invite_uri_fails() {
        assert_eq!(
            decode_invite("easynet://join/aaaa"),
            Err(InviteError::BadPrefix)
        );
    }
}
