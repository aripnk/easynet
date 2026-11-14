# ADR-0003: Single UDP socket multiplexing and Noise IK data plane

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

Cone-NAT mappings are per 5-tuple. Multiple sockets for control vs data
often get different external ports and break hole punching. Data must be
confidential end-to-end even when relayed. Spec §4, wire doc `api.md`.
Owner locked Rust/`snow` in ADR-0005.

## Decision

- Bind **one** primary UDP socket (default port `51820`) for PUNCH,
  GOSSIP, DATA, RELAY, and STUN demux on that socket.
- Temporary extra UDP sockets are allowed **only** for birthday-paradox
  symmetric-NAT probing; they must not carry steady-state GOSSIP/DATA mux.
- Encrypt TUN payloads with **Noise IK** via `snow`
  (`Noise_IK_25519_ChaChaPoly_BLAKE2s`) using
  **`StatelessTransportState`** (explicit nonces/counters over UDP).
- Wire and signed payloads use **`postcard`** codecs ([ADR-0006](0006-offline-root-and-admin-certs.md)).
- `RELAY` forwards opaque `0x03 DATA` frames; relays do not decrypt.
  Relay policy: [ADR-0007](0007-relay-policy.md).

## Consequences

- Demux must distinguish STUN (RFC 5389 cookie) from mesh type bytes.
- MTU 1380 accounts for outer IP/UDP/Noise/mux overhead.
- Handshake message embedding remains an open detail for `M2-00` only
  insofar as the exact Noise message framing bytes on the wire; the
  pattern and AEAD are fixed here.

## Alternatives considered

- **Separate sockets per subprotocol** — breaks consistent NAT mapping.
- **WireGuard kernel module as the only path** — less portable control
  over custom gossip/certs; may revisit later via ADR supersede.
- **TLS-over-TCP relays** — different NAT behavior; not chosen for v1.
