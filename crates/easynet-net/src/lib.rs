//! UDP mux and STUN I/O. One primary socket.

mod stun_io;

pub use stun_io::{StunIoError, binding_reflexive, binding_reflexive_blocking};
