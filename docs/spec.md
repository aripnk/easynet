# EasyNet — Product & technical specification

**Status:** Living document. Source of truth for *what* the system does.
Decisions with lasting consequences are recorded as ADRs in
[`docs/adr/`](adr/README.md). The order of work is in
[`docs/roadmap.md`](roadmap.md). Wire shapes live in [`docs/api.md`](api.md).
Admin UI lives in [`docs/design.md`](design.md).

Historical notes from the first design dump: [`plan.md`](plan.md).
If `plan.md` disagrees with this spec, **this file wins**. Doc index: [`README.md`](README.md).

## 1. Product goals

1. Give desktop, server, and (later) mobile devices a shared Layer-3 overlay so ordinary apps (`ping`, `ssh`, HTTP, SMB) work across NATs without a paid control plane or dedicated coordination server.
2. Admit and revoke devices using cryptographic admin certificates, not a central account database or device cap.
3. Keep the mesh usable when many peers are asleep or offline: a single awake admin can approve a join.
4. Prefer direct peer paths; fall back to peer-assisted relay only when dual-symmetric NAT blocks punching.

### Non-goals

- Replacing WireGuard/Tailscale feature-for-feature (exit nodes marketplace, ACL policy language v1, SSO).
- Running a hosted signaling, DERP, or coordination service operated by the EasyNet project.
- Guaranteeing connectivity when **no** public STUN (or equivalent reflexive discovery) and **no** reachable peer/anchor exist.
- Raft/quorum consensus for membership.
- Shipping Android/iOS clients before the desktop/server daemon proves M0–M3. Mobile is **M5** (`easynet-ffi` / UniFFI).

## 2. Invariants

Numbered rules. Breaking one is a bug even if a task card says otherwise.

1. **No central control plane.** Pairing, join approval, and membership never require a project-operated server. Discovery may use public STUN and local UPnP/NAT-PMP. [ADR-0002](adr/0002-zero-server-admin-trust.md)
2. **Membership is certificate-based.** A peer may send or accept `DATA` only if it holds a valid, non-revoked `PeerCert` for the mesh (bootstrap pair is the documented exception until both hold certs). [ADR-0002](adr/0002-zero-server-admin-trust.md)
3. **Revocation wins.** A valid `RevokeCert` for a peer key permanently overrides any `PeerCert` for that key; revoked peers' packets are dropped. [ADR-0002](adr/0002-zero-server-admin-trust.md)
4. **One primary UDP socket per daemon.** All mesh subprotocols share one bound local UDP port (default `:51820`) so Cone-NAT mappings stay consistent. Temporary birthday-probe sockets are the only exception. [ADR-0003](adr/0003-single-socket-noise.md)
5. **DATA is end-to-end encrypted.** A relay may forward an unmodified `0x03 DATA` or `0x05 HANDSHAKE` and must not decrypt either. [ADR-0003](adr/0003-single-socket-noise.md), [ADR-0012](adr/0012-stun-demux-and-relay-handshake.md)
6. **Virtual IP is deterministic from identity.** VIP is derived per [ADR-0008](adr/0008-vip-salt-unified-hash.md): always `SHA256("easynet-vip-v1" || pub || [vip_salt])` into `/10`. Peers do not pick arbitrary VIPs. Never hard-code `100.64.0.1` as admin.
7. **Endpoint merge is incarnation-monotonic.** When two signed endpoint records conflict, the higher `incarnation` wins; never wall-clock alone.
8. **Admin HTTP bind rules (no master IP).** There is no Raft/leader VIP. (a) Every `easynetd` binds `127.0.0.1:8080`. A node that is not the chosen admin **reverse-proxies** to the lowest-`node_id` `ALIVE` admin VIP `:8080` (never a 302). (b) Only nodes with a valid `AdminCert` bind `:8080` on their TUN VIP. (c) Non-admin VIPs must not serve the portal on the mesh interface. (d) Never bind `0.0.0.0` or a LAN/WAN NIC. Mutating admin actions require `AdminCert`-backed auth, not source VIP alone. Approving on any admin portal has the same effect (state syncs via gossip). [ADR-0015](adr/0015-admin-reverse-proxy-and-auth.md)
9. **Webhook dedupe.** Only the currently `ALIVE` admin with the lexicographically lowest `node_id` may fire an external join webhook for a given pending request.
10. **Production refuses unsigned membership mutations.** Gossip that claims to add/remove members without a valid admin signature is ignored (and should be test-covered). Canonical signatures use domain tag + postcard only. [ADR-0006](adr/0006-offline-root-and-admin-certs.md)

