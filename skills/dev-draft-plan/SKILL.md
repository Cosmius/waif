---
name: dev-draft-plan
description: >-
  Draft or revise the technical plan for an accepted WAIF goal. Use only when
  explicitly invoked, the user requests WAIF, or work continues an existing
  WAIF task. Charter work alone does not count as task context.
---

# Draft Development Plan

Turn the accepted goal into a reviewable general plan.

## Procedure

1. Read [workflow.md](../../protocol/workflow.md), resolve the invoking
   workspace, local library, and current task, and stop if required context is
   missing or unusable.
2. Require `goal.md` to exist with `Status: accepted`.
3. Inspect workspace instructions, relevant code, and test commands. Plan from
   current code, not assumptions.
4. When creating `plan.md`, require a Git repository, a resolvable `HEAD`, and
   a checked-out branch. Stop on detached `HEAD` and tell the user to check out
   a base branch. Record the current branch as immutable `Base branch`, the
   full current `HEAD` as immutable `Base commit`, and the intended task branch
   as `Branch`.
5. Create or revise `plan.md` using the [`plan.md` Contract](#planmd-contract).
6. Keep plan items broad enough to require later `step.md` files. New items
   need stable `P<number>` IDs, goal-criteria coverage, and `pending` status;
   preserve existing IDs and statuses.
7. Keep the plan `drafting` until accepted. Apply the shared
   [amendment rules](../../protocol/workflow.md#revision-rules) to accepted
   plans; judge metadata materiality and preserve completed history.
8. Use `__NOW__` for current timestamps, then run
   `waif fill <PLAN_PATH>` and `waif check <PLAN_PATH>`. Resolve `BAD` and
   fill `INCOMPLETE` before presentation. Run `waif inspect` from the workspace
   root. The AI owns semantic review, including coverage, dependencies,
   feasibility, step impacts, and untracked-file assessment.
9. Show the `plan.md` path and ask the user to review it.
10. After explicit human acceptance and with every question `settled`, run
    `waif plan accept`. Treat unqualified `accept`, `approve`, or equivalent as
    acceptance of the current plan. Follow the
    [helper guidance](../../protocol/waif-helper.md#commands) for repair.

## `plan.md` Contract

Follow the [`plan.md` template](../../protocol/template/plan.md).

Statuses are:

- plan: `drafting`, `accepted`, `amending`, `done`, `aborted`;
- plan item: `pending`, `done`.

- Omit empty `Questions`, `Assumptions`, and pre-amendment `Revisions`.
- Preserve IDs. Append new IDs and mark removed scope explicitly.
- Each plan item must list every `G-AC<number>` it advances.
- Plan items must collectively cover every accepted goal criterion.
- Decisions record durable technical choices that later steps should follow.
- Decisions use `P-D<number>`. A decision that needs its own section follows
  the `- P-D<number>` decisions as `### P-D<number>`, with a `Vide infra`
  placeholder at its number among them.
- Use `P-REV<number>` for expanded plan revision records.

Follow the shared item-ID, ambiguity, amendment, and revision rules.

Keep base provenance immutable. Use the user's task branch name or derive
one from the task dir; retain its identity after creation. Delegate branch
setup and restoration to `waif plan accept`.

## Guardrails

- Do not change workspace source code or recreate product requirements.
- Name concrete components, interfaces, migrations, and validation commands
  when discoverable.
- Keep durable technical choices in `Decisions`, not plan item prose.
- Leave detailed edits and command sequences to steps.
- Never carry unrelated dirty workspace changes onto the new branch without
  explicit user direction.
- Never reset plan or step status to hide prior work.

## Done When

- `plan.md` is grounded in current code and maps the whole accepted goal.
- Every plan item has a unique ID and goal-criteria coverage; new items are
  `pending` and existing statuses are preserved.
- Every plan content item has a stable ID.
- Every plan records immutable `Base branch` and `Base commit` provenance.
- Accepted plans record and use the workspace task branch.
