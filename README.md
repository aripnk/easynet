# EasyNet

EasyNet was created to deliver the seamless, zero-config connectivity of modern mesh VPNs like Tailscale without relying on third-party accounts, user or device caps, paid relay subscriptions, or heavy enterprise control planes like OpenZiti.

A user-space daemon, `easynetd`, gives each device a Layer-3 address so ordinary programs (`ping`, `ssh`, HTTP, SMB) can reach each other across NATs.

There is no project-operated coordination server, no device cap, and no master IP. Devices pair with `easynet://` tokens, discover themselves with public STUN and local UPnP/NAT-PMP, and keep membership in admin-signed certificates. Direct paths are preferred. When two symmetric NATs block a direct path, a reachable peer relays the encrypted frames and does not decrypt them.

Virtual addresses come from `100.64.0.0/10`. The admin portal is `http://127.0.0.1:8080` on every node.

## Status

The protocol, admin API, and milestone plan are written. The daemon is not built yet. The next implementation step is the empty Rust workspace (`M0-02` in [`docs/progress.md`](docs/progress.md)).

Mobile clients are later. Android and iOS wait until the desktop and server daemon has proved punching, encryption, and membership.

## Docs

| Read | For |
|---|---|
| [`docs/spec.md`](docs/spec.md) | What the system does |
| [`docs/api.md`](docs/api.md) | Wire and admin HTTP shapes |
| [`docs/design.md`](docs/design.md) | Admin portal |
| [`docs/roadmap.md`](docs/roadmap.md) | Milestones |
| [`docs/adr/`](docs/adr/README.md) | Why a decision was made |
| [`docs/dev.md`](docs/dev.md) | Toolchain and NAT lab |
| [`AGENTS.md`](AGENTS.md) | How a coding agent must work in this repo |

Start a coding session at [`docs/progress.md`](docs/progress.md). Do one card. [`docs/plan.md`](docs/plan.md) is an old brainstorm and is not the spec.
