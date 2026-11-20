# Progress log

Handoff between agent sessions. **Newest entry first.**
Agents read **Next task**, **Blocked**, and the newest entry.
Do not read the whole log unless a card says to.

## Next task

**M1-06 · Fifteen-second keepalive** — After `PUNCH` ACK, each side sends one keepalive `PUNCH` at 15 seconds and does not answer that frame.

## Blocked

*(none)*

<!-- Format:
- YYYY-MM-DD · card ID · question for the owner · what you tried
-->

---

## Entries

### 2026-11-20 · M1-05 mutual punch on the cone path
- Done: The binary generates `node.key` (ed25519 secret || x25519 secret) and `root.key` (root ed25519 secret), mode `0600`, and reuses them. The user supplies neither. `ed25519_public` and `x25519_public` are in `easynet-core`, which the owner allowed for this card. `invite issue` prints one invite. `punch` uses one UDP socket, probes every 250 ms, answers a SYN from the packet source, and prints `punch-ack`.
- Verified: `just lab-test` printed `nat-lab-topology-ok`, `reflexive=172.30.0.11:51820`, `m1-stun-ok`, `punch-ack` on both nodes, and `m1-punch-ok`. Local two-socket test passed. `just check` passed.
- Notes: M1-05 status is `done`. The roadmap checkbox stays open because `docs/roadmap.md` is outside this card. `root.key` is still raw; passphrase encryption stays with later `easynet init`. The lab routers masquerade, so the test opens UDP `51820` on router-a and router-b for the reflexive port. Next task is M1-06.

### 2026-11-20 · M1-04 PUNCH codec and replay cache
- Done: `encode_punch` / `verify_punch` sign the raw bytes before the last 64. `sender_node_id` is the first 16 bytes of `SHA256(ed25519_pub)`. `PunchCache` keeps 4096 `(sender, nonce)` pairs for 30_000 ms and drops the least recently used pair when full. The cache takes `now_ms` from the caller.
- Verified: a frame 29_999 ms off is accepted; 30_001 ms is `Skew`; the same pair at 30_000 ms is `Replay`; the 4097th pair evicts the oldest; a short frame and a bad signature fail. `just check` passed.
- Notes: M1-04 status is `done`. The roadmap checkbox stays open because `docs/roadmap.md` is outside this card's allowlist. Next task is M1-05.

### 2026-11-20 · M1-03 invite encode and decode
- Done: `encode_invite` and `decode_invite` sign `InviteToken` as `easynet://invite/` plus unpadded base64url of `postcard(body) || sig[64]`. The signature covers `easynet.invite.v1\0` || postcard body. `mesh_id` is the first 8 bytes of `SHA256(root_pub_key)`. Addresses use the packed api §2 layout. No base64 crate and no socket.
- Verified: `cargo test -p easynet-core` round-trips one IPv4 candidate, keeps `issued_at`, rejects five candidates, a flipped signature byte, and a non-invite URI. `just check` passed.
- Notes: M1-03 status is `done`. The roadmap checkbox stays open because `docs/roadmap.md` is outside this card's allowlist. Next task is M1-04.

### 2026-11-20 · M1-02 one socket STUN in the lab
- Done: `easynet stun --bind IP:PORT --server HOST:PORT` binds one UDP socket, sends one Binding Request, and prints `reflexive=IP:PORT`. The lab script appends after `nat-lab-topology-ok`. Every host builds that binary in `rust:1.85.0-alpine` for the Docker engine's architecture, then the script copies it into `node-a`.
- Verified: `just lab-test` printed `nat-lab-topology-ok`, then `reflexive=172.30.0.11:51820`, then `m1-stun-ok`. `just lab-down` and `just check` passed.
- Notes: M1-02 status is `done`. The roadmap checkbox stays open because `docs/roadmap.md` is outside this card's allowlist. Next task is M1-03.

### 2026-11-17 · M1-01 demux and STUN codec
- Done: `easynet-core` classifies datagrams in api §1 order and builds a 20-byte Binding Request. `parse_binding_success` prefers XOR-MAPPED-ADDRESS `0x0020` and falls back to MAPPED-ADDRESS `0x0001`. No new crate and no socket.
- Verified: `cargo nextest run -p easynet-core` and `just check`. A Binding Success whose first byte is `0x01` classifies as STUN. A mismatched transaction id is rejected.
- Notes: M1-01 status is `done`. The roadmap checkbox stays open because `docs/roadmap.md` is outside this card's allowlist. Next task is M1-02.

