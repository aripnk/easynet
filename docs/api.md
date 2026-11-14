# EasyNet wire & local HTTP contracts

**Status:** Living. Shapes and codes only. Behavior narrative lives in
[`spec.md`](spec.md). Locked through the 2026-11-14 protocol lock
(ADR-0008..0016). Do not invent a field, timer, or route that this file
does not list.

Content types:

- UDP mesh: binary frames on one socket (default `:51820`).
- Signed bodies: `DomainTag || postcard(struct)` — never JSON.
- Out-of-band tokens: postcard payload → **base64url (unpadded)** in `easynet://` URIs.
- Admin portal: `application/json` over HTTP. Bind rules: loopback on every node; VIP `:8080` only if `AdminCert` (see §8).

Unknown UDP type bytes → drop silently.
Unknown JSON fields on admin API → reject with `400` + `unknown_field`.

---

## 1. UDP demultiplex

Apply these steps in order on every datagram read from the primary socket.
[ADR-0012](adr/0012-stun-demux-and-relay-handshake.md).

1. If `buf.len() < 8`, drop (`drop_bad_type`).
2. **STUN first.** If `buf.len() >= 20` and `buf[4..8] == [0x21, 0x12, 0xA4, 0x42]` (RFC 5389 / RFC 8489 magic cookie, network byte order), hand the datagram to the STUN client. This includes Binding Requests (`buf[0] == 0x00`) and Binding Success Responses (`0x0101`, so `buf[0] == 0x01`). Do not parse those packets as mesh frames.
3. **Mesh dispatch** only when step 2 did not match, using `buf[0]`:

| `buf[0]` | Frame |
|---|---|
| `0x01` | `PUNCH` (§3) |
| `0x02` | `GOSSIP` (§4) |
| `0x03` | `DATA` (§5) |
| `0x04` | `RELAY` (§6) |
| `0x05` | `HANDSHAKE` (§5b) |
| anything else | drop (`drop_bad_type`) |

---

## 2. Shared binary types

### `SocketAddrBin`

| Bytes | Field |
|---|---|
| 0 | family: `0x04` IPv4 or `0x06` IPv6 |
| 1.. | 4 or 16 byte address |
| last 2 | port, **big-endian** |

### Postcard / signing rule

A `signature: [u8; 64]` is **never** a field of a postcard body. Every signed
object on the wire or inside a token is:

```text
SignedBlob<T> = postcard::to_bytes(&T) || signature[64]
Signature     = Ed25519::sign(signer_priv, DomainTag || postcard::to_bytes(&T))
```

`DomainTag` is ASCII and NUL-terminated. Postcard uses the field order listed
in this file, little-endian fixed-width integers, and no extra padding. Do
not sign JSON. Do not put the signature bytes inside `T`.

Inside a postcard body, `SocketAddrBin` is **not** a native struct. It is a
`Vec<u8>` holding the packed layout from the table above (family, address,
port big-endian). `Vec<SocketAddrBin>` is `Vec<Vec<u8>>`. The packed layout
is also what the raw `PUNCH` tail uses. Do not encode `port` as a postcard
`u16` (that would be little-endian and would not match this table).

---

## 3. Type `0x01` PUNCH

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | `0x01` |
| 1 | 1 | `version` (`1`) |
| 2 | 1 | `flags` bit0 `SYN`, bit1 `ACK` |
| 3 | 1 | reserved `0` |
| 4 | 16 | `sender_node_id` |
| 20 | 8 | `unix_ms` (`u64` LE, wall clock) |
| 28 | 8 | `nonce` (`u64` LE) |
| 36 | N | packed `SocketAddrBin` (§2), observed/advertised self endpoint |
| last 64 | 64 | Ed25519 sig over bytes `[0..sig)` |

**Rules:** On valid `SYN`, reply `ACK` to the packet **source** `IP:Port`.
Drop the frame when `|unix_ms - now| > 30_000` (`drop_punch_skew`) or the
pair `(sender_node_id, nonce)` is already cached (`drop_punch_replay`).
The cache holds those pairs for **30 seconds** and is a bounded LRU of
**4096** entries. A frame shorter than `36 + 7 + 64` bytes is
`drop_bad_type`. Probe interval: 250 ms per candidate while establishing.

---

## 4. Type `0x02` GOSSIP

