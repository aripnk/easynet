# ADR-0001: Record architecture decisions

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

Agents start each session without memory, so decisions that live only in
chat get re-decided or silently reversed. EasyNet has costly choices
(trust model, socket mux, VIP derivation) that must survive handoffs.

## Decision

- Record costly decisions in `docs/adr/NNNN-kebab-title.md` using the
  template, and index them in `docs/adr/README.md`.
- Write an ADR when a choice affects the data model, domain invariants,
  public/wire API, dependencies, deployment, or security, or when the
  owner overrides the spec.
- ADRs are immutable once Accepted except for status. To change a
  decision, write a new ADR that supersedes the old one.
- `docs/spec.md` describes current behavior and links the ADR. Update the
  spec in the same change.
- Agents write **Proposed**. Only the owner marks **Accepted**.

## Consequences

- Session contracts point agents at the ADR index instead of rediscovering
  rationale in `docs/plan.md`.
- Disagreement between card, spec, and ADR → stop and ask.

## Alternatives considered

- **README-only decisions** — rot and contradict silently.
- **Chat history as source of truth** — not available to the next agent.
