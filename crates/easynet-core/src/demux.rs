//! UDP demux from api §1. STUN is decided before mesh type bytes.

/// What a datagram on the primary socket is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Shorter than 8 bytes.
    TooShort,
    /// RFC 5389 magic cookie at bytes 4..8.
    Stun,
    /// Mesh type `0x01`.
    Punch,
    /// Mesh type `0x02`.
    Gossip,
    /// Mesh type `0x03`.
    Data,
    /// Mesh type `0x04`.
    Relay,
    /// Mesh type `0x05`.
    Handshake,
    /// Any other first byte.
    Unknown,
}

const COOKIE: [u8; 4] = [0x21, 0x12, 0xA4, 0x42];

/// Classify one datagram. No I/O.
pub fn classify(buf: &[u8]) -> Class {
    if buf.len() < 8 {
        return Class::TooShort;
    }
    if buf.len() >= 20 && buf[4..8] == COOKIE {
        return Class::Stun;
    }
    match buf[0] {
        0x01 => Class::Punch,
        0x02 => Class::Gossip,
        0x03 => Class::Data,
        0x04 => Class::Relay,
        0x05 => Class::Handshake,
        _ => Class::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::{Class, classify};

    #[test]
    fn stun_success_with_punch_first_byte_is_stun() {
        let mut buf = vec![0x01, 0x01, 0x00, 0x00, 0x21, 0x12, 0xA4, 0x42];
        buf.extend_from_slice(&[0; 12]);
        assert_eq!(classify(&buf), Class::Stun);
    }

    #[test]
    fn punch_when_cookie_is_absent() {
        let mut buf = vec![0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        buf.extend_from_slice(&[0; 12]);
        assert_eq!(classify(&buf), Class::Punch);
    }

    #[test]
    fn short_and_unknown() {
        assert_eq!(classify(&[0; 7]), Class::TooShort);
        assert_eq!(classify(&[0x09, 0, 0, 0, 0, 0, 0, 0]), Class::Unknown);
    }
}
