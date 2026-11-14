# TUN / data plane (easynet-net)

Read this when editing TUN I/O in `crates/easynet-net`.

- VIP from ADR-0008 (always hash `vip_salt` byte, including `0`); interface
  `easynet0` / `utunX`; MTU 1380; use `tun-rs`. Never hard-code `100.64.0.1`.
  Link: [ADR-0008](../adr/0008-vip-salt-unified-hash.md).

```rust
// BAD: if salt == 0 { sha256(prefix || pub) } else { sha256(prefix || pub || [salt]) }
```

```rust
// GOOD: let ip = derive_vip(ed25519_pub, vip_salt); // always || [vip_salt]
```
