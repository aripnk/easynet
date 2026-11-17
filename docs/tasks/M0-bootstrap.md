# M0 · Repository bootstrap

Cards for toolchain, empty skeleton, and lab/CI stubs. Do not implement
punching, TUN data plane, or gossip protocol here.

---

### M0-01 · Choose language and record ADR + check command
- **Status:** done
- **Goal:** Owner-selected language is written into the spec and a language ADR; check command named for later cards.
- **Read first:** `AGENTS.md`; `docs/spec.md` §3; `docs/adr/README.md`.
- **Files:** (historical) ADR-0005 + stack docs; check command later refined by ADR-0011 to `just check`.
- **Steps:** Completed earlier; Rust locked.
- **Acceptance:** ADR-0005 exists; language chosen in spec.
- **Out of scope:** Source scaffold (M0-02).

---

### M0-02 · Scaffold module layout, toolchain pin, and config parser
- **Status:** done
- **Goal:** Workspace builds on Rust 1.85.0; both binaries print help; `easynet config init` writes the spec §9 file; `easynet-core` parses that file.
- **Read first:** `docs/spec.md` §4 Repository layout and §9; `docs/adr/0005-implementation-language.md`; `docs/adr/0016-pinned-toolchain-and-system-linker.md`; `docs/conventions/transport.md` (first line).
- **Files:** + `rust-toolchain.toml`; + `Cargo.toml` (workspace); + `crates/easynet-core/Cargo.toml`; + `crates/easynet-core/src/lib.rs`; + `crates/easynet-core/src/config.rs`; + `crates/easynet-net/Cargo.toml`; + `crates/easynet-net/src/lib.rs`; + `crates/easynet-admin/Cargo.toml`; + `crates/easynet-admin/src/lib.rs`; + `crates/easynet-cli/Cargo.toml`; + `crates/easynet-cli/src/bin/easynetd.rs`; + `crates/easynet-cli/src/bin/easynet.rs`; + `crates/easynet-ffi/Cargo.toml`; + `crates/easynet-ffi/src/lib.rs`; ~ `docs/roadmap.md`; ~ `docs/progress.md`
- **Steps:**
  1. Write `rust-toolchain.toml` **before** any `cargo` command. Exact contents (no `targets` key):
     ```toml
     [toolchain]
     channel = "1.85.0"
     components = ["rustfmt", "clippy"]
     ```
  2. Create the workspace and crates under **Files**. Edition `2024`. No protocol logic.
  3. `easynet-core` depends on `serde` 1 with feature `derive`, and `toml` **0.8**. No other parser. `config.rs` defines `Config` with `node`, `discovery`, `relay`, and `admin` matching spec §9 key names (`listen_port`, `tun_name`, `tun_mtu`, `state_dir`, `stun_servers`, `enable_upnp`, `keepalive_secs`, `allow_relay`, `relay_max_bps`, `relay_burst_bytes`, `bind_port`, `webhook_url`, `webhook_secret`). `pub fn default_toml() -> &'static str` returns the Unix sample from spec §9. `pub fn parse_toml(text: &str) -> Result<Config, String>`. No filesystem calls in this crate.
  4. Unit test: `parse_toml(default_toml())` yields `listen_port == 51820`, `allow_relay == true`, `relay_max_bps == 1_250_000`, `relay_burst_bytes == 2_097_152`.
  5. `easynetd` and `easynet` print a one-line stub containing `easynet` and exit 0 on `--help` or `-h`.
  6. `easynet config init` writes the file only if the destination is absent (exit 0 either way, print the path). `--path <file>` writes the `default_toml()` bytes unchanged. With no `--path`, use the user path from spec §9 (do not require root). The CLI does the write; core only supplies the string and the parser.
  7. Do not add `snow`, `tun-rs`, or `axum`. `easynet-ffi` is an empty lib. `easynet-net` and `easynet-admin` are empty libs.
- **Acceptance:**
  - `grep -q 'channel = "1.85.0"' rust-toolchain.toml` → exit 0
  - `cargo build -p easynet-cli` → exit 0
  - `cargo test -p easynet-core` → exit 0
  - `cargo run -p easynet-cli --bin easynetd -- --help` → stdout contains `easynet`
  - `cargo run -p easynet-cli --bin easynet -- --help` → stdout contains `easynet`
  - `cargo run -p easynet-cli --bin easynet -- config init --path /tmp/easynet-m002.toml` → exit 0, and `grep -q 'relay_max_bps = 1250000' /tmp/easynet-m002.toml`
  - `just scope-check` → stdout contains `card-scope-ok`
- **Out of scope:** `justfile`, `scripts/check-card-scope.sh`, NAT lab, UDP/STUN, admin HTTP, real `init` key generation. Do not edit those two files to silence the scope gate.

---

