# EasyNet docs

Index for humans and agents. The session contract is [`../AGENTS.md`](../AGENTS.md).
Start every coding session at [`progress.md`](progress.md) (**Next task** / **Blocked**).

## Source of truth

| Doc | Purpose |
|---|---|
| [`spec.md`](spec.md) | What the system does (invariants, stack, **open questions**) |
| [`api.md`](api.md) | Wire + admin HTTP shapes |
| [`design.md`](design.md) | Admin portal UX |
| [`dev.md`](dev.md) | Toolchain, `just check`, NAT lab |
| [`glossary.md`](glossary.md) | Terms |
| [`adr/`](adr/README.md) | Why (decisions) |
| [`roadmap.md`](roadmap.md) | Milestones |
| [`tasks/`](tasks/README.md) | Executable cards |
| [`progress.md`](progress.md) | Handoff |
| [`conventions/`](conventions/) | Per-layer coding rules |
| [`plan.md`](plan.md) | Historical brainstorm only — **not** authoritative |

Copy strings: [`../uicopy/en.json`](../uicopy/en.json).

## Open questions (quick)

Q1–Q23 are resolved or deferred — [`spec.md` §8](spec.md#8-open-questions). The only remaining shape is three GET JSON bodies, and only `M4-00` may write them, from design §5 nouns.

| ID | Status | Summary |
|---|---|---|
| **Q15** | **Deferred** | Mesh DNS out of M4; optional M5 nice-to-have ([api.md §10](api.md#10-deferred-q15-mesh-dns)) |
| Q1–Q14, Q16–Q24 | Resolved | See spec §8. Q22's three GET bodies are written by M4-00 |
