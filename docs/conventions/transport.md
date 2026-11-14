# Transport / wire (easynet-core + easynet-net)

Read this when editing `crates/easynet-core` (`wire.rs`, `traversal.rs`) or
`crates/easynet-net` UDP mux.

- Own the single primary UDP socket in `easynet-net`; demux mesh vs STUN; frame
  codecs live in `easynet-core` and stay I/O-free. Types `0x01`–`0x05`. Do not
  open steady-state sockets for GOSSIP/DATA/HANDSHAKE. Link:
  [ADR-0003](../adr/0003-single-socket-noise.md),
  [ADR-0010](../adr/0010-noise-handshake-mux.md), [api.md](../api.md).

```rust
// BAD: second UdpSocket::bind for gossip while DATA uses another port
```

```rust
// GOOD: one UdpSocket; if bytes 4..8 are 21 12 A4 42 { stun } else { match byte0 };
//       temp sockets only for birthday punch. RELAY inner is 0x03 or 0x05 only.
```