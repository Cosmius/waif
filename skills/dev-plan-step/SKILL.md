---
name: dev-plan-step
description: >-
  Use only when the user explicitly invokes `dev-plan-step`. Create or revise
  one drafting step plan for the current accepted general plan.
---

# Plan Development Step

Consume one coherent part of the general plan and produce an executable
`step.md`.

## Procedure

1. Read [workflow.md](../../protocol/workflow.md), resolve the established
   workflow dir and current task dir, and stop if either is missing.
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
5. Inspect relevant code deeply enough to name concrete changes and
   validation. Estimate changed non-test/doc lines. Keep the step suitable for
   one source commit and roughly 200 such lines or fewer. If not practical,
   explain why in `Size and Coherence`. Do not modify source code.
6. Create or revise the initial step only while `Status: drafting`. An accepted
   step may enter `amending` and be revised under the amendment-cycle rules.
   Never revise a done step.
7. Show the step plan and wait for review.
8. On explicit acceptance, verify:
   - plan-item and goal-criterion IDs exist;
   - coverage is exact;
   - no completed plan item receives new work;
   - done conditions match declared coverage;
   - size and coherence rules are satisfied or justified.
   Set step `Status: accepted` and leave plan-item statuses unchanged.

## `step.md` Contract

Use `steps/NN-short-name`, where `NN` is a monotonically increasing,
minimum-two-digit sequence. Choose one plus the greatest number previously
used. Never reuse or backfill a number.

```markdown
# Step NN: <title>

- Status: drafting
- Source commit: not-created
- Created: YYYY-MM-DDTHH:MM:SS+HH:MM
- Updated: YYYY-MM-DDTHH:MM:SS+HH:MM
- Plan items: P1
- Goal criteria: G-AC1
- Estimated non-test/doc changes: ~150 lines

## Objective

<The technical result of this step.>

## Plan Item Coverage

- S1-PC1: P1 - partial - <the exact part consumed by this step>

Use `complete` instead of `partial` when this step is expected to finish the
remaining outcome of the plan item.

## Context

<Relevant current behavior and constraints discovered from the workspace.>

## Open Questions

- S1-Q1: <Major issue, why it matters, and options the human can adopt>

## Assumptions

- S1-A1: <Minor issue and the best-effort solution adopted>

## Done When

- S1-D1: <Observable implementation result>
- S1-D2: <Required validation passes>

## Changes

- S1-C1: <Concrete change, naming paths and interfaces>
- S1-C2: <Concrete change>

## Size and Coherence

<Why this scope is coherent as one commit. If the estimate exceeds roughly 200
changed non-test and non-documentation lines, explain why splitting it further
would make the step impractical or incoherent.>

## Tests

- S1-T1: <Test to add or update>

## Validation

- S1-V1: `<focused command>`
- S1-V2: `<broader regression command when needed>`

## Risks and Edge Cases

- S1-R1: <Risk or edge case and handling>

## Revisions

### S1-REV1: YYYY-MM-DDTHH:MM:SS+HH:MM - <revision title>

- Before: <accepted content before the amendment>
- Changed:
  - <change included in this amendment cycle>
```

Statuses are `drafting`, `accepted`, `amending`, and `done`. `Source commit`
remains `not-created` through drafting, acceptance, and amendment; only a
successfully committed implementation changes it to a full commit hash.
- `Plan items` is a comma-separated exact-ID list, for example
  `Plan items: P2, P3`.
- `Goal criteria` is the union advanced through those plan items.
- `Source commit` must be `not-created` while the step is `drafting`,
  `accepted`, or `amending`. Only `dev-implement-step` may replace it, and only
  with the full commit hash after the workspace commit succeeds.
- `Plan Item Coverage` must contain one `partial` or `complete` entry for every
  listed plan ID and no others.

Omit empty optional sections. While a step is `amending`, use expanded
`Revisions` records and the shared amendment-cycle rules. Do not mark a step
`done` until implementation is accepted and the workspace commit succeeds.
Plan items may remain `pending` across completed steps. Prefix item IDs with
the unpadded step number, such as `S1-C1` for step directory `01-short-name`;
preserve them while drafting and never reuse them. Follow the shared item-ID
and ambiguity rules.

## Guardrails

- Create only one step per invocation and keep implementation work out.
- Map by exact plan-item IDs, never fuzzy heading text, or invented child IDs.
- Multiple steps may advance one plan item only when their coverage is distinct
  or intentionally cumulative.
- Never revise a done step plan.

## Done When

- The step is one commit, roughly 200 changed non-test/doc lines or fewer, or
  explains why a smaller coherent scope is impractical.
- Files, behavior, edge cases, validation, and coverage are concrete and exact.
- Every step content item has a stable, step-qualified ID.
- Acceptance is reflected in `step.md`; `goal.md` and `plan.md` remain
  accepted.