### M0-03 · Green `just check`
- **Status:** done
- **Goal:** `just check` runs the existing scope gate, then fmt, clippy `-D warnings`, and `cargo nextest` on the M0-02 skeleton.
- **Read first:** `AGENTS.md` §Commands; `docs/adr/0011-dev-toolchain-and-nat-lab.md`; `docs/adr/0016-pinned-toolchain-and-system-linker.md`; `docs/adr/0017-card-scope-gate.md`; `docs/dev.md` §2.
- **Files:** ~ `justfile`; ~ `docs/roadmap.md`; ~ `docs/progress.md`
- **Steps:**
  1. Do not create or edit `rust-toolchain.toml`. M0-02 already pinned channel `1.85.0`.
  2. `justfile` and `scripts/check-card-scope.sh` already exist (ADR-0017). Do not delete `scope-check`. `check` must depend on `scope-check` and then run exactly:
     - `cargo fmt --check`
     - `cargo clippy --workspace --all-targets -- -D warnings`
     - `cargo nextest run --workspace`
  3. Leave the M0-02 config unit test in place. Do not replace it.
  4. Do **not** add a Makefile. Tick M0-03; set **Next task** to `M0-04`.
- **Acceptance:**
  - `test -f rust-toolchain.toml && test -f justfile` → exit 0
  - `just check` → exit 0
- **Out of scope:** NAT compose, CI workflow, deny.toml, linker config (M0-04).

---

### M0-04 · NAT lab, deny, linker, CI, devcontainer
- **Status:** done
- **Goal:** The files in `docs/dev.md` §§3–4 exist byte-for-byte, and `just lab-test` prints `nat-lab-topology-ok` only after the topology asserts.
- **Read first:** `docs/dev.md` §§3–4; `docs/adr/0011-dev-toolchain-and-nat-lab.md`; `docs/adr/0016-pinned-toolchain-and-system-linker.md`.
- **Files:** + `docker/nat-lab.yml`; + `docker/router.Dockerfile`; + `docker/node.Dockerfile`; + `docker/router-entry.sh`; + `docker/node-entry.sh`; + `docker/run-nat-tests.sh`; + `deny.toml`; + `.cargo/config.toml`; + `.github/workflows/ci.yml`; + `.devcontainer/devcontainer.json`; ~ `justfile`; ~ `docs/roadmap.md`; ~ `docs/progress.md`
- **Steps:**
  1. Copy each fenced file in `docs/dev.md` §§3–4 into the path named on that fence. Do not change images, subnets, iptables, or the test script.
  2. `chmod +x docker/router-entry.sh docker/node-entry.sh docker/run-nat-tests.sh`.
  3. Add just recipes, and no others in this card:
     - `lab-up`: `docker compose -f docker/nat-lab.yml up -d --build`
     - `lab-test`: `sh docker/run-nat-tests.sh`
     - `lab-down`: `docker compose -f docker/nat-lab.yml down -v`
     - `deny`: `cargo deny check`
  4. Do not set `continue-on-error` on the lab job. Do not remove the `scope` job. Do not put `linker` or `-fuse-ld` in `.cargo/config.toml`. Do not make `run-nat-tests.sh` exit 0 before the asserts.
  5. Tick M0-04. Set **Next task** to `M1-00`.
- **Acceptance:**
  - `docker compose -f docker/nat-lab.yml config` → exit 0
  - `grep -q 'coturn/coturn:4.6.2-alpine' docker/nat-lab.yml` → exit 0
  - `grep -q 'random-fully' docker/router-entry.sh` → exit 0
  - `grep -q 'nat-lab-topology-ok' docker/run-nat-tests.sh` → exit 0
  - `just lab-up` → exit 0, then `just lab-test` → stdout contains `nat-lab-topology-ok`, then `just lab-down`
  - `just check` → exit 0
- **Out of scope:** Running `easynetd` inside the lab; mesh VIP ping; changing spec or API.

---

### M1-00 · Write M1 task cards (planning)
- **Status:** done
- **Goal:** Owner-reviewed execution cards for M1 (single-socket puncher) that use `just check` and, where needed, `just lab-test`.
- **Read first:** `docs/spec.md` §6; `docs/api.md` §1, §3, and §7.1; `docs/dev.md` §3; `docs/adr/0003-single-socket-noise.md`; `docs/adr/0012-stun-demux-and-relay-handshake.md`; `docs/adr/0016-pinned-toolchain-and-system-linker.md`; `docs/tasks/README.md` §Planning tasks.
- **Steps:**
  1. If M0 is incomplete, list that under **Blocked** and stop.
  2. Write M1 execution cards (STUN on the one socket → invite encode/decode → mutual punch → keepalive). Copy demux order and the punch cache from api §1 and §3. Do not change the 30 s / 4096 cache, the `u64` nonce, or the STUN-before-mesh rule. Lab asserts append to `docker/run-nat-tests.sh`; they do not replace the topology asserts. Host TUN is out of M1.
  3. Add roadmap checkboxes. Keep **Next task** as `M1-00` until the owner accepts the card file.
- **Files:** + `docs/tasks/M1-puncher.md`; ~ `docs/roadmap.md`; ~ `docs/progress.md`
- **Acceptance:**
  - Owner review: every execution card has allowlist + command; no “choose library”; lab vs unit test split is explicit.
- **Out of scope:** Implementing the puncher; M2+ cards.
