# M4 · Admin portal

Open this file only when `docs/progress.md` **Next task** is `M4-00`.
Do not implement from this card. It is a planning card.
Mesh DNS (Q15) is out of scope for this file and for every M4 execution card.

---

### M4-00 · Write M4 task cards (planning)
- **Status:** todo
- **Goal:** Freeze the three remaining GET JSON objects, then write owner-reviewed execution cards that follow ADR-0015.
- **Read first:** `docs/api.md` §8; `docs/design.md` §1b, §3, §4, and §5; `docs/adr/0015-admin-reverse-proxy-and-auth.md`; `uicopy/en.json`; `docs/tasks/README.md` §Planning tasks.
- **Files:** ~ `docs/api.md` §8 (only the three GET bodies); + execution cards in this file; ~ `docs/roadmap.md`; ~ `docs/progress.md`
- **Steps:**
  1. If M3 is incomplete, write that under **Blocked** and stop.
  2. Locked, do not reopen: reverse proxy (not 302); cookie `easynet_sess`; Unix socket and Windows named pipe; 24-hour single-use bootstrap token at `POST /api/v1/auth/bootstrap`; challenge, verify, peers, pending, approve, reject, and revoke bodies in api §8; webhook `peer.join_requested` with `X-EasyNet-Signature`. UI strings already in `uicopy/en.json`. The Accept button uses key `pending.accept` and the path `approve`.
  3. Write JSON objects for `GET /api/v1/status`, `GET /api/v1/me/cert`, and `GET /api/v1/topology` into api §8. Keys may only be the nouns in design §5: mesh id, node id, VIP, role, alive count, peer count, `admin_urls`, own cert expiry, and direct-vs-relay edges. If a screen needs another key, stop and record it under **Blocked**. Do not start execution cards in that same turn.
  4. After those three objects are in api §8, write execution cards. Keep **Next task** as `M4-00` until the owner accepts them.
- **Acceptance:**
  - Owner review: api §8 has the three GET bodies; no execution card adds a field, a redirect, or a copy sentence.
- **Out of scope:** Implementing the portal; `admin.easynet`; mobile wrappers.
