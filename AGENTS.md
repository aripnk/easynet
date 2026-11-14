# AGENTS.md

Operating guide for AI agents and human contributors. Read this first,
every session.

## What this is

EasyNet is a zero-control-plane P2P overlay VPN: user-space `easynetd` gives
peers Layer-3 connectivity (`ping`, `ssh`, …) across NATs using out-of-band
`easynet://` tokens, public STUN/UPnP, SWIM gossip, and admin-signed
membership certs (Rust workspace). It must never require a project-hosted
coordination server, never decrypt relayed `DATA` on intermediate peers, and
never bind the admin portal on a LAN/WAN interface.

## Start of every session (do this in order)

1. Read [`docs/progress.md`](docs/progress.md): **Next task** and **Blocked**.
   Read the newest entry. Do not read the whole log.
2. Open the task card it points to in [`docs/tasks/`](docs/tasks/README.md).
   Read only the docs the card lists under **Read first**.
3. Do that **one card**. Touch only the files it lists.
4. Run the card's **Acceptance** commands; all must pass. Then run
   `just scope-check` (it must print `card-scope-ok`). After M0-03, also run
   `just check`, which runs the scope gate before fmt, clippy, and nextest.
5. Mark the card `done`, tick it in `docs/roadmap.md`, update
   **Next task**, and add an entry in `docs/progress.md`.
   Commit as `M?-NN: title` only when the user asked to commit.

If the next task is a planning task (`Mx-00`) and you are not explicitly
asked to plan, **stop and tell the user**. Planning needs a strong model
or the owner.

## Stop and ask (don't guess) when

- an acceptance command fails and the fix is outside the card's files;
- the card, `docs/spec.md`, `docs/api.md`, and the ADRs disagree;
- you need a decision the card doesn't make: a library, a JSON field, a
  UI string, a file location, a default value;
- a change would weaken an invariant below;
- the item is listed as an open question in the spec, or is marked deferred
  (e.g. Q15 mesh DNS — do not implement in M4).

Record the question under **Blocked** in `docs/progress.md` and stop.

## Where things are

| Need | Read |
|---|---|
| Next task, handoff notes | `docs/progress.md` |
| Step-by-step task cards | `docs/tasks/` |
| What the system must do | `docs/spec.md` |
| Wire + admin HTTP shapes | `docs/api.md` |
| Admin UI tokens/screens | `docs/design.md` |
| User-facing strings | `uicopy/en.json` |
| Why it is built this way | `docs/adr/README.md` |
| Milestones | `docs/roadmap.md` |
| Vocabulary | `docs/glossary.md` |
| Layer rules | `docs/conventions/` |
| Dev toolchain & NAT lab | `docs/dev.md` |
| Doc index | `docs/README.md` |
| Historical design dump | `docs/plan.md` (not authoritative) |

## Commands

Host: Rust via `rust-toolchain.toml` (Edition 2024 / MSRV 1.85+), **`just`**,
**`cargo-nextest`**. Repo root. [ADR-0005](docs/adr/0005-implementation-language.md),
[ADR-0011](docs/adr/0011-dev-toolchain-and-nat-lab.md), [`docs/dev.md`](docs/dev.md).

| Action | Command |
|---|---|
| List recipes | `just --list` |
| Format check | `cargo fmt --check` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Test | `cargo nextest run --workspace` |
| Scope gate | **`just scope-check`** → `scripts/check-card-scope.sh` |
| Check (session may finish) | **`just check`** (scope gate, then fmt + clippy + nextest) |
| NAT lab | `just lab-up` / `just lab-test` (created in M0-04) |
| Dependency policy | `just deny` → `cargo deny check` (M0-04 / CI) |

Until M0-03 completes, Rust fmt/clippy/nextest are not required. `just scope-check` is required on every card, including M0-02.

**Devex invariants for agents:** never bind a second primary UDP socket for
STUN/punching; keep state machines in `easynet-core` (no business logic I/O in
`easynet-net`); prefer the Docker NAT lab over host TUN when a card names
`just lab-test`.

## Non-negotiable invariants

Breaking any of these is a bug, regardless of what a task says.

1. No project-operated control plane; discovery may use public STUN and local UPnP/NAT-PMP only.
2. `DATA` requires a valid non-revoked `PeerCert` (bootstrap exception only as documented).
3. `RevokeCert` always wins over `PeerCert` for the same peer key; drop revoked traffic.
4. One primary UDP socket multiplexes mesh protocols (default `:51820`).
5. Relays forward unmodified `0x03 DATA` or `0x05 HANDSHAKE` only, and must not decrypt either.
6. Virtual IP is derived per ADR-0008 (always include `vip_salt` byte in the hash); never hard-code `100.64.0.1` as admin.
7. Endpoint conflicts resolve by highest signed `incarnation`.
8. Admin HTTP: every node binds `127.0.0.1:8080` as a reverse proxy (not a 302) unless it is the chosen admin; only `AdminCert` nodes bind VIP `:8080`; members must not; never LAN/WAN. No master IP. Mutating actions need `AdminCert` auth.
9. Only the alive admin with lexicographically lowest `node_id` sends join webhooks.
10. Unsigned membership mutations are ignored; signatures are domain-tag + postcard only.

## Code conventions

| Crate / area | Rules |
|---|---|
| `easynet-core` wire/identity | [`docs/conventions/transport.md`](docs/conventions/transport.md), [`crypto.md`](docs/conventions/crypto.md) |
| SWIM / certs | [`docs/conventions/gossip.md`](docs/conventions/gossip.md) |
| TUN / net | [`docs/conventions/tun.md`](docs/conventions/tun.md) |
| Admin | [`docs/conventions/admin.md`](docs/conventions/admin.md) |

`easynet-core` must stay free of OS/socket I/O.

## Testing expectations

- Pure VIP/cert/wire: table tests in `easynet-core` (bad sig, revoke wins, VIP vectors, collision loser).
- Transport demux: golden packets for `0x01`–`0x05`. A STUN success whose first byte is `0x01` and whose bytes 4..8 are the magic cookie is STUN, not `PUNCH` (api §1).
- Integration: prefer Docker NAT lab (`just lab-test`) for punch/relay; TUN ping in M2.
- After M0-03, **`just check`** must stay green (fmt + clippy + nextest).

## Docs and decisions

- Behavior change → update `docs/spec.md` in the same change.
- Wire/HTTP change → update `docs/api.md` first, in the same change as the code.
- UI string → add key to `uicopy/en.json`; cards name the key.
- New significant decision → new ADR, status **Proposed**, row in the index.
  Only the owner marks it Accepted. Never edit an Accepted ADR's decision;
  supersede it.
- Don't add features that are not in the spec, the roadmap, or a task card.

## Card file scope gate

`just check` runs `scripts/check-card-scope.sh` first. The script reads the active card from the committed **Next task** in `docs/progress.md` and fails if any modified, staged, or untracked file lies outside that card's `**Files:**` list (plus `docs/progress.md`). A path the card does not list is a stop: record it under **Blocked**. Do not widen the allowlist, and do not retarget **Next task**, in order to make the gate pass. [ADR-0017](docs/adr/0017-card-scope-gate.md).

## Definition of done (per card)

- [ ] Card acceptance commands pass; `just scope-check` prints `card-scope-ok`; `just check` passes once M0-03 has left it in place
- [ ] Tests cover the new behavior, including failure paths
- [ ] Spec / api / design / copy / ADRs updated if behavior changed
- [ ] Card marked `done`, roadmap ticked, `docs/progress.md` updated
- [ ] No invariant weakened
