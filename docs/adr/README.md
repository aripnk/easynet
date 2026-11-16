# Architecture Decision Records

Decisions that are costly to reverse, or that a new contributor would
otherwise re-litigate. Format: [`template.md`](template.md).
Process: [ADR-0001](0001-record-architecture-decisions.md).

| # | Title | Status |
|---|---|---|
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Accepted |
| [0002](0002-zero-server-admin-trust.md) | Zero-server membership via admin-signed certificates | Accepted |
| [0003](0003-single-socket-noise.md) | Single UDP socket multiplexing and Noise IK data plane | Accepted |
| [0004](0004-deterministic-virtual-ip.md) | Deterministic virtual IP across 100.64.0.0/10 | Superseded by ADR-0008 |
| [0005](0005-implementation-language.md) | Implement EasyNet in Rust | Accepted |
| [0006](0006-offline-root-and-admin-certs.md) | Offline root CA and delegated AdminCerts | Accepted |
| [0007](0007-relay-policy.md) | Relay eligibility and abuse limits | Superseded by ADR-0009 |
| [0008](0008-vip-salt-unified-hash.md) | Unified VIP hash always includes vip_salt | Accepted |
| [0009](0009-relay-two-knob-policy.md) | Relay eligibility is cert policy ∧ runtime flag | Accepted |
| [0010](0010-noise-handshake-mux.md) | Noise IK handshake as UDP type 0x05 | Accepted |
| [0011](0011-dev-toolchain-and-nat-lab.md) | just + nextest check, pinned toolchain, NAT lab | Accepted (toolchain file + linker amended by 0016) |
| [0012](0012-stun-demux-and-relay-handshake.md) | STUN-before-mesh demux; RELAY may wrap HANDSHAKE | Accepted |
| [0013](0013-noise-prologue-and-snow-nonce.md) | Noise prologue and snow stateless nonces | Accepted |
| [0014](0014-swim-failure-detector.md) | SWIM payloads and failure-detector timers | Accepted |
| [0015](0015-admin-reverse-proxy-and-auth.md) | Loopback reverse proxy, IPC, admin HTTP shapes | Accepted |
| [0016](0016-pinned-toolchain-and-system-linker.md) | Rust 1.85.0 file and system linker | Accepted |
| [0017](0017-card-scope-gate.md) | just check rejects paths outside the active card | Accepted (status line amended by 0018) |
| [0018](0018-card-status-line.md) | Card status line is inside the scope gate | Accepted |

When adding an ADR, add a row here in the same change.
