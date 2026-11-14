# ADR-0006: Offline root CA and delegated AdminCerts

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

If every online admin holds the mesh root, compromise forces a full mesh
rebuild. Owner locked Q3–Q5 (storage, lifetimes, canonical signing) on
2026-11-14.

## Decision

- **Offline root:** `easynet init` creates a root Ed25519 keypair.
  `root.key` is encrypted at rest with **Argon2id + XChaCha20-Poly1305**
  and a user passphrase, and is exportable as a paper QR backup so it
  can be removed from online hosts.
- **`mesh_id`:** first 8 bytes of `SHA256(root_pub_key)`.
- **Node identity key** at `/var/lib/easynet/node.key` (Unix) or
  `%ProgramData%\EasyNet\node.key` (Windows), mode `0600`, optionally
  sealed with the OS keychain via the `keyring` crate.
- **Delegated `AdminCert`:** signed by `root_pub_key` (pinned in genesis
  config). Fields and domain tag: `docs/api.md`. Permissions bitmask:
  `0x01 = APPROVE_PEER`, `0x02 = REVOKE_PEER`.
- A `PeerCert` is valid only if its `issuer_pub_key` has a valid,
  unexpired, unrevoked `AdminCert` chaining to `root_pub_key` (or is the
  root during documented bootstrap).
- **Lifetimes (Q4):** `PeerCert` default **90 days**; `AdminCert` default
  **365 days**. Auto-renewal: when remaining life &lt; 30 days or &lt; 1/3
  of TTL, member sends signed `RENEW_REQ` to an ALIVE admin; admin mints
  a new cert if not revoked, returns it, and gossips it. Expired offline
  members re-join via `join_request`.
- **Canonical signing (Q5):** never sign JSON. Sign
  `DomainTag (ASCII, NUL-terminated) || postcard(body)` with Ed25519.
  Tags and field order are frozen in `docs/api.md`.

## Consequences

- Compromised delegated admin can be revoked with the offline root
  without rotating every member identity.
- Agents must not invent JSON canonicalization for certs.
- Renewal and `RENEW_REQ` wire shape are M3 work; until then cards must
  not invent alternate lifetimes.

## Alternatives considered

- **Online root on every admin** — simpler ops, worse compromise story.
- **Sign JSON / protobuf with ad-hoc field order** — verification bugs;
  rejected in favor of postcard + domain tags.
