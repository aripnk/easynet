# M3 · SWIM gossip, certs, and relay

Open this file only when `docs/progress.md` **Next task** is `M3-00`.
Do not implement from this card. It is a planning card.

---

### M3-00 · Write M3 task cards (planning)
- **Status:** todo
- **Goal:** Owner-reviewed execution cards for gossip, certs, and relay that use the locked timers and the locked relay inner types.
- **Read first:** `docs/spec.md` §6; `docs/api.md` §4 and §6; `docs/adr/0002-zero-server-admin-trust.md` (bootstrap exception); `docs/adr/0009-relay-two-knob-policy.md`; `docs/adr/0012-stun-demux-and-relay-handshake.md`; `docs/adr/0014-swim-failure-detector.md`; `docs/conventions/gossip.md`; `docs/tasks/README.md` §Planning tasks.
- **Files:** + execution cards in this file; ~ `docs/roadmap.md`; ~ `docs/progress.md`
- **Steps:**
  1. If M2 is incomplete, write that under **Blocked** and stop.
  2. Locked, do not reopen:
     - Gossip header is two bytes, then postcard. No outer `sender_node_id`.
     - `PROTOCOL_PERIOD` 1000 ms, `PING_TIMEOUT` 300 ms, `INDIRECT_K` 2, `SUSPECT_TIMEOUT` 5000 ms, at most 8 deltas.
     - `GossipDelta` variant order in api §4. `PendingJoin` expires after 15 minutes.
     - `RELAY` inner is `0x03` or `0x05` only. Inner `0x05` is ≤ 128 bytes and ≤ 2 packets/sec per pair. The byte bucket is for `0x03` only.
     - After a direct `PUNCH` ACK, `DATA` leaves the relay.
     - `BootstrapMode` ends when the first cert is stored (ADR-0002).
     - Punch replay cache stays 30 s / 4096 / `nonce` as `u64`.
  3. Write execution cards for membership, the failure detector, renewal, and the symmetric-NAT relay path in the lab. Tests must include suspect → dead, a higher incarnation refuting suspect, a 9th delta omitted, a 129-byte relayed handshake dropped, and bootstrap traffic rejected after the first cert.
  4. Keep **Next task** as `M3-00` until the owner accepts the new cards.
- **Acceptance:**
  - Owner review: no execution card picks a suspicion timeout or a relay inner type.
- **Out of scope:** Implementing gossip; the admin HTTP server; mesh DNS.
