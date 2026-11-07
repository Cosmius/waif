---
name: dev-plan-step
description: >-
  Plan one implementation step for an accepted WAIF plan. Use only when
  explicitly invoked, the user requests WAIF, or work continues an existing
  WAIF task. Charter work alone does not count as task context.
---

# Plan Development Step

Consume one coherent part of the general plan and produce an executable
`step.md`.

## Procedure

1. Read [workflow.md](../../protocol/workflow.md), resolve the invoking
   workspace, local library, and current task, and stop if required context is
   missing or unusable.
2. Require `goal.md` and `plan.md` to exist with `Status: accepted`.
   Otherwise, ask the user to revise and accept the upstream artifact first.
   Do not draft, revise, or accept upstream artifacts. Verify the workspace is
   on the branch recorded in `plan.md`.
3. Inspect existing `steps/*/step.md` files and choose the next coherent
   unimplemented scope, honoring any explicit user choice. A step may consume
   all or part of one or more plan items. Do not force one-to-one boundaries.
4. Create `steps/NN-short-name/step.md`, using one plus the greatest existing
   sequence under `steps/` with at least two digits, and follow the
   [`step.md` Contract](#stepmd-contract).
5. Inspect relevant code deeply enough to name key decisions, affected files,
   and validation. Estimate changed non-test/doc lines. Keep the step suitable for
   one source commit and roughly 200 such lines or fewer. If not practical,
   explain why in `Size and Coherence`. Do not modify source code.
6. Create or revise the initial step only while `Status: drafting`. An accepted
   step may enter `amending` and be revised under the amendment-cycle rules.
   Do not revise a done step unless the human has reopened it under the
   shared status rules.
7. Show the step plan and wait for review.
8. On explicit acceptance, verify:
   - plan-item and goal-criterion IDs exist;
   - coverage is exact;
   - no completed plan item receives new work;
   - done conditions match declared coverage;
   - size and coherence rules are satisfied or justified.
   Then run `waif step accept`. Treat unqualified `accept`, `approve`, or
   equivalent as acceptance of the current step. The helper rechecks the
   mechanical preconditions and changes only the step's `Status` and
   `Updated`; plan-item statuses stay unchanged. Follow the
   [helper guidance](../../protocol/waif-helper.md#commands) for repair.

## `step.md` Contract

Use `steps/NN-short-name`, where `NN` is a monotonically increasing,
minimum-two-digit sequence. Choose one plus the greatest number previously
used. Never reuse or backfill a number. Use the same number without padding in
the step title and item-ID prefix.

Follow the [`step.md` template](../../protocol/template/step.md).

Statuses are `drafting`, `accepted`, `amending`, `done`, and `aborted`.
`Source commit` remains `not-created` through drafting, acceptance, and
amendment; in the normal lifecycle, only a successfully committed
implementation changes it to a full commit hash. An `aborted` step keeps
whatever `Source commit` it had.
- `Plan items` is a comma-separated exact-ID list, for example
  `Plan items: P2, P3`.
- `Goal criteria` is the union advanced through those plan items.
- `Source commit` must be `not-created` while the step is `drafting`,
  `accepted`, or `amending`. In the normal lifecycle, `waif step commit`, run
  by `dev-implement-step`, replaces it with the full commit hash after the
  workspace commit succeeds.
- `Plan Item Coverage` must contain one entry for every listed plan ID and no
  others, naming it in `Plan item` and setting `Coverage` to `partial` or
  `complete`. Use `complete` instead of `partial` when this step is expected
  to finish the remaining outcome of the plan item.

Omit empty optional sections. While a step is `amending`, use expanded
`Revisions` records and the shared amendment-cycle rules. In the normal
lifecycle, a step becomes `done` only after implementation is accepted and the
workspace commit succeeds; a human-requested close follows the shared status
rules.
Plan items may remain `pending` across completed steps. Prefix item IDs with
the unpadded step number, such as `S1-C1` for step directory `01-short-name`;
preserve them while drafting and never reuse them. Follow the shared item-ID
and ambiguity rules.

## Guardrails

- Create only one step per invocation and keep implementation work out.
- Map by exact plan-item IDs, never fuzzy heading text, or invented child IDs.
- Multiple steps may advance one plan item only when their coverage is distinct
  or intentionally cumulative.
- Do not revise a done step plan unless the human has reopened it.

## Done When

- The step is one commit, roughly 200 changed non-test/doc lines or fewer, or
  explains why a smaller coherent scope is impractical.
- Files, behavior, edge cases, validation, and coverage are concrete and exact.
- Every step content item has a stable, step-qualified ID.
- Acceptance is reflected in `step.md`; `goal.md` and `plan.md` remain
  accepted.