## 3. Stack

| Concern | Choice | Reference |
|---|---|---|
| Language / toolchain | Rust Edition 2024, MSRV 1.85+, tokio | [ADR-0005](adr/0005-implementation-language.md) |
| TUN | `tun-rs` | ADR-0005 |
| Noise | `snow` / `Noise_IK_25519_ChaChaPoly_BLAKE2s`, prologue `easynet.noise-ik.v1`, `StatelessTransportState` | [ADR-0010](adr/0010-noise-handshake-mux.md), [ADR-0013](adr/0013-noise-prologue-and-snow-nonce.md) |
| Identity crypto | `ed25519-dalek`, `x25519-dalek` | ADR-0005 |
| Codec | `postcard` | ADR-0006, [api.md](api.md) |
| Admin HTTP | `axum`: loopback on every node; VIP `:8080` only if `AdminCert` | design.md §How to open |
| Mobile FFI | `uniffi` via `easynet-ffi` (M5) | ADR-0005 |
| Default UDP port | `51820` | [api.md](api.md) |
| VIP subnet | `100.64.0.0/10`, MTU `1380` | [ADR-0008](adr/0008-vip-salt-unified-hash.md) |
| URI scheme | `easynet://` | Q8 locked |
| Task runner | `just` (`justfile`, `set dotenv-load`) | [ADR-0011](adr/0011-dev-toolchain-and-nat-lab.md) |
| Check command | **`just check`** → card scope gate, then fmt + clippy `-D warnings` + `cargo nextest` | [ADR-0017](adr/0017-card-scope-gate.md), [dev.md](dev.md) |
| NAT integration lab | `docker/nat-lab.yml` (cone / port-restricted / symmetric + local STUN) | ADR-0011, [dev.md](dev.md) |
| Dep policy | `cargo-deny` / `deny.toml` | ADR-0011 |
| Config file | `serde` + `toml` 0.8, schema in §9 | this spec |
| Fast link | system linker; `.cargo/config.toml` sets no `-fuse-ld` | [ADR-0016](adr/0016-pinned-toolchain-and-system-linker.md) |
| Devcontainer / CI | `.devcontainer/`, `.github/workflows/ci.yml` | ADR-0011 |

## 4. Architecture

Every peer runs the same user-space daemon (`easynetd`) in four layers:

```
OS apps (ping, ssh, …)
        │ raw IPv4/IPv6 (MTU 1380)
        ▼
Layer 4  TUN / utun / Wintun   — VIP, packet I/O
        ▼
Layer 3  Control plane         — SWIM gossip, AdminCert/PeerCert/Revoke, admin portal
        ▼
Layer 2  NAT + crypto + relay  — Noise IK, punch, birthday probe, RELAY
        ▼
Layer 1  Transport + discovery — single UDP socket, STUN, UPnP/NAT-PMP, tokens
```

Call direction: Layer 4 → 3 → 2 → 1 for egress; reverse for ingress.
The admin portal talks only to Layer 3 state; it never opens a second primary UDP socket.

### Repository layout

