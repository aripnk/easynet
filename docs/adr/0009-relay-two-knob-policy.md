# ADR-0009: Relay eligibility is cert policy ∧ runtime flag

- **Status:** Accepted
- **Date:** 2026-11-14
- **Supersedes:** [ADR-0007](0007-relay-policy.md)

## Context

`PeerCert.role` and `EndpointRecord.relay_capable` looked redundant. Owner
confirmed they are distinct layers (2026-11-14). ADR-0007 only documented the
runtime flag.

## Decision

1. **`PeerCert.role`:** admin-enforced policy. `0 = Member`, `1 = RelayDisabled`.
   A node must not act as relay if marked `RelayDisabled` (cannot self-promote).
2. **`EndpointRecord.relay_capable`:** runtime/ops flag controlled by the node
   (`false` by default on iOS/Android, battery, or `--no-relay`; desktop/server
   default `true` when otherwise eligible).
3. **Effective relay rule:** a peer may forward `0x04 RELAY` iff
   `cert.role == Member (0)` **and** `endpoint.relay_capable == true`.
4. Keep hop limit `ttl = 1` and per-pair token bucket defaults from ADR-0007
   (10 Mbps sustained, 2 MB burst, `relay_max_bps` in `easynet.toml`).
5. Relays must not decrypt inner `DATA`.

## Consequences

- Path selection checks **both** knobs; tests cover each alone failing.
- M3-00 cards must name both fields; do not collapse them.

## Alternatives considered

- **Single flag only** — either admins cannot pin relay-off, or nodes cannot
  opt out on battery; rejected.
