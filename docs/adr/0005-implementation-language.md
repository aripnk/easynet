# ADR-0005: Implement EasyNet in Rust

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

M0 needs a single language, module layout, and check command. The product
needs zero-GC forwarding, one-binary distribution, and a clean path to
mobile FFI (`uniffi`) for later Network Extension / VpnService wrappers.
Owner locked Q1 on 2026-11-14.

## Decision

- Implement in **Rust Edition 2024**, **MSRV 1.85+**, with a **tokio**
  multi-threaded async runtime.
- Cargo **workspace** under `crates/`:
  - `easynet-core` — pure state machines (no OS/socket I/O): identity,
    wire, swim, traversal
  - `easynet-net` — Tokio UDP mux, STUN/UPnP, TUN I/O
  - `easynet-admin` — Axum admin portal + webhook notifier
  - `easynet-cli` — `easynetd` daemon + `easynet` CLI
  - `easynet-ffi` — UniFFI bindings (used from M5; crate may be stubbed earlier)
- Core dependencies (do not substitute without a superseding ADR):
  - `tun-rs` — TUN / utun / Wintun
  - `snow` with `StatelessTransportState` — Noise IK
    `Noise_IK_25519_ChaChaPoly_BLAKE2s`
  - `ed25519-dalek`, `x25519-dalek`
  - `axum` — admin HTTP
  - `postcard` — canonical binary serialization for wire and signed bodies
- **Check command:** superseded by [ADR-0011](0011-dev-toolchain-and-nat-lab.md)
  — use **`just check`** (`fmt` + `clippy -D warnings` + `cargo nextest`).

## Consequences

- Agents must not introduce a Go tree or alternate Noise stack.
- `easynet-core` stays I/O-free so VIP/cert/wire tests run without root/TUN.
- M0-02 scaffolds this workspace; M0-03+ make `just check` green (ADR-0011).

## Alternatives considered

- **Go** — strong WireGuard/TUN ecosystem; weaker zero-cost FFI story for
  the planned mobile wrappers; rejected by owner.
- **Noise via a different crate than `snow`** — would reopen UDP nonce
  handling; not without a new ADR.
