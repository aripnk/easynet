# Crypto / identity (easynet-core)

Read this when editing `crates/easynet-core` (`identity.rs`) or Noise session
code in `easynet-net`.

- Noise via `snow` (`Noise_IK_25519_ChaChaPoly_BLAKE2s`). Prologue bytes are
  `b"easynet.noise-ik.v1"`. Nonces go through `StatelessTransportState`
  (`write_message` / `read_message` with the `u64` counter). Do not build a
  12-byte ChaCha nonce. Sign `DomainTag || postcard(body)` only — the
  signature is appended, never a struct field. Relays never decrypt.
  Link: [ADR-0013](../adr/0013-noise-prologue-and-snow-nonce.md),
  [api.md](../api.md) §5.

```rust
// BAD: ed25519_sign(serde_json::to_string(&cert).unwrap().as_bytes())
```

```rust
// GOOD: let mut msg = b"easynet.peer-cert.v1\0".to_vec();
//       msg.extend(postcard::to_stdvec(&body)?);
//       signing_key.sign(&msg)
```
