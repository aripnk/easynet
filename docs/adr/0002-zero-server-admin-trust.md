# ADR-0002: Zero-server membership via admin-signed certificates

- **Status:** Accepted. Bootstrap `DATA` exception below added by owner lock 2026-11-14. Earlier bullets unchanged.
- **Date:** 2026-11-14

## Context

EasyNet must admit devices without a hosted coordination service and
without requiring a quorum of online peers (mobiles sleep). Spec §1–§2,
§5. Alternatives like Raft membership or a central IdP conflict with
zero-subscription / zero-control-plane goals.

## Decision

- Trust is anchored by a **network root** Ed25519 keypair created at mesh
  genesis (preferably kept offline — [ADR-0006](0006-offline-root-and-admin-certs.md)).
- Membership is granted by signed **`PeerCert`** records and removed by
  signed **`RevokeCert`** tombstones. Any single awake admin that holds a
  valid **`AdminCert`** may issue them (permissions bitmask).
- Gossip disseminates certs and revocations; **revocation always wins**
  over a cert for the same peer key.
- Do not introduce a project-operated signaling or membership server.
- URI scheme for out-of-band tokens is **`easynet://`** (Q8).

### Bootstrap `DATA` exception

During initial network creation (`easynet init` ↔ `easynet pair`), the two initial bootstrap peers (Node A and Node B) exchange `invite_token` payloads anchored to the same `root_pub_key` before either peer possesses a signed `PeerCert`. While in `BootstrapMode`, Node A and Node B accept `0x05 HANDSHAKE` and `0x03 DATA` frames exclusively from the single `peer_ed25519_pub` / `peer_x25519_pub` imported via that `invite_token`, and drop frames from any other key. This exception terminates permanently on a node the moment it writes its first valid `AdminCert` or `PeerCert` to disk. Once terminated, `BootstrapMode` can never be re-enabled without wiping node state (`easynet reset`).

## Consequences

- Lost root/admin keys are a human recovery problem (paper QR + passphrase).
- Tests must cover: unsigned membership gossip ignored; revoked peer
  `DATA` dropped; dual-admin webhook dedupe (spec invariant 9);
  `BootstrapMode` accepts handshake and `DATA` only from the invited key,
  and refuses that path after the first cert is stored.
- Mobile-unfriendly consensus protocols stay out of scope.

## Alternatives considered

- **Raft/quorum membership** — fails when most nodes sleep.
- **Central coordination / Tailscale-like control plane** — violates
  zero-server product goal.
- **TOFU without certs** — weak revocation and no admin role model.
