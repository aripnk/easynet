# ADR-0018: Card status line is inside the scope gate

- **Status:** Accepted. Owner lock 2026-11-16.
- **Date:** 2026-11-16
- **Amends:** item 2 of [ADR-0017](0017-card-scope-gate.md). The rest of that ADR stays in force.

## Context

The definition of done says to mark the card `done`. ADR-0017 fails any dirty path that is not on the card's `**Files:**` line. The task file is not on that line, and adding it there is how an agent would silence the gate. M0-02, M0-03, and M0-04 were left `todo` for that reason while their roadmap boxes were ticked. The two docs disagreed.

## Decision

1. `scripts/check-card-scope.sh` still reads the card id from the committed **Next task** and still fails a dirty path outside that card's `**Files:**` list and `docs/progress.md`.
2. A dirty `docs/tasks/*.md` file also passes when the diff against the baseline changes only lines of the form `- **Status:** todo`, `- **Status:** done`, or `- **Status:** blocked`, and every changed card is either the active card or a card already marked `[x]` on `docs/roadmap.md` in the tree under check. The active card may become `done` or `blocked`. A roadmap-ticked card may become `done`.
3. Goal, **Files**, Steps, Acceptance, and any other line in that task file still fail the gate. Do not add the task file to `**Files:**` to pass.
4. After acceptance passes, the session sets that card's status to `done` in the same change as the roadmap tick and the progress entry.
5. The commit that introduces this ADR is the one whose parent does not contain this file. That commit may also change `scripts/check-card-scope.sh`, `AGENTS.md`, `docs/adr/0011-dev-toolchain-and-nat-lab.md` (status line only), `docs/adr/0017-card-scope-gate.md` (status line only), `docs/adr/README.md`, `docs/tasks/README.md`, `docs/spec.md`, and `docs/dev.md`. Those paths are not an allowlist once this file is in the parent.

## Consequences

- M0-02, M0-03, and M0-04 are `done` in `docs/tasks/M0-bootstrap.md`. M1-00 stays `todo`.
- `AGENTS.md`, `docs/tasks/README.md`, `docs/spec.md`, and `docs/dev.md` describe the same status-line exception.
- An agent still cannot rewrite another card's steps by editing the shared task file.

## Alternatives considered

- **Put every task file on every card's Files line** — the first edit still fails, because the allowlist is read from the parent commit. Rejected.
- **Allow the whole task file** — one file holds later cards, so a session could rewrite them. Rejected.
