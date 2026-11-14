# ADR-0004: Deterministic virtual IP across 100.64.0.0/10

- **Status:** Superseded by ADR-0008 (VIP hash / `EndpointRecord` order). Portal bind-who rules in §3 remain in force via ADR-0008 §4. The §3 phrase "proxies or 302s" is superseded by [ADR-0015](0015-admin-reverse-proxy-and-auth.md): loopback is a reverse proxy.
- **Date:** 2026-11-14

## Context

Peers need collision-resistant addresses without DHCP or a coordinator.
The first design used only `100.64.a.b` (~16 bits) and sometimes assumed
`100.64.0.1` was the admin portal. Owner locked Q2 and Q10 on 2026-11-14.

## Decision

1. **Keep identity-derived VIPs** (no central IP allocator). Preserve
   invariant 6: peers do not pick arbitrary VIPs.
2. **Hash into the full `/10` (22 host bits):**

   ```text
   H = SHA256("easynet-vip-v1" || ed25519_pub_key)
   offset = u32_be(H[0..4]) & 0x003F_FFFF
   if offset == 0x0000_0000 { offset = 0x0000_0001 }
   if offset == 0x003F_FFFF { offset = 0x003F_FFFE }
   VIP = IPv4(100.64.0.0) + offset
   ```

3. **Admin portal addressing (no master IP):** Never reserve `100.64.0.1`.
   There is no Raft leader VIP. Each **AdminCert** node binds
   `http://<its-own-VIP>:8080` (A and B are equivalent; gossip syncs
   pending joins and certs). **Member** VIPs must not bind `:8080`.
   **Every** node binds `http://127.0.0.1:8080` and proxies or 302s to an
   `ALIVE` admin (prefer lowest `node_id`). **Transport amended by
   ADR-0015:** that hop is a reverse proxy, not a 302. `easynet status` lists ALIVE
   admin URLs. Mesh DNS `admin.easynet` is **deferred** (Q15 → post-M4 /
   M5 nice-to-have; sketch in `docs/api.md` §10).
4. **Collision (Q10):** Authentication is by Ed25519 key, not VIP. If two
   distinct valid `PeerCert`s claim the same VIP, the cert with the
   earlier `issued_at` wins (tie-break: lower `ed25519_pub` bytes). The
   loser derives a secondary VIP:

   ```text
   H' = SHA256("easynet-vip-v1" || ed25519_pub_key || 0x01)
   ```

   then the same 22-bit offset/clamp rule. The peer sets
   `EndpointRecord.vip_salt` to that trailing byte (`0` = primary,
   `1` = first collision rehash, …) and gossips the record. Peers
   **must** derive VIP as
   `SHA256("easynet-vip-v1" || pub || [salt_byte_if_salt>0])`
   using the advertised `vip_salt` (omit the extra byte when salt is 0).

5. **`PeerCert` does not carry a VIP field** — verifiers derive VIP from
   the peer public key plus `vip_salt` from the latest signed
   `EndpointRecord`. See `docs/api.md` canonical structs.

## Consequences

- ~4.19M usable addresses; birthday collision risk is negligible at
  household/small-org scale; collision rule still required.
- UI/docs must never hard-code `.1`.
- Tests: golden VIP vectors for the domain-separated hash; collision
  winner/loser behavior.

## Alternatives considered

- **`100.64.a.b` only (16 bits)** — too small; rejected.
- **Admin-assigned VIP in cert** — breaks pure derive-and-verify; rejected.
- **Privileged `.1` admin** — breaks derivation and creates a false SPOF; rejected.
