# ADR-0017: just check rejects paths outside the active card

- **Status:** Accepted
- **Date:** 2026-11-14
- **Amends:** the `just check` recipe in [ADR-0011](0011-dev-toolchain-and-nat-lab.md). Fmt, clippy, and nextest stay.

## Context

The session contract told agents to touch only the card's **Files** list. Nothing failed the session when they did not. Owner locked a mechanical gate on 2026-11-14, before M0-02.

## Decision

1. `scripts/check-card-scope.sh` reads the card id from the **Next task** section of `docs/progress.md` as stored in `HEAD` (the worktree copy cannot retarget the card). It loads that card's `**Files:**` paths from `docs/tasks/`, plus `docs/progress.md`.
2. A dirty path that is not an exact allowlist entry, and not under an allowlist entry that ends in `/`, is a failure. The script prints `SCOPE VIOLATION` and exits 1. A clean match prints `card-scope-ok`.
3. `just check` runs `scope-check` first, then fmt, clippy, and nextest. Every card also runs `just scope-check` before it is done, including M0-02, which does not yet run the Rust steps.
4. CI on a pull request runs `scripts/check-card-scope.sh --range <base>...HEAD`. Each commit is checked against the card in its parent. A commit whose parent does not contain the script is skipped, so the commit that introduces the gate is not judged by it.

## Consequences

- M0-03 must not delete `scope-check` or drop it from `check`.
- An agent who needs a file the card does not list stops and records **Blocked**. They do not add the path in order to pass.

## Alternatives considered

- **Honor-system allowlist only** — agents edit extra files and still finish; rejected.
- **Read the card id from the worktree progress file** — editing **Next task** would switch to a wider card in the same session; rejected.
