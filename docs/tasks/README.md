# Task cards

Each milestone in [`../roadmap.md`](../roadmap.md) is split into task
cards: small, ordered units that one agent session can finish and
verify alone. Cards are written so a less capable model can execute
them without making design decisions.

| File | Milestone | Status |
|---|---|---|
| [`M0-bootstrap.md`](M0-bootstrap.md) | M0 · Repository bootstrap | done (M0-01..M0-04). Also holds `M1-00` (todo) |
| [`M2-noise.md`](M2-noise.md) | M2 · Noise and TUN | planning card only; open when Next task is `M2-00` |
| [`M3-gossip.md`](M3-gossip.md) | M3 · SWIM, certs, relay | planning card only; open when Next task is `M3-00` |
| [`M4-admin.md`](M4-admin.md) | M4 · Admin portal | planning card only; open when Next task is `M4-00` |

## How to execute a card

1. Pick the first card whose status is `todo` in the current milestone file. Do cards in order.
2. Read only the documents listed under **Read first**.
3. Follow **Steps**. Create or modify only the files listed under **Files**. If another file must change, stop and ask. The card's own `**Status:**` line is the step 5 exception; the scope gate allows it ([ADR-0018](../adr/0018-card-status-line.md)).
4. Run every command under **Acceptance**. All must pass.
5. Set the card's status to `done`, tick the matching roadmap checkbox, and add an entry to [`../progress.md`](../progress.md).
6. Commit with message `M0-02: <card title>` only when the user asked to commit.

## When to stop and ask

- An acceptance command fails and the fix is outside the card's files.
- The card conflicts with `docs/spec.md`, `docs/api.md`, or an ADR.
- A step requires a decision the card doesn't make.
- You would need to touch an invariant listed in `AGENTS.md`.

Write the question under **Blocked** in `docs/progress.md`, then stop.

## Planning tasks (`Mx-00`)

Before work starts on a milestone after M0, its card file must be written.
That is task `Mx-00`. It is a planning task: a strong model or the owner
writes it, and the owner reviews it before any `Mx-01` starts.
A planning task may also fill missing API shapes or spec sections.

## Card template

### M?-NN · Title
- **Status:** todo | done | blocked
- **Goal:** one sentence.
- **Read first:** exact doc sections.
- **Files:** every file to create (+) or modify (~).
- **Steps:** numbered, concrete. Include signatures and sketches when the card touches code.
- **Acceptance:** shell commands and the expected result.
- **Out of scope:** what not to do in this card.