[ADR-0014](adr/0014-swim-failure-detector.md). Two-byte header, then the
payload. The previous 20-byte header (reserved bytes and an outer
`sender_node_id`) is withdrawn. `sender_id` lives inside the postcard body
for `PING`, `ACK`, and `PING_REQ`.

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | `0x02` |
| 1 | 1 | `subtype` |
| 2 | N | payload (below) |

### Timers

| Name | Value |
|---|---|
| `PROTOCOL_PERIOD` | 1000 ms (one probe round) |
| `PING_TIMEOUT` | 300 ms (direct `PING` → `ACK`) |
| `INDIRECT_K` | 2 random `ALIVE` peers get `PING_REQ` after a direct timeout |
| `SUSPECT_TIMEOUT` | 5000 ms, then `Suspect` → `Dead` if unrefuted |
| `MAX_PIGGYBACK_DELTAS` | 8 `GossipDelta` items on a `PING` or `ACK`, lowest broadcast count first |

A suspected peer refutes by gossiping `Alive` with `incarnation + 1`.

### Gossip subtypes

| Subtype | Name | Direction | Payload |
|---|---|---|---|
| `0x01` | `PING` | peer ↔ peer | postcard `PingBody` |
| `0x02` | `ACK` | peer ↔ peer | postcard `AckBody` |
| `0x03` | `PING_REQ` | peer → peer | postcard `PingReqBody` |
| `0x04` | `RENEW_REQ` | member → admin | `SignedBlob<RenewRequest>` (raw, not wrapped) |
| `0x05` | `RENEW_RESP` | admin → member | postcard `RenewResp` |

```text
PingBody    { seq: u64, sender_id: [u8; 16], deltas: Vec<GossipDelta> }
AckBody     { seq: u64, sender_id: [u8; 16], deltas: Vec<GossipDelta> }
PingReqBody { seq: u64, requester_id: [u8; 16], target_id: [u8; 16],
              target_endpoints: Vec<Vec<u8>> }  # each inner vec is packed SocketAddrBin
```

`RenewResp` is a postcard enum in this variant order, default postcard
discriminant (`u32` index, then fields). Do not rename or reorder variants.

```text
RenewResp::Ok(Vec<u8>)   # SignedBlob<PeerCert>
RenewResp::Err(u8)       # revoked or invalid
```

`GossipDelta` uses the same postcard enum rule, in this variant order.
Do not add `#[serde(tag)]` or `#[serde(rename)]`. `MemberStatus` is a
unit enum in the order `Alive`, `Suspect`, `Dead` (indices 0, 1, 2).

```text
GossipDelta::MemberState { peer_id: [u8; 16], status: MemberStatus, incarnation: u64 }
GossipDelta::Endpoint(Vec<u8>)     # SignedBlob<EndpointRecord>
GossipDelta::PeerCert(Vec<u8>)     # SignedBlob<PeerCert>
GossipDelta::AdminCert(Vec<u8>)    # SignedBlob<AdminCert>
GossipDelta::RevokeCert(Vec<u8>)   # SignedBlob<RevokeCert>
GossipDelta::PendingJoin(Vec<u8>)  # SignedBlob<JoinRequest>; drop after 15 minutes
```

`JoinRequest` is the join-token body in §7.2 (no signature field). Domain
tag `easynet.join.v1\0`, signed by `peer_ed25519_pub`.

### Canonical signed records

| Record | Domain tag | Body fields (exact order) |
|---|---|---|
| `AdminCert` | `easynet.admin-cert.v1\0` | `version:u8`, `mesh_id:[u8;8]`, `admin_pub_key:[u8;32]`, `permissions:u8`, `issued_at:u64`, `expires_at:u64` |
| `PeerCert` | `easynet.peer-cert.v1\0` | `version:u8`, `mesh_id:[u8;8]`, `peer_ed25519_pub:[u8;32]`, `peer_x25519_pub:[u8;32]`, `role:u8`, `issued_at:u64`, `expires_at:u64`, `issuer_pub_key:[u8;32]` |
| `RevokeCert` | `easynet.revoke-cert.v1\0` | `version:u8`, `mesh_id:[u8;8]`, `target_pub_key:[u8;32]`, `revoked_at:u64`, `reason:u8`, `issuer_pub_key:[u8;32]` |
| `EndpointRecord` | `easynet.endpoint.v1\0` | `version:u8`, `mesh_id:[u8;8]`, `peer_ed25519_pub:[u8;32]`, `incarnation:u64`, `vip_salt:u8`, `endpoints:Vec<Vec<u8>>` (packed `SocketAddrBin`), `relay_capable:bool` |
| `RenewRequest` | `easynet.renew-req.v1\0` | `version:u8`, `mesh_id:[u8;8]`, `peer_ed25519_pub:[u8;32]`, `peer_x25519_pub:[u8;32]`, `requested_at:u64` |

