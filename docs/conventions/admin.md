# Admin portal (easynet-admin)

Read this when editing `crates/easynet-admin`.

- Every node: bind `127.0.0.1:8080`. Reverse-proxy to the lowest-`node_id`
  `ALIVE` admin unless this node is that admin. Never send a 302.
- `AdminCert` nodes only: also bind TUN VIP `:8080`. Member VIP must not.
  Never `0.0.0.0`. Cookie name `easynet_sess`.
- Local IPC: `/run/easynet/admin.sock` or `\\.\pipe\easynet-admin`.
  Mutating routes need an AdminCert session, that IPC, or the one-time
  bootstrap token. Copy from `uicopy/en.json`. Webhook HMAC: lowest ALIVE
  `node_id` only. Link: [api.md §8](../api.md), [ADR-0015](../adr/0015-admin-reverse-proxy-and-auth.md).

```rust
// BAD: always bind((vip, 8080)) even when the node has no AdminCert
```

```rust
// GOOD: bind((Ipv4Addr::LOCALHOST, 8080)) on every node;
//       if has_admin_cert { bind((vip, 8080)) }
```
