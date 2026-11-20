//! Packed `SocketAddrBin` from api §2. No postcard `u16` ports.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

/// A packed address could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddrError {
    /// The buffer is shorter or longer than one address.
    Truncated,
    /// Family is not `0x04` or `0x06`.
    BadFamily,
}

/// Family, address, then port in big-endian.
pub fn pack_socket_addr(addr: SocketAddr) -> Vec<u8> {
    match addr {
        SocketAddr::V4(v4) => {
            let mut out = Vec::with_capacity(7);
            out.push(0x04);
            out.extend_from_slice(&v4.ip().octets());
            out.extend_from_slice(&v4.port().to_be_bytes());
            out
        }
        SocketAddr::V6(v6) => {
            let mut out = Vec::with_capacity(19);
            out.push(0x06);
            out.extend_from_slice(&v6.ip().octets());
            out.extend_from_slice(&v6.port().to_be_bytes());
            out
        }
    }
}

/// Inverse of [`pack_socket_addr`]. Extra trailing bytes are rejected.
pub fn unpack_socket_addr(buf: &[u8]) -> Result<SocketAddr, AddrError> {
    if buf.is_empty() {
        return Err(AddrError::Truncated);
    }
    match buf[0] {
        0x04 => {
            if buf.len() != 7 {
                return Err(AddrError::Truncated);
            }
            let ip = Ipv4Addr::new(buf[1], buf[2], buf[3], buf[4]);
            let port = u16::from_be_bytes([buf[5], buf[6]]);
            Ok(SocketAddr::V4(SocketAddrV4::new(ip, port)))
        }
        0x06 => {
            if buf.len() != 19 {
                return Err(AddrError::Truncated);
            }
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&buf[1..17]);
            let port = u16::from_be_bytes([buf[17], buf[18]]);
            Ok(SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::from(octets),
                port,
                0,
                0,
            )))
        }
        _ => Err(AddrError::BadFamily),
    }
}

#[cfg(test)]
mod tests {
    use super::{AddrError, pack_socket_addr, unpack_socket_addr};
    use std::net::SocketAddr;

    #[test]
    fn ipv4_round_trip_uses_big_endian_port() {
        let addr: SocketAddr = "203.0.113.5:51820".parse().unwrap();
        let packed = pack_socket_addr(addr);
        assert_eq!(packed, [0x04, 203, 0, 113, 5, 0xCA, 0x6C]);
        assert_eq!(unpack_socket_addr(&packed).unwrap(), addr);
    }

    #[test]
    fn truncated_buffer_is_rejected() {
        assert_eq!(
            unpack_socket_addr(&[0x04, 203, 0, 113]),
            Err(AddrError::Truncated)
        );
    }
}
