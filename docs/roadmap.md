# Roadmap

Work proceeds in small, runnable milestones. Each milestone is split
into task cards in [`tasks/`](tasks/README.md). The checkboxes mirror
those cards. Each milestone ends with a green check command, updated
docs, and a progress entry.

**Next task:** see the top of [`progress.md`](progress.md).

Status legend: `[ ]` todo · `[~]` in progress · `[x]` done

## M0 · Repository bootstrap — [task cards](tasks/M0-bootstrap.md)

- [x] M0-01 Choose language and record ADR + check command
- [x] M0-02 Scaffold module layout and empty daemon entrypoint
- [x] M0-03 Green `just check`
- [ ] M0-04 NAT lab skeleton, deny, linker, CI, devcontainer
- [ ] **M1-00 Write task cards (planning — strong model or owner, owner reviews)**

**Done when:** `just check` is green; `rust-toolchain.toml` is the 1.85.0 pin
from ADR-0016; `justfile` exists; `docker compose -f docker/nat-lab.yml config`
succeeds; `just lab-test` proves the topology in [`dev.md`](dev.md) §3 (not an
empty success); CI + devcontainer exist; `easynetd` / `easynet` build.

## M1 · Single-socket puncher

Execution cards are not written yet. [`M1-00`](tasks/M0-bootstrap.md) writes
them. Do not start `M1-01` until the owner has reviewed that file. Demux and
the punch cache are already locked (ADR-0012).

**Done when:** Two NATed peers (prefer `just lab-test` cone path) exchange
`easynet://invite/…`, complete `PUNCH` SYN/ACK on one UDP port, hold 15s
keepalive; STUN reflexive candidates from lab STUN.

## M2 · Noise encryption & TUN data plane

Planning card: [`M2-00`](tasks/M2-noise.md), after M1. Implement ADR-0010 and ADR-0013. Do not reinvent the prologue or the nonce.

**Done when:** `0x05` handshake + `0x03` DATA carry TUN traffic; `easynet0`/`utun`
up with MTU 1380 and ADR-0008 VIP; `ping` (and preferably `ssh`) works
between two peers (lab and/or privileged host).

## M3 · SWIM gossip, certs, and relay

Planning card: [`M3-00`](tasks/M3-gossip.md), after M2. SWIM timers, relay-of-handshake, and renewal are locked (ADR-0012, ADR-0014).

**Done when:** Third peer joins via admin-signed `PeerCert`; gossip syncs
endpoints; Symmetric path exercises birthday probe / `RELAY` under ADR-0009
in the NAT lab; revocation with `RevokeReason`; `RENEW_REQ`/`RENEW_RESP` work.

## M4 · Admin portal

Planning card: [`M4-00`](tasks/M4-admin.md), after M3. **Q15 mesh DNS is out of scope.** ADR-0015 locks the proxy, IPC, bearer, and the JSON that already has bodies. M4-00 still writes the three remaining GET objects before any handler card.

**Done when:** Every node serves `127.0.0.1:8080` as a reverse proxy to the
lowest-ID `ALIVE` admin (not a 302); VIP `:8080` admin-only; `easynet status`
prints admin URLs; member read-only + admin mutations (Q14 / ADR-0015); copy
keys from `uicopy/en.json`; `approve` returns `easynet://roster/…`; HMAC
webhook dedupe obeyed.
**Does not include** `admin.easynet` / TUN `:53` DNS.

## M5 · Nice-to-have / later (optional tracks)

No cards until M4 is done and the owner opens the relevant `M5-*-00` planning
card. Tracks may proceed independently:

| Track | Outcome |
|---|---|
| **Mesh DNS (Q15)** | TUN VIP `:53` responder for `*.easynet` per [api.md §10](api.md#10-deferred-q15-mesh-dns) |
| **Mobile wrappers** | `easynet-ffi` + Android `VpnService` / iOS `NEPacketTunnelProvider` |
