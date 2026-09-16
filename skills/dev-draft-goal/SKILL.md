---
name: dev-draft-goal
description: >-
  Use only when the user explicitly invokes `dev-draft-goal`. Create or revise
  the durable, non-technical goal for a software-development task.
---

# Draft Development Goal

Create the task dir and keep `goal.md` as the product-level contract.

## Procedure

1. Read the request, workspace instructions,
   [workflow.md](../../protocol/workflow.md), and
   [waif-helper.md](../../protocol/waif-helper.md).
2. Run `waif inspect` to discover the current goal-stage context. Resolve the
   task using the deterministic commands:
   - when the user is clearly continuing an existing task that is not current,
     run `waif switch <TASK_NAME>`;
   - when starting a distinct task, run `waif new <TASK_SLUG>`;
   - do not reproduce these operations with ad hoc filesystem work.
3. Draft or revise `goal.md` from the latest user input using the
   [`goal.md` Contract](#goalmd-contract).
4. Keep the goal product-level: outcome, scope, and stable acceptance
   criteria, with no implementation choices.
5. Keep `Status: drafting` before initial acceptance. When changing an
   accepted goal, set `Status: amending`, maintain one revision subsection for
   the current cycle, preserve history, and identify downstream impacts.
6. Use `__NOW__` for current timestamps, then run
   `waif fill <GOAL_PATH>` and `waif check <GOAL_PATH>`. Resolve `BAD` and
   fill `INCOMPLETE` before presentation. The AI owns semantic review.
7. Show the `goal.md` path and ask the user to review it.
8. After explicit human acceptance and with no open questions, run
   `waif goal accept`. Treat unqualified `accept`, `approve`, or equivalent as
   acceptance of the current goal.
9. Do not modify or commit workspace files.

## `goal.md` Contract

```markdown
# Goal: <short title>

- Status: drafting
- Created: YYYY-MM-DDTHH:MM:SS+HH:MM
- Updated: YYYY-MM-DDTHH:MM:SS+HH:MM

## Outcome

<What should be true for the user or system when the task succeeds.>

## Acceptance Criteria

- G-AC1: <Observable, technology-independent result>

## Open Questions

- G-Q1: <Major issue, why it matters, and options the human can adopt>

## Assumptions

- G-A1: <Minor issue and the best-effort solution adopted>

## In Scope

- G-IN1: <Included behavior>

## Out of Scope

- G-OUT1: <Explicit exclusion>

## Revisions

### G-REV1: YYYY-MM-DDTHH:MM:SS+HH:MM - <amendment title>

- Before: <What the accepted goal said before this amendment.>
- Changed:
  - <Change made during this amendment cycle.>
```

`Outcome` and `Acceptance Criteria` are required. Omit other empty sections.
Preserve IDs and never reuse retired IDs.

`waif check` handles structural validation and `waif fill` expands supported
macros. The AI agent owns semantic review: keep `In Scope` substantive and put
exclusions and non-goals in `Out of Scope`.

Do not accept a goal with blocking `Open Questions`. Follow shared amendment
and revision rules.

## Guardrails

- Preserve useful existing content, active task dirs, and downstream history.
- Ask only questions that materially change outcome or scope.
- Do not add architecture, libraries, file paths, APIs, sequencing, or other
  technical planning. That belongs to `dev-draft-plan`.

## Done When

- The workflow dir, task dir, and `current` symlink are valid.
- `goal.md` matches the contract and latest user direction.
- Every goal content item has a stable ID.
- Accepted goals have no unresolved blocking questions.
