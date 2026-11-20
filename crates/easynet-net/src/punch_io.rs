//! Invite files and one-socket PUNCH. The binary generates the keys.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use easynet_core::{
    AddrError, Class, InviteError, InviteToken, KEEPALIVE_MS, PROBE_MS, PUNCH_ACK, PUNCH_SYN,
    PunchCache, binding_request, classify, decode_invite, due, ed25519_public, encode_invite,
    encode_punch, mesh_id, pack_socket_addr, unpack_socket_addr, verify_punch, x25519_public,
};
use tokio::net::UdpSocket;
use tokio::time::{Duration, Instant, timeout};

const NODE_KEY: &str = "node.key";
const ROOT_KEY: &str = "root.key";
const PUNCH_LIMIT: Duration = Duration::from_secs(5);

/// Punch or invite I/O failed.
#[derive(Debug)]
pub enum PunchIoError {
    /// No ACK arrived within 5 seconds.
    Timeout,
    /// The invite has no candidate.
    NoCandidate,
    /// A stored key file has the wrong length.
    BadKey,
    /// Invite parse or signature failed.
    Invite(InviteError),
    /// A candidate was not a packed address.
    Addr(AddrError),
    /// Disk or socket I/O failed.
    Io(io::Error),
    /// A second ACK arrived after the keepalive.
    ExtraAck,
    /// The hold ended before a keepalive was sent.
    NoKeepalive,
}

impl std::fmt::Display for PunchIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "punch timed out"),
            Self::NoCandidate => write!(f, "invite has no candidate"),
            Self::BadKey => write!(f, "key file has the wrong length"),
            Self::Invite(err) => write!(f, "invite error {err:?}"),
            Self::Addr(err) => write!(f, "address error {err:?}"),
            Self::Io(err) => write!(f, "{err}"),
            Self::ExtraAck => write!(f, "extra ack"),
            Self::NoKeepalive => write!(f, "keepalive was not sent"),
        }
    }
}

impl From<io::Error> for PunchIoError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

struct NodeSecrets {
    ed25519: [u8; 32],
    x25519: [u8; 32],
    root: [u8; 32],
}

/// Create `node.key` and `root.key` when missing, then print-ready invite URI.
pub fn issue_invite(state_dir: &Path, candidate: SocketAddr) -> Result<String, PunchIoError> {
    let secrets = load_or_create(state_dir)?;
    let root_pub = ed25519_public(&secrets.root);
    let token = InviteToken {
        token_type: 0x01,
        version: 0x01,
        mesh_id: mesh_id(&root_pub),
        root_pub_key: root_pub,
        peer_ed25519_pub: ed25519_public(&secrets.ed25519),
        peer_x25519_pub: x25519_public(&secrets.x25519),
        candidates: vec![pack_socket_addr(candidate)],
        issued_at: unix_ms(),
    };
    encode_invite(&token, &secrets.ed25519).map_err(PunchIoError::Invite)
}