### 2026-11-17 · M1-00 accepted
- Done: Owner accepted `docs/tasks/M1-puncher.md`. M1-00 is `done`. Roadmap M1-00 is ticked.
- Verified: review only; no puncher code in this step.
- Notes: Next task is M1-01.

### 2026-11-16 · M1-00 puncher cards drafted
- Done: Wrote `docs/tasks/M1-puncher.md` (M1-01..M1-06) and ticked nothing. Order is demux and STUN codec, one-socket STUN in the lab, invite encode/decode, `PUNCH` codec and the 30 s / 4096 cache, mutual punch on node-a and node-b, then the 15 s keepalive. Lab lines append after `nat-lab-topology-ok`. No new UDP type, no STUN crate, no second steady-state socket.
- Verified: `just scope-check` prints `card-scope-ok (M1-00)`.
- Notes: **Next task stays M1-00** until the owner accepts the card file. Do not start M1-01 before that review.

### 2026-11-16 · Card status lines match the roadmap
- Done: Owner lock. M0-02, M0-03, and M0-04 are `done` in `docs/tasks/M0-bootstrap.md`. ADR-0018 amends ADR-0017 so the scope gate allows a `**Status:**` line for the active card and for a card already ticked on the roadmap. `AGENTS.md`, `docs/tasks/README.md`, `docs/spec.md`, and `docs/dev.md` say the same thing. M1-00 stays `todo`.
- Verified: `just scope-check` prints `card-scope-ok (M0-04)`. A non-status edit in the task file still fails the gate.
- Notes: Next task remains M1-00. Do not write M1 cards until the owner asks to plan.

### 2026-11-16 · M0-04 NAT lab, deny, linker, CI, devcontainer
- Done: Copied `docs/dev.md` §§3–4 into `docker/nat-lab.yml`, the router and node Dockerfiles and entry scripts, `docker/run-nat-tests.sh`, `deny.toml`, `.cargo/config.toml` (no linker), `.github/workflows/ci.yml`, and `.devcontainer/devcontainer.json`. `justfile` gained `lab-up`, `lab-test`, `lab-down`, and `deny`. Scope job and `just check` stay.
- Verified: `docker compose -f docker/nat-lab.yml config` exits 0; greps match the coturn image, `random-fully`, and `nat-lab-topology-ok`. `just lab-up`, then `just lab-test` printed `nat-lab-topology-ok` after the cross-LAN ping, conntrack, and symmetric-port asserts, then `just lab-down`. `just check` exits 0 (`card-scope-ok (M0-04)`, fmt, clippy, nextest 2 passed).
- Notes: Roadmap M0-04 is ticked. `Cargo.lock` and `target/` stay out of the tree. Next task is M1-00. Do not write M1 cards until the owner asks to plan.

### 2026-11-14 · M0-03 green `just check`
- Done: `just check` already depended on `scope-check`, then `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo nextest run --workspace`. Left that recipe in place. Did not edit `rust-toolchain.toml`, did not add a Makefile, and did not replace the M0-02 config tests.
- Verified: `test -f rust-toolchain.toml && test -f justfile`; `just check` exits 0. `Cargo.lock` and `target/` stay out of the tree because they are outside this card's allowlist.
- Notes: Roadmap M0-03 is ticked. Next task is M0-04.

### 2026-11-14 · M0-02 scaffold, toolchain pin, config parser
- Done: `rust-toolchain.toml` channel `1.85.0` with rustfmt and clippy. Workspace crates `easynet-core`, `easynet-net`, `easynet-admin`, `easynet-cli`, `easynet-ffi` (edition 2024). Core parses spec §9 with serde and toml 0.8; `default_toml()` is the Unix sample. `easynet config init` writes that sample only when the path is absent. Net, admin, and ffi are stubs. No snow, tun-rs, or axum.
- Verified: `cargo build -p easynet-cli`; `cargo test -p easynet-core` (sample numbers and a truncated document); both binaries' `--help` / `-h` print a line containing `easynet`; `config init --path` writes `relay_max_bps = 1250000` and does not overwrite an existing file; user path uses `$HOME/.config/easynet/easynet.toml`; clippy `-D warnings` and `cargo fmt --check` are clean. `Cargo.lock` and `target/` are not in the card allowlist, so they are not kept in the tree.
- Notes: Roadmap M0-02 is ticked. Next task is M0-03.

