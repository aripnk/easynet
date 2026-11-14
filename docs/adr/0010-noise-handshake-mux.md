# ADR-0010: Noise IK handshake as UDP type 0x05

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

ADR-0003 fixed Noise IK via `snow` but left UDP embedding open (Q11). Owner
locked the mux framing on 2026-11-14.

## Decision

- Add mux type **`0x05 HANDSHAKE`** for `Noise_IK_25519_ChaChaPoly_BLAKE2s`
  (`snow` / `StatelessTransportState` after handshake).
- IK is 1-RTT: initiator already knows responder static X25519 from
  `invite_token`, `roster_export`, or `PeerCert`.
- Exact byte layouts for `HANDSHAKE_INIT`, `HANDSHAKE_RESP`, and aligned
  `0x03 DATA` headers are frozen in [`docs/api.md`](../api.md) §§5–5b.
  Do not invent alternate offsets.

## Consequences

- Demux accepts `0x01`–`0x05`.
- M2 cards implement these layouts; golden packet tests required.
- Replay window on handshake timestamp: `|now - ts| <= 60s`.

## Alternatives considered

- **Embed handshake inside `0x03 DATA`** — harder demux/debugging; rejected.
- **XX pattern** — extra RTT; rejected (initiator already has static key).
