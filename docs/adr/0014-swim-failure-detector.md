# ADR-0014: SWIM wire layout and failure-detector timers

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

Gossip subtypes were named ("SWIM ping + piggybacked deltas") without fields or timers, while the spec said no question was still open. Owner locked the layout and the failure detector on 2026-11-14. Cert domain tags and the two-knob relay rule are unchanged.

## Decision

- A `0x02` frame is two bytes (`0x02`, subtype) plus the postcard payload in [api.md §4](../api.md#4-type-0x02-gossip). The old 20-byte header (reserved + outer `sender_node_id`) is withdrawn.
- Timers: protocol period **1000 ms**, direct ping timeout **300 ms**, indirect fanout **K = 2**, suspect-to-dead **5000 ms** unless refuted by `incarnation + 1`. At most **8** piggybacked `GossipDelta` values, lowest broadcast count first.
- `PING`, `ACK`, and `PING_REQ` field lists, the `GossipDelta` variant order, and the 15-minute `PendingJoin` lifetime are in that same section. `RENEW_REQ` / `RENEW_RESP` bodies stay the Q12 records.
- `PUNCH` replay cache: `(sender_node_id, nonce u64)` for **30 s**, LRU **4096**, and reject `|unix_ms - now| > 30 s`.

## Consequences

- M3 cards use these constants. They do not pick a suspicion timeout.
- Tests cover suspect → dead at 5 s, refutation by a higher incarnation, and a 9th delta left off the packet.

## Alternatives considered

- **Leave SWIM as a library's default timers** — agents would disagree; rejected.
- **Keep the 20-byte gossip header and also put `sender_id` in the body** — two sender fields; rejected.