/// Bind one socket, probe each candidate, and finish after the first ACK.
///
/// `hold_secs == 0` returns after that ACK. `hold_secs > 0` stays up, sends one
/// ACK-only keepalive at 15 seconds, and does not answer that kind of frame.
/// `on_ack` runs once for the first valid ACK. `on_keepalive` runs once when
/// that keepalive frame is sent.
pub async fn punch_until_ack(
    state_dir: &Path,
    bind: SocketAddr,
    peer_uri: &str,
    hold_secs: u64,
    stun_server: Option<SocketAddr>,
    mut on_ack: impl FnMut(),
    mut on_keepalive: impl FnMut(),
) -> Result<(), PunchIoError> {
    let secrets = load_or_create(state_dir)?;
    let invite = decode_invite(peer_uri).map_err(PunchIoError::Invite)?;
    let targets = peer_targets(&invite)?;
    let socket = UdpSocket::bind(bind).await?;
    let endpoint = pack_socket_addr(bind);
    let mut cache = PunchCache::new();
    let deadline = Instant::now() + PUNCH_LIMIT;
    let mut next_probe = Instant::now();
    let mut acked_at_ms: Option<u64> = None;
    let mut peer_src: Option<SocketAddr> = None;
    let mut last_stun_ms = unix_ms();
    let mut keepalive_sent = false;
    let mut acks_after_keepalive = 0u8;
    // `hold_secs == 0` still answers one in-flight SYN so the peer can finish.
    let mut ack_grace: Option<Instant> = None;
    let mut hold_deadline: Option<Instant> = None;
    let mut buf = [0u8; 1500];

    loop {
        if let Some(grace) = ack_grace {
            if Instant::now() >= grace {
                return Ok(());
            }
        } else if let Some(hold) = hold_deadline {
            if Instant::now() >= hold {
                if keepalive_sent {
                    return Ok(());
                }
                return Err(PunchIoError::NoKeepalive);
            }
        } else if Instant::now() >= deadline {
            return Err(PunchIoError::Timeout);
        }

        let now_ms = unix_ms();
        if acked_at_ms.is_none() && Instant::now() >= next_probe {
            send_syns(&socket, &secrets.ed25519, &endpoint, &targets).await?;
            next_probe = Instant::now() + Duration::from_millis(PROBE_MS);
        }
        if acked_at_ms.is_none() {
            if let Some(server) = stun_server {
                if due(last_stun_ms, now_ms, KEEPALIVE_MS) {
                    send_stun(&socket, server).await?;
                    last_stun_ms = unix_ms();
                }
            }
        }
        if hold_deadline.is_some()
            && !keepalive_sent
            && due(acked_at_ms.unwrap_or(now_ms), now_ms, KEEPALIVE_MS)
        {
            let dest = peer_src.unwrap_or(targets[0]);
            send_keepalive(&socket, &secrets.ed25519, &endpoint, dest).await?;
            keepalive_sent = true;
            on_keepalive();
        }

        let wait_for = if let Some(grace) = ack_grace {
            grace
        } else if let Some(hold) = hold_deadline {
            let next_keep = if keepalive_sent {
                hold
            } else {
                Instant::now() + Duration::from_millis(PROBE_MS)
            };
            hold.min(next_keep)
        } else {
            let stun_wait = if stun_server.is_some() {
                Instant::now() + Duration::from_millis(PROBE_MS)
            } else {
                deadline
            };
            next_probe.min(deadline).min(stun_wait)
        };
        let wait = wait_for.saturating_duration_since(Instant::now());
        if wait.is_zero() {
            continue;
        }
        match timeout(wait, socket.recv_from(&mut buf)).await {
            Ok(Ok((n, src))) => {
                match handle_datagram(
                    &socket,
                    &buf[..n],
                    src,
                    &invite.peer_ed25519_pub,
                    &secrets.ed25519,
                    &endpoint,
                    &mut cache,
                )
                .await?
                {
                    PacketKind::Ack => {
                        if acked_at_ms.is_none() {
                            acked_at_ms = Some(unix_ms());
                            peer_src = Some(src);
                            on_ack();
                            if hold_secs == 0 {
                                ack_grace.get_or_insert_with(|| {
                                    Instant::now() + Duration::from_millis(300)
                                });
                            } else {
                                hold_deadline.get_or_insert_with(|| {
                                    Instant::now() + Duration::from_secs(hold_secs)
                                });
                            }
                        } else if keepalive_sent {
                            acks_after_keepalive = acks_after_keepalive.saturating_add(1);
                            if acks_after_keepalive > 1 {
                                return Err(PunchIoError::ExtraAck);
                            }
                        }
                    }
                    PacketKind::Ignored => {}
                }
            }
            Ok(Err(err)) => return Err(PunchIoError::Io(err)),
            Err(_) => {}
        }
    }
}

/// Run [`punch_until_ack`] on a current-thread runtime.
pub fn punch_until_ack_blocking(
    state_dir: &Path,
    bind: SocketAddr,
    peer_uri: &str,
    hold_secs: u64,
    stun_server: Option<SocketAddr>,
    on_ack: impl FnMut(),
    on_keepalive: impl FnMut(),
) -> Result<(), PunchIoError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(punch_until_ack(
        state_dir,
        bind,
        peer_uri,
        hold_secs,
        stun_server,
        on_ack,
        on_keepalive,
    ))
}

async fn send_syns(
    socket: &UdpSocket,
    secret: &[u8; 32],
    endpoint: &[u8],
    targets: &[SocketAddr],
) -> Result<(), PunchIoError> {
    let unix_ms = unix_ms();
    for target in targets {
        let frame = encode_punch(PUNCH_SYN, secret, unix_ms, random_u64()?, endpoint);
        socket.send_to(&frame, target).await?;
    }
    Ok(())
}

enum PacketKind {
    Ignored,
    Ack,
}

/// An ACK-only frame is not answered. A SYN still is, until the path is up.
async fn handle_datagram(
    socket: &UdpSocket,
    packet: &[u8],
    src: SocketAddr,
    peer_ed25519: &[u8; 32],
    secret: &[u8; 32],
    endpoint: &[u8],
    cache: &mut PunchCache,
) -> Result<PacketKind, PunchIoError> {
    if classify(packet) != Class::Punch {
        return Ok(PacketKind::Ignored);
    }
    let verified = match verify_punch(packet, peer_ed25519, unix_ms(), cache) {
        Ok(verified) => verified,
        Err(_) => return Ok(PacketKind::Ignored),
    };
    if verified.flags & PUNCH_ACK != 0 {
        return Ok(PacketKind::Ack);
    }
    if verified.flags & PUNCH_SYN != 0 {
        let frame = encode_punch(PUNCH_ACK, secret, unix_ms(), random_u64()?, endpoint);
        socket.send_to(&frame, src).await?;
    }
    Ok(PacketKind::Ignored)
}

