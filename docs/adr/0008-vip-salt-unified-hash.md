# ADR-0008: Unified VIP hash always includes vip_salt

- **Status:** Accepted. Loopback "proxy or 302" wording inherited from ADR-0004 §3 is amended by [ADR-0015](0015-admin-reverse-proxy-and-auth.md) (reverse proxy only). VIP hash and who-may-bind rules are unchanged.
- **Date:** 2026-11-14
- **Supersedes:** [ADR-0004](0004-deterministic-virtual-ip.md) (VIP derivation formula and `EndpointRecord` field order only; portal bind rules from 0004 remain in force)

## Context

ADR-0004 omitted the salt byte when `vip_salt == 0`, creating two code paths and
easy implementer bugs. Owner accepted a single formula that always appends the
salt byte (2026-11-14).

## Decision

1. **One hash path for all salts:**

   ```text
   H = SHA256("easynet-vip-v1" || ed25519_pub || [vip_salt])
   offset = u32_be(H[0..4]) & 0x003F_FFFF
   if offset == 0x0000_0000 { offset = 0x0000_0001 }
   if offset == 0x003F_FFFF { offset = 0x003F_FFFE }
   VIP = IPv4(100.64.0.0) + offset
   ```

   `vip_salt` is always one byte in the preimage (`0` = primary, `1..=255` =
   collision fallback). Never omit the byte when salt is 0.

2. **Collision policy** unchanged from ADR-0004: earlier `issued_at` wins
   (tie-break lower `ed25519_pub`); loser increments `vip_salt` and gossips.

3. **Canonical `EndpointRecord` body order** (domain `easynet.endpoint.v1\0`):
   `version`, `mesh_id`, `peer_ed25519_pub`, `incarnation`, `vip_salt`,
   `endpoints`, `relay_capable`. Signature follows the signed body on the wire.

4. Portal addressing / no-master-IP rules from ADR-0004 §3 remain **Accepted**, except the loopback hop, which ADR-0015 fixes as a reverse proxy rather than a 302.

## Consequences

- Golden test vectors must use salt `0` with the extra `0x00` byte in the hash.
- Agents must not reintroduce the “omit salt when zero” shortcut.
- Update `docs/api.md` and `docs/spec.md` in the same change as any code.

## Alternatives considered

- **Omit salt byte when zero** — two paths; rejected.
