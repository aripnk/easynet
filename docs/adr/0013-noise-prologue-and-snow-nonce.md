# ADR-0013: Noise prologue and snow stateless nonces

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

ADR-0010 froze the `0x05` byte layout and named `snow`, but not the prologue string or who builds the ChaCha nonce. Two implementations with different prologues cannot handshake. Owner locked this on 2026-11-14. ADR-0010's offsets stay in force.

## Decision

- Pattern remains `Noise_IK_25519_ChaChaPoly_BLAKE2s`.
- Both sides call `snow::Builder::new(...).prologue(b"easynet.noise-ik.v1")` before the handshake. The prologue is those ASCII bytes and nothing else.
- After completion, call `into_stateless_transport_mode()`. Do not build a 12-byte ChaCha nonce by hand.
- Send: increment a per-session `u64` `tx_counter` starting at `0`, store `tx_counter.to_le_bytes()` at `0x03` header bytes `5..13`, and call `write_message(tx_counter, plaintext, out)`.
- Receive: read `rx_counter` from bytes `5..13`, accept it only inside a 64-packet sliding anti-replay window, then `read_message(rx_counter, ciphertext, out)`.

Wire text: [api.md §5](../api.md#5-type-0x03-data) and [§5b](../api.md#5b-type-0x05-handshake).

## Consequences

- M2 cards copy this prologue and these calls. They do not add a second nonce scheme.
- A handshake with an empty prologue must fail a unit test.

## Alternatives considered

- **Empty prologue** — interoperates only by accident; rejected.
- **Manual 12-byte nonce next to snow's nonce** — two counters drift; rejected.
