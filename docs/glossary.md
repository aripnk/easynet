# Glossary

Use these terms in code (identifier) and in user-facing copy (display term).

| Code term | Display term | Meaning |
|---|---|---|
| `mesh` | Network | One overlay identified by `mesh_id`, sharing a trust root and peer table. |
| `mesh_id` | Network ID | First 8 bytes of `SHA256(root_pub_key)`. |
| `node` / `peer` | Device | A running daemon instance with its own keypairs and virtual IP. Prefer `peer` in protocol docs, `node` in ops copy. |
| `node_id` | Device ID | First 16 bytes of `SHA256(ed25519_public_key)`; used in wire headers. |
| `AdminCert` | Admin certificate | Root-signed delegation allowing approve/revoke. |
| `PeerCert` | Membership certificate | Admin-signed record authorizing a peer's keys and role. VIP is derived, not stored in the cert. |
| `RevokeCert` | Revocation | Admin-signed tombstone that permanently removes a peer's membership. |
| `admin` | Admin | Holder of a valid `AdminCert` (or offline root when issuing AdminCerts). |
| `member` | Member | Holds `PeerCert`; may use the data plane; may not issue certificates. |
| `root_key` | Network root key | Ed25519 genesis key; preferably offline, passphrase-encrypted. |
| `virtual_ip` / `VIP` | Mesh IP | Address in `100.64.0.0/10` on `easynet0` / `utunX`; derived with `vip_salt` always in the hash (ADR-0008). |
| `vip_salt` | VIP salt | `u8` in `EndpointRecord`; `0` primary, `1..=255` collision fallback. |
| `RelayDisabled` | Relay disabled | `PeerCert.role = 1`; admin policy forbidding relay even if node sets `relay_capable`. |
| `incarnation` | Endpoint generation | Monotonic counter; higher signed value wins when merging endpoints. |
| `candidate` | Endpoint candidate | Discovered `IP:Port` used for punching. |
| `relay_capable` | Can relay | Signed `EndpointRecord` flag; mobiles default false. |
| `PUNCH` | Hole punch | `0x01` frames. |
| `GOSSIP` | Gossip | `0x02` SWIM frames. |
| `DATA` | Encrypted data | `0x03` Noise transport frames. |
| `RELAY` | Relay | `0x04` blind forward of `DATA`. |
| `BootstrapMode` | First-pair mode | The two `easynet init` / `easynet pair` peers, before either has stored a cert. Accepts handshake and `DATA` only from the invited key. Ends permanently when the first cert is written. |
| `SignedBlob` | Signed bytes | `postcard(body) || signature[64]`. The signature is never a postcard field. |
| `invite_token` | Invite | `easynet://invite/<base64url>` bootstrap pair token. |
| `join_request` | Join request | `easynet://join/<base64url>`. |
| `roster_export` | Mesh roster | `easynet://roster/<base64url>` after accept. |
| `anchor` | Anchor peer | Stable discovery via UPnP/NAT-PMP and/or Dynamic DNS. |
| `easynetd` | EasyNet daemon | Long-running process (`crates/easynet-cli`). |
| `easynet` | EasyNet CLI | User commands: init, join, approve, qr, status. |
| `admin_portal` | Admin portal | Full UI on admin VIP `:8080`; every node also serves `127.0.0.1:8080` as a proxy/redirect helper. No master IP. |