async fn send_keepalive(
    socket: &UdpSocket,
    secret: &[u8; 32],
    endpoint: &[u8],
    dest: SocketAddr,
) -> Result<(), PunchIoError> {
    let frame = encode_punch(PUNCH_ACK, secret, unix_ms(), random_u64()?, endpoint);
    socket.send_to(&frame, dest).await?;
    Ok(())
}

async fn send_stun(socket: &UdpSocket, server: SocketAddr) -> Result<(), PunchIoError> {
    let mut tx_id = [0u8; 12];
    random_bytes(&mut tx_id)?;
    socket.send_to(&binding_request(tx_id), server).await?;
    Ok(())
}

fn peer_targets(invite: &InviteToken) -> Result<Vec<SocketAddr>, PunchIoError> {
    if invite.candidates.is_empty() {
        return Err(PunchIoError::NoCandidate);
    }
    invite
        .candidates
        .iter()
        .map(|packed| unpack_socket_addr(packed).map_err(PunchIoError::Addr))
        .collect()
}

fn load_or_create(state_dir: &Path) -> Result<NodeSecrets, PunchIoError> {
    fs::create_dir_all(state_dir)?;
    let node = read_or_create(&state_dir.join(NODE_KEY), 64)?;
    let root = read_or_create(&state_dir.join(ROOT_KEY), 32)?;
    let mut ed25519 = [0u8; 32];
    let mut x25519 = [0u8; 32];
    let mut root_secret = [0u8; 32];
    ed25519.copy_from_slice(&node[..32]);
    x25519.copy_from_slice(&node[32..]);
    root_secret.copy_from_slice(&root);
    Ok(NodeSecrets {
        ed25519,
        x25519,
        root: root_secret,
    })
}

fn read_or_create(path: &Path, len: usize) -> Result<Vec<u8>, PunchIoError> {
    if path.exists() {
        let bytes = fs::read(path)?;
        if bytes.len() != len {
            return Err(PunchIoError::BadKey);
        }
        return Ok(bytes);
    }
    let mut bytes = vec![0u8; len];
    random_bytes(&mut bytes)?;
    write_secret(path, &bytes)?;
    Ok(bytes)
}

fn write_secret(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    Ok(())
}

fn random_u64() -> Result<u64, PunchIoError> {
    let mut buf = [0u8; 8];
    random_bytes(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn random_bytes(buf: &mut [u8]) -> io::Result<()> {
    #[cfg(unix)]
    {
        let mut file = File::open("/dev/urandom")?;
        file.read_exact(buf)?;
        Ok(())
    }
    #[cfg(windows)]
    {
        windows_random(buf)
    }
}

#[cfg(windows)]
fn windows_random(buf: &mut [u8]) -> io::Result<()> {
    #[link(name = "bcrypt")]
    unsafe extern "system" {
        fn BCryptGenRandom(alg: isize, pb: *mut u8, cb: u32, flags: u32) -> i32;
    }
    let status = unsafe { BCryptGenRandom(0, buf.as_mut_ptr(), buf.len() as u32, 2) };
    if status == 0 {
        Ok(())
    } else {
        Err(io::Error::other("bcrypt rng failed"))
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{issue_invite, punch_until_ack};
    use easynet_core::decode_invite;
    use std::env;
    use std::fs;
    use std::net::SocketAddr;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn scratch() -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let path = env::temp_dir().join(format!(
            "easynet-punch-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn issue_reuses_generated_keys() {
        let dir = scratch();
        let candidate: SocketAddr = "203.0.113.5:51820".parse().unwrap();
        let first = issue_invite(&dir, candidate).unwrap();
        let second = issue_invite(&dir, candidate).unwrap();
        let a = decode_invite(&first).unwrap();
        let b = decode_invite(&second).unwrap();
        assert_eq!(a.peer_ed25519_pub, b.peer_ed25519_pub);
        assert_eq!(a.peer_x25519_pub, b.peer_x25519_pub);
        assert_eq!(a.root_pub_key, b.root_pub_key);
        assert_eq!(a.candidates, b.candidates);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(dir.join("node.key"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn two_local_sockets_exchange_ack() {
        let left_dir = scratch();
        let right_dir = scratch();
        let left_bind: SocketAddr = "127.0.0.1:45121".parse().unwrap();
        let right_bind: SocketAddr = "127.0.0.1:45122".parse().unwrap();
        let left_uri = issue_invite(&left_dir, left_bind).unwrap();
        let right_uri = issue_invite(&right_dir, right_bind).unwrap();
        let left = punch_until_ack(&left_dir, left_bind, &right_uri, 0, None, || {}, || {});
        let right = punch_until_ack(&right_dir, right_bind, &left_uri, 0, None, || {}, || {});
        let (left, right) = tokio::join!(left, right);
        let _ = fs::remove_dir_all(&left_dir);
        let _ = fs::remove_dir_all(&right_dir);
        left.unwrap();
        right.unwrap();
    }
}
