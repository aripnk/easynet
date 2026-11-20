//! One-socket STUN Binding client.

use std::io;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use easynet_core::{Class, binding_request, classify, parse_binding_success};
use tokio::net::UdpSocket;
use tokio::time::{Duration, timeout};

/// STUN exchange failed.
#[derive(Debug)]
pub enum StunIoError {
    /// No Binding Success arrived within 2 seconds.
    Timeout,
    /// The socket could not bind, send, or receive.
    Io(io::Error),
}

impl std::fmt::Display for StunIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "stun timed out"),
            Self::Io(err) => write!(f, "{err}"),
        }
    }
}

impl From<io::Error> for StunIoError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

/// Bind one UDP socket, send one Binding Request, and return the reflexive address.
pub async fn binding_reflexive(
    bind: SocketAddr,
    server: SocketAddr,
) -> Result<SocketAddr, StunIoError> {
    let socket = UdpSocket::bind(bind).await?;
    let tx_id = transaction_id(bind);
    let request = binding_request(tx_id);
    socket.send_to(&request, server).await?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let mut buf = [0u8; 1500];
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(StunIoError::Timeout);
        }
        let received = match timeout(remaining, socket.recv(&mut buf)).await {
            Ok(Ok(n)) => n,
            Ok(Err(err)) => return Err(StunIoError::Io(err)),
            Err(_) => return Err(StunIoError::Timeout),
        };
        if classify(&buf[..received]) != Class::Stun {
            continue;
        }
        if let Ok(addr) = parse_binding_success(&buf[..received], tx_id) {
            return Ok(addr);
        }
    }
}

/// Run [`binding_reflexive`] on a multi-thread runtime.
pub fn binding_reflexive_blocking(
    bind: SocketAddr,
    server: SocketAddr,
) -> Result<SocketAddr, StunIoError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()?;
    runtime.block_on(binding_reflexive(bind, server))
}

fn transaction_id(bind: SocketAddr) -> [u8; 12] {
    let mut id = [0u8; 12];
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    id[..8].copy_from_slice(&nanos.to_le_bytes()[..8]);
    id[8..10].copy_from_slice(&bind.port().to_be_bytes());
    id
}
