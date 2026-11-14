# ADR-0007: Relay eligibility and abuse limits

- **Status:** Superseded by ADR-0009
- **Date:** 2026-11-14

## Context

Dual-symmetric NAT needs `0x04 RELAY` through authenticated peers, but
mobile battery and bandwidth abuse are risks. Owner locked Q9 on
2026-11-14.

## Decision

1. **Eligibility:** `EndpointRecord.relay_capable` is signed gossip state.
   Default `true` on desktop/server (Linux, macOS, Windows) behind cone
   or public paths; default `false` on Android/iOS and on battery.
   Override with CLI `--no-relay` / config at any time.
2. **Hop limit:** `RELAY` frames use `ttl = 1`. A relay never forwards a
   relayed frame to another relay.
3. **Rate limit:** per `(source_peer, dest_peer)` token bucket default
   **10 Mbps** sustained, **2 MB** burst; configurable as
   `relay_max_bps` in `easynet.toml`. Excess drops; do not starve the
   relay's own mesh traffic.
4. Relays still must not decrypt inner `DATA` ([ADR-0003](0003-single-socket-noise.md)).

## Consequences

- Path selection may only pick relays advertising `relay_capable`.
- Tests: ttl≠1 dropped; rate limiter trips; mobile default false.

## Alternatives considered

- **Every peer always relays** — bad for cellular; rejected.
- **Multi-hop relay** — amplification risk; rejected for v1.