```
easynet/
├── AGENTS.md
├── rust-toolchain.toml
├── justfile                   # just check / lab-up / lab-test / deny
├── Cargo.toml                 # workspace root
├── deny.toml
├── .cargo/config.toml
├── .devcontainer/
├── .github/workflows/ci.yml
├── docker/nat-lab.yml         # plus router/node Dockerfiles and entry scripts; see dev.md §3
├── docs/ …                    # includes docs/dev.md
├── uicopy/en.json
└── crates/
    ├── easynet-core/          # identity, wire, swim, traversal (no OS I/O)
    ├── easynet-net/           # Tokio UDP, STUN/UPnP, TUN loop
    ├── easynet-admin/         # Axum portal + webhook
    ├── easynet-cli/           # easynetd + easynet CLI
    └── easynet-ffi/           # UniFFI (M5; stub OK earlier)
```

## 5. Identity, VIP, and trust

### Keypairs and ids

- **X25519** static — Noise.
- **Ed25519** — gossip, certs, endpoints.
- `node_id` = first 16 bytes of `SHA256(ed25519_public_key)`.
- `mesh_id` = first 8 bytes of `SHA256(root_pub_key)`.

### Virtual IP

See [ADR-0008](adr/0008-vip-salt-unified-hash.md): always hash `vip_salt` (including `0`) into the 22-bit `/10` offset; collision loser increments salt and gossips `EndpointRecord.vip_salt`.

Admin portal access (no single “master” IP — SWIM, not Raft):

| Address | Who binds it | Effect |
|---|---|---|
| `http://127.0.0.1:8080` | **Every** node (admin or member) | Reverse proxy to the lowest-`node_id` `ALIVE` admin, or the portal itself when this node is that admin. Not a 302. |
| `http://<admin-VIP>:8080` | Nodes holding a valid **`AdminCert`** only | Full admin portal; A and B are equivalent (pending joins / certs gossip) |
| `http://<member-VIP>:8080` | **Must not bind** | Non-admin mesh IPs do not expose the portal |

