//! RFC 5389 Binding Request and Binding Success. No sockets.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

const COOKIE: [u8; 4] = [0x21, 0x12, 0xA4, 0x42];
const BINDING_REQUEST: u16 = 0x0001;
const BINDING_SUCCESS: u16 = 0x0101;
const MAPPED_ADDRESS: u16 = 0x0001;
const XOR_MAPPED_ADDRESS: u16 = 0x0020;
const XOR_PORT: u16 = 0x2112;

/// Why a Binding Success could not be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StunError {
    /// Header, cookie, or declared length does not match a Binding Success.
    BadHeader,
    /// Transaction id differs from the request.
    TransactionMismatch,
    /// An attribute runs past the declared message length.
    Truncated,
    /// Address family is not IPv4 (`0x01`) or IPv6 (`0x02`).
    BadFamily,
    /// Neither XOR-MAPPED-ADDRESS nor MAPPED-ADDRESS is present.
    MissingAddress,
}

/// 20-byte Binding Request. No attributes.
pub fn binding_request(tx_id: [u8; 12]) -> [u8; 20] {
    let mut out = [0u8; 20];
    out[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());
    out[4..8].copy_from_slice(&COOKIE);
    out[8..20].copy_from_slice(&tx_id);
    out
}

/// Parse a Binding Success. Prefers XOR-MAPPED-ADDRESS (`0x0020`).
pub fn parse_binding_success(buf: &[u8], tx_id: [u8; 12]) -> Result<SocketAddr, StunError> {
    if buf.len() < 20 {
        return Err(StunError::BadHeader);
    }
    let msg_type = u16::from_be_bytes([buf[0], buf[1]]);
    let msg_len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
    if msg_type != BINDING_SUCCESS || buf[4..8] != COOKIE {
        return Err(StunError::BadHeader);
    }
    if buf[8..20] != tx_id {
        return Err(StunError::TransactionMismatch);
    }
    let end = 20 + msg_len;
    if buf.len() < end {
        return Err(StunError::Truncated);
    }

    let mut mapped = None;
    let mut xor_mapped = None;
    let mut offset = 20;
    while offset + 4 <= end {
        let attr_type = u16::from_be_bytes([buf[offset], buf[offset + 1]]);
        let attr_len = u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]) as usize;
        let value_at = offset + 4;
        let value_end = value_at + attr_len;
        if value_end > end {
            return Err(StunError::Truncated);
        }
        let pad = (4 - (attr_len % 4)) % 4;
        let next = value_end + pad;
        if next > end {
            return Err(StunError::Truncated);
        }
        if attr_type == XOR_MAPPED_ADDRESS {
            xor_mapped = Some(decode_address(&buf[value_at..value_end], &tx_id, true)?);
        } else if attr_type == MAPPED_ADDRESS {
            mapped = Some(decode_address(&buf[value_at..value_end], &tx_id, false)?);
        }
        offset = next;
    }
    xor_mapped.or(mapped).ok_or(StunError::MissingAddress)
}

fn decode_address(value: &[u8], tx_id: &[u8; 12], xored: bool) -> Result<SocketAddr, StunError> {
    if value.len() < 4 {
        return Err(StunError::Truncated);
    }
    let family = value[1];
    let mut port = u16::from_be_bytes([value[2], value[3]]);
    if xored {
        port ^= XOR_PORT;
    }
    if family == 0x01 {
        if value.len() < 8 {
            return Err(StunError::Truncated);
        }
        let mut addr = [0u8; 4];
        addr.copy_from_slice(&value[4..8]);
        if xored {
            for (byte, mask) in addr.iter_mut().zip(COOKIE) {
                *byte ^= mask;
            }
        }
        Ok(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::from(addr),
            port,
        )))
    } else if family == 0x02 {
        if value.len() < 20 {
            return Err(StunError::Truncated);
        }
        let mut addr = [0u8; 16];
        addr.copy_from_slice(&value[4..20]);
        if xored {
            let mut mask = [0u8; 16];
            mask[..4].copy_from_slice(&COOKIE);
            mask[4..].copy_from_slice(tx_id);
            for (byte, mask_byte) in addr.iter_mut().zip(mask) {
                *byte ^= mask_byte;
            }
        }
        Ok(SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::from(addr),
            port,
            0,
            0,
        )))
    } else {
        Err(StunError::BadFamily)
    }
}

#[cfg(test)]
mod tests {
    use super::{StunError, binding_request, parse_binding_success};
    use std::net::SocketAddr;

    fn success_xor_mapped(tx_id: [u8; 12]) -> Vec<u8> {
        // 203.0.113.5:3478 XORed with the magic cookie.
        let mut buf = vec![0x01, 0x01, 0x00, 0x0C];
        buf.extend_from_slice(&[0x21, 0x12, 0xA4, 0x42]);
        buf.extend_from_slice(&tx_id);
        buf.extend_from_slice(&[
            0x00, 0x20, 0x00, 0x08, 0x00, 0x01, 0x2C, 0x84, 0xEA, 0x12, 0xD5, 0x47,
        ]);
        buf
    }

    #[test]
    fn binding_request_is_twenty_zero_attribute_bytes() {
        let req = binding_request([0xAB; 12]);
        assert_eq!(&req[..8], &[0x00, 0x01, 0x00, 0x00, 0x21, 0x12, 0xA4, 0x42]);
        assert_eq!(&req[8..], &[0xAB; 12]);
    }

    #[test]
    fn xor_mapped_address_decodes() {
        let tx_id = [0u8; 12];
        let addr = parse_binding_success(&success_xor_mapped(tx_id), tx_id).unwrap();
        assert_eq!(addr, "203.0.113.5:3478".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn mismatched_transaction_id_is_rejected() {
        let buf = success_xor_mapped([0u8; 12]);
        let err = parse_binding_success(&buf, [1u8; 12]).unwrap_err();
        assert_eq!(err, StunError::TransactionMismatch);
    }

    #[test]
    fn mapped_address_is_used_when_xor_is_absent() {
        let tx_id = [0u8; 12];
        let mut buf = vec![0x01, 0x01, 0x00, 0x0C];
        buf.extend_from_slice(&[0x21, 0x12, 0xA4, 0x42]);
        buf.extend_from_slice(&tx_id);
        // MAPPED-ADDRESS, not XORed: 203.0.113.5:3478.
        buf.extend_from_slice(&[
            0x00, 0x01, 0x00, 0x08, 0x00, 0x01, 0x0D, 0x96, 203, 0, 113, 5,
        ]);
        let addr = parse_binding_success(&buf, tx_id).unwrap();
        assert_eq!(addr, "203.0.113.5:3478".parse::<SocketAddr>().unwrap());
    }
}
