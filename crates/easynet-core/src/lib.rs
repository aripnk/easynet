//! Pure EasyNet state. No OS or socket I/O.

mod addr;
mod config;
mod demux;
mod identity;
mod invite;
mod punch;
mod schedule;
mod stun;

pub use addr::{AddrError, pack_socket_addr, unpack_socket_addr};
pub use config::{Config, default_toml, parse_toml};
pub use demux::{Class, classify};
pub use identity::{ed25519_public, x25519_public};
pub use invite::{InviteError, InviteToken, decode_invite, encode_invite, mesh_id};
pub use punch::{
    PUNCH_ACK, PUNCH_SYN, PunchCache, PunchError, VerifiedPunch, encode_punch, node_id,
    verify_punch,
};
pub use schedule::{KEEPALIVE_MS, PROBE_MS, due};
pub use stun::{StunError, binding_request, parse_binding_success};