`easynet status` prints all `ALIVE` admin URLs. The loopback proxy chooses the lowest-`node_id` `ALIVE` admin (same tie-break as webhook dedupe). Zero-memorization URL for M4: `http://127.0.0.1:8080` on every node. Mesh DNS (`admin.easynet`) is **deferred** — [Q15](#8-open-questions).

`BootstrapMode` (invite pair, before either peer has stored a cert) is the only `DATA` exception. It is specified in [ADR-0002](adr/0002-zero-server-admin-trust.md) and ends when the first cert hits disk.

### Roles and certs

- **Root** offline preferred; encrypted `root.key` (Argon2id + XChaCha20-Poly1305). [ADR-0006](adr/0006-offline-root-and-admin-certs.md)
- **AdminCert** delegated from root; permissions `APPROVE_PEER` / `REVOKE_PEER`.
- **PeerCert** issued by admin; TTL **90 days**; auto-renew below 30 days or &lt;⅓ TTL via `RENEW_REQ`.
- **AdminCert** TTL **365 days**.
- Canonical field layouts: [api.md](api.md). `PeerCert` has **no** VIP field.

### Node key paths

- Unix: `/var/lib/easynet/node.key` (`0600`), optional OS keychain via `keyring`.
- Windows: `%ProgramData%\EasyNet\node.key`.

## 6. Control plane behavior

- One SWIM round every **1000 ms** (`PROTOCOL_PERIOD`): direct `PING`, **300 ms** `PING_TIMEOUT`, then `PING_REQ` to **K = 2** alive peers. `Suspect` becomes `Dead` after **5000 ms** unless refuted with `incarnation + 1`. At most 8 piggybacked deltas. Field layouts: [api.md §4](api.md), [ADR-0014](adr/0014-swim-failure-detector.md).
- Merge: highest signed `incarnation` for endpoints; `RevokeCert` > `PeerCert`.
- Keepalives: 15s on established paths; STUN keepalive 15s during pairing hold.
- Punch: **250 ms** probes; `|unix_ms - now| > 30 s` or a cached `(sender, nonce)` is dropped. Dual-symmetric falls back to `RELAY` after **2.0 s** without ACK.
- Birthday probe: up to **128** temporary sockets (symmetric side); cone side sprays up to **1024** ports from the **primary** socket.
- Relay: [ADR-0009](adr/0009-relay-two-knob-policy.md) — `role == Member` ∧ `relay_capable`. Inner frame is `0x03` or `0x05` only ([ADR-0012](adr/0012-stun-demux-and-relay-handshake.md)). After a direct `PUNCH` ACK, `DATA` moves to that direct path.
- Noise session setup: prologue `easynet.noise-ik.v1`, mux `0x05 HANDSHAKE`, then `0x03 DATA` via snow's stateless nonce ([ADR-0013](adr/0013-noise-prologue-and-snow-nonce.md), [api.md](api.md)).
- Cert renewal: gossip subtypes `RENEW_REQ` / `RENEW_RESP` ([api.md](api.md) §4).

## 7. Data plane

- Interface name: **`easynet0`** (Linux/Windows) or `utunX` (macOS).
- MTU **1380**.
- Optional LAN subnet routing / NAT masquerade is later; not required for M1–M3 Done-when.

## 8. Open questions

### Still open

One shape is reserved for `M4-00` and is not an execution-card decision:

- JSON bodies for `GET /api/v1/status`, `GET /api/v1/me/cert`, and `GET /api/v1/topology`. Keys come only from design §5 nouns (mesh id, node id, VIP, role, alive count, peer count, `admin_urls`, own cert expiry, direct vs relay edges). `M4-00` writes them into api §8 and stops for owner review before any handler card.

Everything else an agent needs for M0–M3 is locked. Do not invent a feature that is absent from the spec, the roadmap, or a card.

### Resolved (do not re-open without a new ADR)

| ID | Resolution |
|---|---|
| **Q1** | Rust (Edition 2024, MSRV 1.85+, tokio workspace) — [ADR-0005](adr/0005-implementation-language.md), [ADR-0011](adr/0011-dev-toolchain-and-nat-lab.md) |
| **Q2 / Q10** | Full `/10` VIP + `vip_salt` always in hash — [ADR-0008](adr/0008-vip-salt-unified-hash.md) |
| **Q3–Q5** | Offline root, AdminCert, postcard domain tags, TTLs — [ADR-0006](adr/0006-offline-root-and-admin-certs.md) |
| **Q6** | `easynet://invite\|join\|roster` token layouts — [api.md](api.md) §7 |
| **Q7 / Q14** | Portal bind rules + member read-only / admin mutate — [design.md](design.md), [api.md](api.md) §8 |
| **Q8** | URI scheme `easynet://` |
| **Q9** | Two-knob relay — [ADR-0009](adr/0009-relay-two-knob-policy.md) |
| **Q11** | `0x05 HANDSHAKE` — [ADR-0010](adr/0010-noise-handshake-mux.md), [api.md](api.md) §5b |
| **Q12** | Gossip `RENEW_REQ` / `RENEW_RESP` — [api.md](api.md) §4 |
| **Q13** | `RevokeReason` 0–4, unknown 5–255 — [api.md](api.md) |
| **Q15** | **Deferred** — out of scope for M4. M4 uses `127.0.0.1:8080` reverse proxy + `easynet status` admin URLs (avoids OS split-DNS). Optional post-M4 mesh DNS sketch in [api.md §10](api.md#10-deferred-q15-mesh-dns); schedule under roadmap **M5 nice-to-have**, not M4-00. |
| **Q16** | STUN magic cookie is tested before mesh type bytes — [ADR-0012](adr/0012-stun-demux-and-relay-handshake.md), [api.md §1](api.md) |
| **Q17** | `RELAY` may wrap `0x03` or `0x05` only, with the 128-byte / 2 pps handshake caps — ADR-0012, [api.md §6](api.md) |
| **Q18** | Bootstrap `DATA` exception — [ADR-0002](adr/0002-zero-server-admin-trust.md) |
| **Q19** | `easynet.toml` schema and paths — §9 below. Parser is `serde` + `toml` 0.8 |
| **Q20** | Noise prologue `easynet.noise-ik.v1` and snow nonces — [ADR-0013](adr/0013-noise-prologue-and-snow-nonce.md) |
| **Q21** | SWIM timers, 2-byte gossip header, `GossipDelta` order — [ADR-0014](adr/0014-swim-failure-detector.md) |
| **Q22** | Loopback reverse proxy, IPC paths, bearer, admin JSON, HMAC webhook — [ADR-0015](adr/0015-admin-reverse-proxy-and-auth.md). Three GET bodies (`status`, `me/cert`, `topology`) are written by M4-00 from design §5 nouns before handlers exist |
| **Q23** | `rust-toolchain.toml` is channel `1.85.0` plus rustfmt and clippy; system linker — [ADR-0016](adr/0016-pinned-toolchain-and-system-linker.md) |
| **Q24** | `just check` fails if a dirty path is outside the committed card's `**Files:**` list — [ADR-0017](adr/0017-card-scope-gate.md) |

## 8b. M4 portal access (no DNS)

Operators never need a master IP or system DNS in M4:

1. `http://127.0.0.1:8080` on **every** node → reverse proxy to the lowest-`node_id` `ALIVE` admin VIP `:8080` (or the local portal when this node is that admin).
2. `easynet status` prints all active `http://<admin-VIP>:8080` URLs.

## 9. Environment / config

Parser: `serde` with `derive`, crate `toml` **0.8**. No other TOML crate.
`easynet-core` parses strings only; the CLI writes the file.
M0-02 implements this schema. Do not add keys.

`easynet config init` writes the file below when it is absent, and never
overwrites. `--path <file>` selects the path (tests use this). With no
`--path`, root (euid 0) writes `/etc/easynet/easynet.toml`; any other user
writes `~/.config/easynet/easynet.toml`. On Windows the only path is
`%ProgramData%\EasyNet\easynet.toml` (expand the environment variable).

| | Path |
|---|---|
| Linux / macOS system (euid 0) | `/etc/easynet/easynet.toml` |
| Linux / macOS user | `~/.config/easynet/easynet.toml` |
| Windows | `%ProgramData%\EasyNet\easynet.toml` |

`state_dir` in the Unix sample is `/var/lib/easynet`. On Windows, `config init`
writes `state_dir` as the expanded `%ProgramData%\EasyNet\state`. Keys live
inside `state_dir` (`node.key`, `bootstrap_token.sha256`). `tun_name` is
ignored on macOS (`utun`). `allow_relay = true` in the file is forced to
`false` on iOS and Android. Empty `webhook_url` disables the webhook.
`relay_max_bps` is **bytes per second** (1_250_000 = 10 Mbps). `--no-relay`
on the CLI forces `allow_relay` off for that process.

```toml
[node]
listen_port = 51820
tun_name = "easynet0"          # ignored on macOS (uses utun)
tun_mtu = 1380
state_dir = "/var/lib/easynet" # %ProgramData%\EasyNet\state on Windows

[discovery]
stun_servers = [
  "stun.l.google.com:19302",
  "stun.cloudflare.com:3478",
]
enable_upnp = true
keepalive_secs = 15

[relay]
allow_relay = true             # forced false automatically on iOS/Android
relay_max_bps = 1250000        # bytes/s; 10 Mbps
relay_burst_bytes = 2097152    # 2 MB burst

[admin]
bind_port = 8080
webhook_url = ""               # empty = do not POST
webhook_secret = ""            # HMAC-SHA256 key for the raw webhook body
```