The signature is the 64 bytes after `postcard(body)`. It is not a field of
`RenewRequest` or of any other body in the table.

`AdminCert` has no `issuer_pub_key`. Only `mesh_state.root_pub_key` (pinned
at `easynet init` or `invite_token` import) may sign one. Verifiers check
the signature against that root key and no other key.

`AdminCert.permissions`: `0x01 APPROVE_PEER`, `0x02 REVOKE_PEER`.

`PeerCert.role`: `0 = Member`, `1 = RelayDisabled` (admin policy). Runtime
relay also needs `EndpointRecord.relay_capable` — [ADR-0009](adr/0009-relay-two-knob-policy.md).

`RevokeCert.reason` (`#[repr(u8)]`):

| Value | Name |
|---|---|
| `0` | `Unspecified` |
| `1` | `KeyCompromise` |
| `2` | `DeviceLostOrRetired` |
| `3` | `PolicyViolation` |
| `4` | `Superseded` (routine key rotation) |
| `5..=255` | `Unknown(u8)` — accept without failing signature verification |

`mesh_id` = first 8 bytes of `SHA256(root_pub_key)`.

VIP derivation uses `vip_salt` always: [ADR-0008](adr/0008-vip-salt-unified-hash.md).

Defaults: `PeerCert` TTL **90d**; `AdminCert` TTL **365d**.

---

## 5. Type `0x03` DATA

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | `0x03` |
| 1 | 4 | `receiver_index: u32` LE (peer’s `sender_index` from handshake) |
| 5 | 8 | `counter: u64` LE (snow nonce input; see below) |
| 13 | N | ChaCha20-Poly1305 ciphertext of one raw TUN IP packet |

Pattern: `Noise_IK_25519_ChaChaPoly_BLAKE2s` via `snow` after a finished
`0x05` handshake. [ADR-0013](adr/0013-noise-prologue-and-snow-nonce.md).

- Prologue on both sides, before `build_initiator` / `build_responder`:
  `builder.prologue(b"easynet.noise-ik.v1")`.
- After the handshake: `let mut state = handshake.into_stateless_transport_mode()?`.
- Do **not** assemble a 12-byte ChaCha nonce. `snow` formats the Noise nonce
  as `4 zero bytes || u64` le inside `StatelessTransportState`.
- Send: `tx_counter` starts at `0` for each session. Write
  `tx_counter.to_le_bytes()` into bytes `5..13`, then
  `state.write_message(tx_counter, plaintext, out_buf)`, then increment.
- Receive: read `rx_counter` from bytes `5..13`. Reject it unless it falls
  in a **64-packet** sliding anti-replay window (`drop_aead` on replay or
  AEAD failure). Then `state.read_message(rx_counter, ciphertext, out_buf)`.

Drop `DATA` with `drop_no_cert` unless the sender has a valid non-revoked
`PeerCert`, or the receiver is still in `BootstrapMode` and the sender is
the single invited key ([ADR-0002](adr/0002-zero-server-admin-trust.md)).
`BootstrapMode` ends permanently when the first `AdminCert` or `PeerCert`
is written to disk. `easynet reset` is the only way back.

---

## 5b. Type `0x05` HANDSHAKE

[ADR-0010](adr/0010-noise-handshake-mux.md),
[ADR-0013](adr/0013-noise-prologue-and-snow-nonce.md). Pattern
`Noise_IK_25519_ChaChaPoly_BLAKE2s` with prologue `b"easynet.noise-ik.v1"`.
Common header **11 bytes**:

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | `0x05` |
| 1 | 1 | `version = 0x01` |
| 2 | 1 | `msg_type` (`0x01` INIT, `0x02` RESP) |
| 3 | 4 | `sender_index: u32` LE (random session id chosen by sender) |
| 7 | 4 | `receiver_index: u32` LE (`0` in INIT; echoes initiator `sender_index` in RESP) |

### `HANDSHAKE_INIT` (`msg_type = 0x01`, **115 bytes** total)

| Offset | Size | Field |
|---|---|---|
| 11 | 32 | Initiator ephemeral X25519 public (`e`) |
| 43 | 48 | Encrypted initiator static X25519 (`32` + `16` tag) |
| 91 | 24 | Encrypted `timestamp_secs: u64` LE (`8` + `16` tag); reject if `\|now - ts\| > 60s` |

