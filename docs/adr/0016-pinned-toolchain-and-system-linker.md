# ADR-0016: Pin Rust 1.85.0 and keep the system linker

- **Status:** Accepted
- **Date:** 2026-11-14
- **Amends:** [ADR-0011](0011-dev-toolchain-and-nat-lab.md) items 1 and 4 only.

## Context

ADR-0011 required a toolchain file and a fast linker, but not the file's bytes. It also allowed `lld` or `mold`, which fails CI on machines where those linkers are absent. The scaffold card ran `cargo build` before the toolchain file existed. Owner locked the file and the linker on 2026-11-14.

## Decision

1. M0-02 creates `rust-toolchain.toml` before the first `cargo` invocation. The file is exactly:

   ```toml
   [toolchain]
   channel = "1.85.0"
   components = ["rustfmt", "clippy"]
   ```

   Cross targets are installed by CI steps, not by this file.
2. `.cargo/config.toml` does not set `linker` or `-fuse-ld`. The system linker is the default. A developer who already has `lld` may export `RUSTFLAGS="-C link-arg=-fuse-ld=lld"` in their own shell. CI must not set that variable.
3. NAT lab images, subnets, and iptables are the text in [docs/dev.md](../dev.md) §3. `docker/run-nat-tests.sh` must prove topology in M0-04. It must not exit 0 without those asserts.

## Consequences

- M0-02's allowlist includes `rust-toolchain.toml`. M0-03 does not recreate it.
- A compose file that uses a different STUN image or omits `--random-fully` fails review.

## Alternatives considered

- **`mold` on every platform** — missing binary fails the build; rejected.
- **Defer the toolchain pin until after the first build** — Edition 2024 then depends on whatever `rustc` is on `PATH`; rejected.