### 2026-11-14 · Card scope gate (Q24)
- Done: `scripts/check-card-scope.sh` and `justfile` (`scope-check` then fmt/clippy/nextest). ADR-0017. The card id is the committed **Next task**; paths come from that card's `**Files:**` line. CI range check is in the `dev.md` workflow that M0-04 copies. Next task remains M0-02.
- Verified: script parses M0-02 from committed progress when HEAD has it.
- Notes: Commit this steering tree before M0-02. Until that commit, `just scope-check` fails because these docs are outside M0-02's allowlist. Do not add them to the card to silence it.

### 2026-11-14 · Protocol locks (Q16–Q23)
- Done: Owner locks recorded. STUN-before-mesh demux and relay-of-handshake (ADR-0012). Noise prologue and snow nonces (ADR-0013). SWIM timers and 2-byte gossip header (ADR-0014). Reverse proxy, IPC, bearer, admin JSON, HMAC webhook (ADR-0015). Toolchain file `1.85.0` and system linker (ADR-0016). Bootstrap exception appended to ADR-0002. `easynet.toml` schema in spec §9. M0-02 now creates the toolchain file and the config parser. M0-04 copies `docs/dev.md` §§3–4 and must prove topology. M2-00, M3-00, M4-00 planning cards list the locks.
- Verified: docs only; no product code. Next task is still M0-02.
- Notes: `GET /status`, `GET /me/cert`, and `GET /topology` JSON is the one shape M4-00 must write from design §5 nouns. Invite and join domain tags are `easynet.invite.v1` and `easynet.join.v1`.

### 2026-11-14 · Q15 deferred (out of M4)
- Done: Q15 → deferred post-M4 / M5 nice-to-have; M4 Done-when uses `127.0.0.1:8080` + `easynet status` only; sketch in `api.md` §10; roadmap M5 split into DNS + mobile tracks; open-questions “Still open” empty.
- Verified: M4-00 unblocked from DNS; no product code.
- Notes: Next task still M0-02.

### 2026-11-14 · Docs tidy (plan under docs/, Q15 surfaced)
- Done: moved `plan.md` → `docs/plan.md`; added `docs/README.md`; spec §8 lists **Q15** under “Still open” (admin.easynet DNS).
- Verified: links updated in AGENTS/spec/ADR-0001.
- Notes: Next task still M0-02.

### 2026-11-14 · Dev toolchain + NAT lab (ADR-0011)
- Done: ADR-0011 Accepted; `docs/dev.md`; AGENTS/spec/roadmap use `just check` + nextest; M0-03/M0-04 cards rewritten (no Makefile).
- Verified: docs cross-links; no product code.
- Notes: Next task still M0-02.

### 2026-11-14 · Q11–Q14 + vip_salt + two-knob relay
- Done: ADR-0008 (supersedes VIP hash in 0004), ADR-0009 (supersedes 0007), ADR-0010 (`0x05` handshake); api/spec/design/conventions updated; Q11–Q14 resolved; Q15 remains optional DNS.
- Verified: EndpointRecord field order matches owner list; DATA header uses u64 counter; effective relay = Member ∧ relay_capable.
- Notes: Next task still M0-02.

### 2026-11-14 · Portal access rules (no master IP)
- Done: Documented loopback-on-every-node vs AdminCert VIP-only vs member VIP forbidden; resolved Q14; added optional Q15 (`admin.easynet` DNS). Updated spec invariant 8, design §How to open, api §8, ADR-0004, conventions/admin, AGENTS.
- Verified: docs cross-read; no product code.
- Notes: Next task still M0-02.

### 2026-11-14 · M0-01 + Q1–Q10 lock
- Done: Accepted ADR-0002..0007; rewrote VIP (ADR-0004), Rust toolchain (0005), offline root/AdminCert (0006), relay policy (0007); froze postcard domain tags + `easynet://` tokens in `docs/api.md`; updated spec/design/glossary/conventions/AGENTS; marked M0-01 done.
- Verified: `test -f docs/adr/0005-implementation-language.md`; spec no longer says language “not chosen”; `make check` named in `AGENTS.md`.
- Notes: Remaining open questions Q11–Q14 only (Noise mux framing, RENEW_REQ bytes, revoke reason enum, member portal read access). Next: M0-02 scaffold.

### 2026-11-14 · Steering installed (Full)
- Done: initial Full steering from `plan.md`.
- Verified: audit checklist for Full size (documentation pass).
- Notes: superseded blockers Q1/Q2/Q8 by owner answers above.