### `HANDSHAKE_RESP` (`msg_type = 0x02`, **59 bytes** total)

| Offset | Size | Field |
|---|---|---|
| 11 | 32 | Responder ephemeral X25519 public (`e`) |
| 43 | 16 | Encrypted empty payload (`0` + `16` tag) |

---

## 6. Type `0x04` RELAY

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | `0x04` |
| 1 | 1 | `hop_ttl` (must be `1`) |
| 2 | 2 | reserved `0` |
| 4 | 16 | `target_node_id` |
| 20 | N | unmodified inner frame: `0x03` **or** `0x05` only |

Rules: [ADR-0009](adr/0009-relay-two-knob-policy.md),
[ADR-0012](adr/0012-stun-demux-and-relay-handshake.md).

- Forward only if `PeerCert.role == Member` **and** `relay_capable`. No decrypt. No multi-hop (`hop_ttl` must be `1`).
- Inner byte 0 must be `0x03` or `0x05`. Drop `0x01`, `0x02`, and nested `0x04` (`drop_relay_inner`).
- Inner `0x03`: per-`(src,dst)` token bucket **1_250_000 bytes/s** (10 Mbps) and **2_097_152** byte burst (`relay_max_bps` / `relay_burst_bytes` in `easynet.toml`). `relay_max_bps` is **bytes per second**. Drop with `drop_relay_rate`.
- Inner `0x05`: hard maximum **128 bytes** (`HANDSHAKE_INIT` is 115, `HANDSHAKE_RESP` is 59). Larger → `drop_relay_handshake_size`. Also at most **2 packets/sec** per `(src_node_id, dst_node_id)` → `drop_relay_handshake_rate`. The byte bucket does not apply to `0x05`.
- When a direct `PUNCH` ACK later succeeds, the two peers send further `0x03 DATA` on that direct socket and stop sending those `DATA` frames through the relay. In-flight relayed `DATA` may still arrive.

---

## 7. Out-of-band tokens (`easynet://`)

The URI path is **base64url (unpadded)** of the bytes below. The postcard
body does **not** contain a signature field. For invite and join the URI
decodes to `SignedBlob<Body>` (`postcard(body) || sig[64]`).

### 7.1 Invite — `easynet://invite/<base64url>` (`token_type = 0x01`)

Body `InviteToken`, domain tag `easynet.invite.v1\0`, signed by
`peer_ed25519_pub`. Reject more than 4 candidates.

| Field | Type |
|---|---|
| `token_type` | `u8 = 0x01` |
| `version` | `u8 = 0x01` |
| `mesh_id` | `[u8; 8]` |
| `root_pub_key` | `[u8; 32]` |
| `peer_ed25519_pub` | `[u8; 32]` |
| `peer_x25519_pub` | `[u8; 32]` |
| `candidates` | `Vec<Vec<u8>>` packed `SocketAddrBin`, max 4 |
| `issued_at` | `u64` |

### 7.2 Join — `easynet://join/<base64url>` (`token_type = 0x02`)

Body `JoinRequest` (also the `PendingJoin` payload), domain tag
`easynet.join.v1\0`, signed by `peer_ed25519_pub`. `hostname` is display
only, UTF-8, at most 32 bytes.

| Field | Type |
|---|---|
| `token_type` | `u8 = 0x02` |
| `version` | `u8 = 0x01` |
| `mesh_id` | `[u8; 8]` |
| `hostname` | `String` |
| `peer_ed25519_pub` | `[u8; 32]` |
| `peer_x25519_pub` | `[u8; 32]` |
| `candidates` | `Vec<Vec<u8>>` packed `SocketAddrBin` |
| `issued_at` | `u64` |

### 7.3 Roster — `easynet://roster/<base64url>` (`token_type = 0x03`)

`RosterToken` is postcard-encoded and **not** wrapped in an outer signature.
Integrity of the certs is the `SignedBlob` bytes inside. Each `Vec<u8>` is
`postcard(body) || sig[64]` so a verifier checks those exact bytes and does
not re-serialize.

| Field | Type |
|---|---|
| `token_type` | `u8 = 0x03` |
| `version` | `u8 = 0x01` |
| `root_pub_key` | `[u8; 32]` |
| `admin_cert` | `Vec<u8>` = `SignedBlob<AdminCert>` |
| `member_cert` | `Vec<u8>` = `SignedBlob<PeerCert>` |
| `bootstrap_peers` | `Vec<BootstrapPeer>`, length ≤ 3 |

