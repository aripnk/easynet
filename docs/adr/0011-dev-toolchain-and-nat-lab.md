# ADR-0011: just + nextest check, pinned toolchain, NAT lab

- **Status:** Accepted. Toolchain-file contents and the linker (items 1 and 4) are amended by [ADR-0016](0016-pinned-toolchain-and-system-linker.md). The `just check` recipe is preceded by `scope-check` ([ADR-0017](0017-card-scope-gate.md)). `just`, nextest, NAT lab, deny, devcontainer, and CI remain in force.
- **Date:** 2026-11-14
- **Supersedes:** Check-command wording in [ADR-0005](0005-implementation-language.md) (`make check` / `cargo test`). Language, crates, and deps in ADR-0005 remain in force.

## Context

TUN needs `CAP_NET_ADMIN` / root; UDP hole punching and Symmetric NAT cannot
be validated on `127.0.0.1` alone. Fast agent iteration needs a pinned
compiler, a single discoverable task runner, parallel tests, and a
containerized multi-router lab. Owner locked this shape on 2026-11-14.

## Decision

Where item 1 or item 4 differs from [ADR-0016](0016-pinned-toolchain-and-system-linker.md), ADR-0016 wins.

1. **`rust-toolchain.toml`** at repo root pins channel/MSRV-compatible
   toolchain, components `rustfmt` + `clippy`, and cross targets needed for
   CI (`aarch64-apple-darwin`, `x86_64-unknown-linux-musl`, plus Windows as
   used in CI matrix).
2. **`just`** (not Make) is the task runner. `justfile` uses `set dotenv-load`.
   Session check command is **`just check`**, which runs:

   ```just
   check:
       cargo fmt --check
       cargo clippy --workspace --all-targets -- -D warnings
       cargo nextest run --workspace
   ```

   Also provide `just lab-up` and `just lab-test` wrapping the NAT compose file.
3. **`cargo-nextest`** replaces `cargo test` in the check path (parallel,
   process-isolated).
4. **Fast linker:** `.cargo/config.toml` uses `lld` and/or `mold` where
   supported to cut incremental link time.
5. **`cargo-deny`** with `deny.toml` — run in CI (and optionally a
   `just deny` recipe); agents must not ignore deny failures when adding crates.
6. **NAT lab:** `docker/nat-lab.yml` (Linux or OrbStack/Docker Desktop) with
   `cap_add: [NET_ADMIN]`, `/dev/net/tun`, topology documented in
   [`docs/dev.md`](../dev.md): `wan` + local STUN, cone (`router-a`/`node-a`),
   port-restricted (`router-b`/`node-b`), symmetric (`router-c`/`node-c` with
   `--random-fully`), optional `tc netem` chaos.
7. **Dev container** `.devcontainer/devcontainer.json` with
   `--cap-add=NET_ADMIN --device=/dev/net/tun` and the Rust/`just`/nextest/
   iptables tooling.
8. **CI** `.github/workflows/ci.yml`: (a) matrix unit/fmt/clippy/nextest on
   ubuntu/macOS/windows; (b) privileged ubuntu job that brings up the NAT lab
   and runs integration scripts.

## Consequences

- Agents finish sessions with `just check`, not `make check` or bare
  `cargo test`.
- Punch/relay acceptance in M1+ should prefer `just lab-test` over host TUN
  when the card says so.
- Host macOS/Windows developers use Docker/OrbStack or the devcontainer for
  privileged lab work; pure `easynet-core` tests stay root-free.

## Alternatives considered

- **Makefile** — tab/phony friction for agents; rejected.
- **Only localhost tests** — cannot prove NAT traversal; rejected.
- **External public STUN in every test** — flaky offline CI; lab ships local STUN.
