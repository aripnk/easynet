# M2 · Noise encryption and TUN data plane

Open this file only when `docs/progress.md` **Next task** is `M2-00`.
Do not implement from this card. It is a planning card.

---

### M2-00 · Write M2 task cards (planning)
- **Status:** todo
- **Goal:** Owner-reviewed execution cards that implement the locked Noise prologue, snow nonces, and TUN path without reopening them.
- **Read first:** `docs/spec.md` §6 and §7; `docs/api.md` §5 and §5b; `docs/adr/0010-noise-handshake-mux.md`; `docs/adr/0013-noise-prologue-and-snow-nonce.md`; `docs/conventions/crypto.md`; `docs/conventions/tun.md`; `docs/tasks/README.md` §Planning tasks.
- **Files:** + `docs/tasks/M2-noise.md` (execution cards below this one); ~ `docs/roadmap.md`; ~ `docs/progress.md`
- **Steps:**
  1. If M1 is incomplete, write that under **Blocked** and stop.
  2. Locked, do not reopen:
     - Prologue bytes `b"easynet.noise-ik.v1"` on both sides.
     - `into_stateless_transport_mode`, then `write_message` / `read_message` with the `u64` counter. No hand-built 12-byte nonce.
     - 64-packet anti-replay window. Header bytes `5..13` are that counter, little-endian.
     - `HANDSHAKE_INIT` is 115 bytes and `HANDSHAKE_RESP` is 59 bytes (api §5b).
  3. Write execution cards for handshake, DATA, and `easynet0` / `utun` at MTU 1380 with the ADR-0008 VIP. Golden tests include a wrong prologue, which must fail.
  4. Keep **Next task** as `M2-00` until the owner accepts the new cards.
- **Acceptance:**
  - Owner review: every execution card has an allowlist and a command; none says "choose a nonce" or names a prologue other than `easynet.noise-ik.v1`.
- **Out of scope:** Implementing the handshake; SWIM; the admin portal.