```text
BootstrapPeer { peer_cert: Vec<u8>, endpoint: Vec<u8> }
# peer_cert = SignedBlob<PeerCert>
# endpoint  = SignedBlob<EndpointRecord>
```

---

## 8. Admin HTTP API (M4)

[ADR-0015](adr/0015-admin-reverse-proxy-and-auth.md). JSON only. Unknown
fields → `400` + `unknown_field`. `peer_id_hex` is lowercase hex of
`node_id` (32 hex chars). Mesh DNS is out of scope (§10).

**Listeners (no master IP):**

| Bind | Who | Behavior |
|---|---|---|
| `127.0.0.1:8080` | Every `easynetd` | **Reverse proxy** to `http://<lowest node_id ALIVE admin VIP>:8080`. Not a 302. If this node itself holds `AdminCert` and is that admin (or the only admin), serve the portal here. `Host` stays `127.0.0.1:8080`. |
| `<vip>:8080` on TUN | Only a valid `AdminCert` | Full portal + `/api/v1/*` |
| `<member-vip>:8080` | Forbidden | Must not listen |

The proxy does not skip auth. Cookies are `Set-Cookie: easynet_sess=<opaque>; HttpOnly; SameSite=Strict; Path=/; Max-Age=43200`.

**Local IPC** (admin authority, same HTTP paths, no cookie):

| OS | Path | ACL |
|---|---|---|
| Linux, macOS | `/run/easynet/admin.sock` | mode `0600` |
| Windows | `\\.\pipe\easynet-admin` | DACL: Administrators and SYSTEM only |

**First-boot bearer.** `easynet init` draws 32 random bytes, shows them as 64 lowercase hex chars, and writes `state_dir/bootstrap_token.sha256` as two lines: hex SHA-256 of those bytes, then the unix expiry (issued-at + 86400). `POST /api/v1/auth/bootstrap` with `{"bootstrap_token_hex":"<64 hex>"}` succeeds only if the hash matches and `now < expiry`. On success, delete the file **before** the response, set `easynet_sess`, and return `200` `{"role":"admin"}`. A second call is `401`. There is no `/api/auth/bootstrap` path.

**Auth:**

| Session | Proof | Allowed |
|---|---|---|
| Member | Ed25519 challenge over a live `PeerCert` key | GETs below except pending |
| Admin | Ed25519 challenge over an `AdminCert` key, or local IPC, or the one-time bootstrap token | Member GETs plus pending, approve, reject, revoke |
| Member on a mutating route | — | `403` |

Error envelope:

```json
{ "error": { "code": "pending_not_found", "message_key": "error.pending_not_found" } }
```

| Method | Path | Member | Admin | Errors |
|---|---|---|---|---|
| `GET` | `/api/v1/status` | yes | yes | `401` |
| `GET` | `/api/v1/peers` | yes | yes | `401` |
| `GET` | `/api/v1/me/cert` | yes (own) | yes | `401` |
| `GET` | `/api/v1/topology` | yes | yes | `401` |
| `POST` | `/api/v1/auth/bootstrap` | — | first boot | `401` |
| `POST` | `/api/v1/auth/challenge` | yes | yes | `401` |
| `POST` | `/api/v1/auth/verify` | yes | yes | `401` |
| `GET` | `/api/v1/pending` | **no** | yes | `401`, `403` |
| `POST` | `/api/v1/pending/{peer_id_hex}/approve` | **no** | yes | `401`, `403`, `404`, `409` |
| `POST` | `/api/v1/pending/{peer_id_hex}/reject` | **no** | yes | `401`, `403`, `404` |
| `POST` | `/api/v1/peers/{peer_id_hex}/revoke` | **no** | yes | `401`, `403`, `404` |

`GET /status`, `GET /me/cert`, and `GET /topology` response keys are not listed yet. M4-00 writes those three JSON objects into this section from the nouns in design §5 (mesh id, node id, VIP, role, alive count, peer count, `admin_urls`, own cert expiry, direct vs relay edges) before any handler card. Do not add keys in an execution card.

