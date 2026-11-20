# M1 · Single-socket puncher

Execution cards for demux, STUN, invite tokens, and `PUNCH` on the one
primary UDP socket. Do not open a second steady-state socket. Do not add
`snow`, `tun-rs`, `axum`, or a STUN crate. Host TUN, birthday probes,
`RELAY`, gossip, and Noise are out of M1 (M2 and M3).

Locked, and not to be reopened by any card below:

- Demux order and the STUN magic cookie: [api.md §1](../api.md#1-udp-demultiplex). A Binding Success whose first byte is `0x01` is STUN.
- `PUNCH` layout, `nonce` as `u64` LE, skew **30 s**, replay LRU **4096** / **30 s**: [api.md §3](../api.md#3-type-0x01-punch).
- Invite body and domain tag `easynet.invite.v1\0`: [api.md §7.1](../api.md#71-invite--easynetinvitebase64url-token_type--0x01).
- Probe interval **250 ms** while establishing. Keepalive **15 s** on an established path. STUN keepalive **15 s** during pairing hold. [spec.md §6](../spec.md#6-control-plane-behavior).
- Lab topology asserts in `docker/run-nat-tests.sh` stay. Later cards append after `nat-lab-topology-ok`.

`SocketAddrBin` is the packed layout in api §2 (family, address, port **big-endian**). Inside postcard it is `Vec<u8>`, not a native `u16`.

---

### M1-01 · Demux and STUN codec
- **Status:** done
- **Goal:** `easynet-core` classifies datagrams in api §1 order and builds and parses a STUN Binding Request and Binding Success with no socket I/O.
- **Read first:** `docs/api.md` §1; `docs/adr/0012-stun-demux-and-relay-handshake.md`; `docs/conventions/transport.md`.
- **Files:** + `crates/easynet-core/src/demux.rs`; + `crates/easynet-core/src/stun.rs`; ~ `crates/easynet-core/src/lib.rs`
- **Steps:**
  1. No new crate. `classify(buf) -> Class` with variants `TooShort`, `Stun`, `Punch`, `Gossip`, `Data`, `Relay`, `Handshake`, `Unknown`. Rules are api §1 exactly: length under 8 is `TooShort`; length at least 20 and bytes `4..8` equal `21 12 A4 42` is `Stun`; otherwise `buf[0]` selects the mesh type; any other first byte is `Unknown`.
  2. `binding_request(tx_id: [u8; 12]) -> [u8; 20]` is an RFC 5389 Binding Request: type `0x0001`, length `0`, magic cookie, then `tx_id`. No attributes, no `MESSAGE-INTEGRITY`, no `FINGERPRINT`.
  3. `parse_binding_success(buf, tx_id) -> Result<std::net::SocketAddr, StunError>` requires type `0x0101`, the magic cookie, and the same `tx_id`. Read attributes as type `u16` BE, length `u16` BE, then value, padded to a multiple of 4. Prefer attribute `0x0020` `XOR-MAPPED-ADDRESS`. If it is absent, use `0x0001` `MAPPED-ADDRESS`. If neither is present, return an error. Ignore every other attribute. IPv4 family byte is `0x01` and IPv6 is `0x02` (RFC 5389, not the mesh `0x04` / `0x06` family). XOR the port with `0x2112` and the IPv4 address with the magic cookie. XOR an IPv6 address with magic cookie || `tx_id`.
  4. Unit tests, no sockets:
     - Bytes `01 01 00 00 21 12 a4 42` followed by 12 zero bytes classify as `Stun`, not `Punch`.
     - The same 20 bytes with cookie `00 00 00 00` classify as `Punch`.
     - A 7-byte buffer is `TooShort`. First byte `0x09` with length 8 is `Unknown`.
     - `parse_binding_success` of a success whose `XOR-MAPPED-ADDRESS` decodes to `203.0.113.5:3478` returns that address. A mismatched `tx_id` returns an error.
- **Acceptance:**
  - `cargo nextest run -p easynet-core` → exit 0
  - `just check` → exit 0
- **Out of scope:** Binding a UDP socket; the NAT lab; invite tokens; `PUNCH`.

---

### M1-02 · One socket STUN in the lab
- **Status:** done
- **Goal:** `easynet stun` binds one UDP socket, sends one Binding Request to the lab STUN server, and prints the reflexive address from inside `node-a`.
- **Read first:** `docs/api.md` §1; `docs/adr/0003-single-socket-noise.md`; `docs/dev.md` §3; `docs/tasks/M1-puncher.md` (M1-01).
- **Files:** + `crates/easynet-net/src/stun_io.rs`; ~ `crates/easynet-net/src/lib.rs`; ~ `crates/easynet-net/Cargo.toml`; ~ `crates/easynet-cli/Cargo.toml`; ~ `crates/easynet-cli/src/bin/easynet.rs`; ~ `docker/run-nat-tests.sh`
- **Steps:**
  1. `easynet-net` depends on `easynet-core` and `tokio` 1 with features `net`, `time`, `rt-multi-thread`, and `macros`. `binding_reflexive(bind, server) -> Result<SocketAddr>` binds **one** `UdpSocket`, sends `binding_request`, and waits at most 2 seconds for a datagram `classify` calls `Stun`. Parse it with `parse_binding_success`. Drop anything else. Do not bind a second socket.
  2. `easynet stun --bind IP:PORT --server HOST:PORT` prints one line `reflexive=IP:PORT` and exits 0. Exit 1 on timeout.
  3. Append to `docker/run-nat-tests.sh` **after** the existing `echo nat-lab-topology-ok`. Do not change the topology asserts or that line. The append builds a static Linux binary and runs it in `node-a`:
     - Read the container arch with `docker info -f '{{.Architecture}}'` (`aarch64` or `x86_64`).
     - `rustup target add <arch>-unknown-linux-musl` if that target is missing.
     - `cargo build --release --target <arch>-unknown-linux-musl -p easynet-cli --bin easynet`.
     - `docker cp` that binary to `node-a` as `/tmp/easynet`.
     - `docker compose exec` `/tmp/easynet stun --bind 0.0.0.0:51820 --server 172.30.0.10:3478`.
     - Require the printed address to be in `172.30.0.0/24`. Then `echo m1-stun-ok`.
  4. Do not set `linker` or `-fuse-ld`. Do not edit `rust-toolchain.toml` or the compose networks.
- **Acceptance:**
  - `just lab-up` → exit 0; `just lab-test` stdout contains `nat-lab-topology-ok` and then `m1-stun-ok`; `just lab-down`
  - `just check` → exit 0
- **Out of scope:** Invite tokens; `PUNCH`; running STUN from `node-b` or `node-c`.

---

### M1-03 · Invite encode and decode
- **Status:** done
- **Goal:** `easynet-core` emits and parses `easynet://invite/…` as a signed `InviteToken` with no socket I/O.
- **Read first:** `docs/api.md` §2 and §7.1; `docs/spec.md` §5 ( `node_id`, `mesh_id` ); `docs/adr/0005-implementation-language.md`.
- **Files:** + `crates/easynet-core/src/addr.rs`; + `crates/easynet-core/src/invite.rs`; ~ `crates/easynet-core/src/lib.rs`; ~ `crates/easynet-core/Cargo.toml`
- **Steps:**
  1. Add only these dependencies: `ed25519-dalek` 2, `x25519-dalek` 2 with feature `static_secrets`, `postcard` 1 with feature `alloc`, `sha2` 0.10. No `base64` crate. Implement unpadded base64url (RFC 4648 §5, no `=`).
  2. `pack_socket_addr` / `unpack_socket_addr` use api §2: IPv4 family `0x04` plus 4 address bytes, IPv6 family `0x06` plus 16, port big-endian. Reject a truncated buffer.
  3. `InviteToken` fields and order are api §7.1. `encode_invite(token, ed25519_secret) -> String` produces `easynet://invite/` plus unpadded base64url of `postcard(body) || sig[64]`. The signature is Ed25519 over `easynet.invite.v1\0` || `postcard(body)`. `mesh_id` is the first 8 bytes of `SHA256(root_pub_key)`. The signer’s public key equals `peer_ed25519_pub`.
  4. `decode_invite` checks the prefix, the signature, `token_type == 0x01`, `version == 0x01`, and **at most 4** candidates. Do not invent an expiry. A fifth candidate, a bad signature, and a non-invite URI are errors.
  5. Tests: round-trip one IPv4 candidate; five candidates fail; flipping one signature byte fails; `issued_at` is preserved. No clock check.
- **Acceptance:**
  - `cargo nextest run -p easynet-core` → exit 0
  - `just check` → exit 0
- **Out of scope:** Join and roster tokens; a CLI subcommand; the NAT lab; `PUNCH`.

---

### M1-04 · PUNCH codec and replay cache
- **Status:** todo
- **Goal:** `easynet-core` builds, verifies, and replay-filters `PUNCH` frames using the api §3 numbers.
- **Read first:** `docs/api.md` §3; `docs/spec.md` §5 (`node_id`) and §6 (250 ms, 30 s); `docs/tasks/M1-puncher.md` (M1-03).
- **Files:** + `crates/easynet-core/src/punch.rs`; ~ `crates/easynet-core/src/lib.rs`
- **Steps:**
  1. Frame layout is api §3. The signature covers the raw bytes before the last 64, **not** a domain tag and **not** postcard. `sender_node_id` is the first 16 bytes of `SHA256(ed25519_pub)`.
  2. `PunchCache` is an LRU of **4096** `(sender_node_id, nonce)` pairs kept **30_000 ms**. `admit(now_ms, frame_unix_ms, sender, nonce)` returns `Skew` when `abs(now_ms - frame_unix_ms) > 30_000`, `Replay` when the pair is still cached, and inserts it otherwise. When the cache is full, drop the least recently used pair. The caller passes `now_ms`. Do not read the wall clock inside the cache.
  3. `encode_punch` writes `SYN` as flags bit0 (`0x01`) and `ACK` as flags bit1 (`0x02`). `verify_punch(frame, ed25519_pub, now_ms, cache)` checks length at least `36 + 7 + 64`, version `1`, reserved `0`, node id, signature, skew, replay, then returns the flags and the packed endpoint.
  4. Tests: a frame 1 ms inside the skew window is accepted; 30_001 ms outside is `Skew`; the same `(sender, nonce)` inside 30 s is `Replay`; the 4097th distinct pair evicts the oldest; a short frame and a bad signature fail. No sockets.
- **Acceptance:**
  - `cargo nextest run -p easynet-core` → exit 0
  - `just check` → exit 0
- **Out of scope:** Sending the frame; the NAT lab; changing 30 s, 4096, or the `u64` nonce.

---

### M1-05 · Mutual punch on the cone path
- **Status:** todo
- **Goal:** `node-a` (cone) and `node-b` (restricted) exchange invites carrying STUN reflexive candidates and complete `PUNCH` SYN/ACK on one UDP port each.
- **Read first:** `docs/api.md` §3 and §7.1; `docs/dev.md` §3; `docs/adr/0003-single-socket-noise.md`; `docs/tasks/M1-puncher.md` (M1-02, M1-04).
- **Files:** + `crates/easynet-net/src/punch_io.rs`; ~ `crates/easynet-net/src/lib.rs`; ~ `crates/easynet-cli/src/bin/easynet.rs`; ~ `docker/run-nat-tests.sh`
- **Steps:**
  1. `easynet invite issue --state-dir DIR --candidate HOST:PORT` creates ed25519, x25519, and root key files in `DIR` when they are absent (`0600`), reuses them when present, and prints one `easynet://invite/…` line. The single candidate is that `HOST:PORT` packed as `SocketAddrBin`. `mesh_id` is derived from the stored root public key.
  2. `easynet punch --state-dir DIR --bind IP:PORT --peer-uri URI` binds **one** UDP socket. It sends `PUNCH` `SYN` (flags `0x01`) to every candidate in the peer invite every **250 ms**, from that socket. On a valid `ACK` (flags bit1 set, `verify_punch` succeeds for the peer’s ed25519 key) it prints `punch-ack` and exits 0. Exit 1 after 5 seconds without an ACK. Reply to a valid `SYN` with one `ACK` to the packet’s source address, not to the address inside the frame. Do not answer an `ACK` with another frame.
  3. Append after `m1-stun-ok` in `docker/run-nat-tests.sh`. Rebuild the musl binary the same way as M1-02. Copy it to `node-a` and `node-b`. On each, run `stun` on `0.0.0.0:51820` against `172.30.0.10:3478`, then `invite issue` with **that reflexive** `IP:PORT` as the only candidate. Pass each URI to the other side. Start both `punch` processes. Require both stdout streams to contain `punch-ack`, then `echo m1-punch-ok`. Do not use `node-c`. Do not put `10.10.1.2` or `10.10.2.2` in the invite.
- **Acceptance:**
  - `just lab-up` → exit 0; `just lab-test` stdout contains `nat-lab-topology-ok`, `m1-stun-ok`, and `m1-punch-ok`, in that order; `just lab-down`
  - `just check` → exit 0
- **Out of scope:** Keepalive; `node-c`; birthday probes; `RELAY`; TUN.

---

### M1-06 · Fifteen-second keepalive
- **Status:** todo
- **Goal:** After `PUNCH` ACK, each side sends one keepalive `PUNCH` at 15 seconds and does not answer that frame.
- **Read first:** `docs/spec.md` §6; `docs/api.md` §3; `docs/tasks/M1-puncher.md` (M1-05).
- **Files:** + `crates/easynet-core/src/schedule.rs`; ~ `crates/easynet-core/src/lib.rs`; ~ `crates/easynet-net/src/punch_io.rs`; ~ `crates/easynet-cli/src/bin/easynet.rs`; ~ `docker/run-nat-tests.sh`
- **Steps:**
  1. `easynet-core` function `due(last_ms, now_ms, interval_ms) -> bool` is true when `now_ms - last_ms >= interval_ms`. Constants: probe **250**, keepalive **15_000**. The core function does not sleep and does not read the clock.
  2. While `punch` is waiting for the first ACK, send one STUN Binding Request on the same socket when 15_000 ms have passed since the previous Binding Request. After ACK, stop SYN probes. Every 15_000 ms send one `PUNCH` with flags `0x02` (`ACK` only), a new `nonce`, and the current `unix_ms`. Do not send a reply to a flags-`0x02` frame. A second `ACK` received after the first is a failure.
  3. `easynet punch` gains `--hold-secs N`. `N = 0` keeps today’s exit-on-first-ACK behavior. `N > 0` stays up that many seconds after the first ACK, prints `keepalive-sent` when the first keepalive goes out, and exits 0 only if that line was printed and no extra ACK arrived.
  4. In `docker/run-nat-tests.sh`, change the M1-05 punch invocation to `--hold-secs 16`. Require `keepalive-sent` on both sides, then `echo m1-keepalive-ok`. Leave `nat-lab-topology-ok`, `m1-stun-ok`, and `m1-punch-ok` in place.
  5. Unit test the scheduler with a fake `now_ms`: not due at 14_999, due at 15_000. Do not shrink 15_000 to speed the lab up.
- **Acceptance:**
  - `cargo nextest run -p easynet-core` → exit 0
  - `just lab-up` → exit 0; `just lab-test` stdout contains `nat-lab-topology-ok`, `m1-stun-ok`, `m1-punch-ok`, and `m1-keepalive-ok`, in that order; `just lab-down`
  - `just check` → exit 0
- **Out of scope:** A new UDP type for keepalive; birthday probes; `RELAY`; TUN; Noise.