```json
POST /api/v1/auth/challenge
{"pub_key_b64":"<32-byte ed25519 pub, standard base64>"}
→ 200 {"nonce_b64":"<32 random bytes, standard base64>","expires_in_secs":60}

POST /api/v1/auth/verify
{"pub_key_b64":"<32B>","nonce_b64":"<32B>","sig_b64":"<64B>"}
→ 200 Set-Cookie easynet_sess {"role":"admin"|"member","vip":"100.64.x.y"}

GET /api/v1/peers
→ 200 {"peers":[{"peer_id_hex":"<32 hex>","hostname":"<display>","vip":"100.64.x.y","role":"admin"|"member","status":"alive"|"suspect"|"dead","path":"direct"|"relayed","endpoint":"203.0.113.5:51820","cert_expires_at":1826196000}]}

GET /api/v1/pending
→ 200 {"pending":[{"peer_id_hex":"<32 hex>","hostname":"<display>","pub_key_b64":"<32B>","requested_at":1794660000,"token":"easynet://join/..."}]}

POST /api/v1/pending/{peer_id_hex}/approve
{"role":"member"|"relay_disabled","ttl_days":90}
→ 200 {"roster_export":"easynet://roster/...","vip":"100.64.x.y"}

POST /api/v1/pending/{peer_id_hex}/reject
→ 204 empty body

POST /api/v1/peers/{peer_id_hex}/revoke
{"reason":1}
→ 200 {"revoked_at":1794660500}
```

`reason` is the `RevokeReason` `u8` from §4. `role: "relay_disabled"` issues `PeerCert.role = 1`. `role: "member"` issues `0`. The UI button still uses copy key `pending.accept`; the path is `approve`.

**Webhook.** Only the `ALIVE` admin with the lowest `node_id` sends it, and only when `webhook_url` is non-empty. `POST` that URL with header `X-EasyNet-Signature: sha256=<lowercase hex of HMAC-SHA256(webhook_secret, raw_body)>` and body:

```json
{
  "event": "peer.join_requested",
  "mesh_id_hex": "9f8a21abcd012345",
  "peer_id_hex": "a1b2c3d4e5f6789012345678abcdef01",
  "hostname": "pixel-9",
  "approve_url": "http://127.0.0.1:8080/",
  "timestamp": 1794660000
}
```

`mesh_id_hex` is 16 lowercase hex chars. `approve_url` is always `http://127.0.0.1:8080/`.

---

## 9. Log / test drop codes

| Code | When |
|---|---|
| `drop_bad_type` | Unknown UDP type, or shorter than 8 bytes, or truncated `PUNCH` |
| `drop_bad_sig` | Signature fail |
| `drop_revoked` | Sender revoked |
| `drop_no_cert` | `DATA` or `HANDSHAKE` without membership, and not the invited bootstrap key |
| `drop_aead` | Decrypt fail or `DATA` counter outside the 64-packet window |
| `drop_relay_ttl` | RELAY ttl ≠ 1 |
| `drop_relay_rate` | Inner `0x03` token bucket exceeded |
| `drop_relay_inner` | Inner frame is not `0x03` or `0x05` |
| `drop_relay_handshake_size` | Inner `0x05` longer than 128 bytes |
| `drop_relay_handshake_rate` | Inner `0x05` over 2 packets/sec for that pair |
| `drop_not_relay_capable` | `role != Member` or `relay_capable == false` |
| `drop_punch_skew` | `PUNCH` `unix_ms` more than 30s from local wall clock |
| `drop_punch_replay` | `(sender_node_id, nonce)` still in the 4096-entry cache |
| `drop_handshake_skew` | Handshake timestamp outside 60s |
| `drop_handshake_bad` | Noise handshake decrypt/verify fail |

---

## 10. Deferred: Q15 mesh DNS (post-M4 / M5 nice-to-have)

**Out of scope for M4.** Do not implement in `M4-00` or M4 execution cards.
M4 zero-memorization path: loopback `127.0.0.1:8080` + `easynet status` URLs.

If scheduled later (owner opens an M5 DNS card), the locked sketch is:

| Item | Rule |
|---|---|
| Bind | UDP DNS on local TUN VIP only: `<vip>:53` (never LAN/WAN) |
| Answer | `A` queries for `*.easynet` only; TTL **5s** |
| `admin.easynet` | → lowest-`node_id` `ALIVE` admin VIP |
| `<hostname>.easynet` | → that peer’s VIP from SWIM (hostname from join token / local config) |
| Other names | Forward or refuse — never hijack non-`.easynet` |
| Non-goal in that card | Full OS split-DNS installers for every platform (document only if a later card owns it) |
