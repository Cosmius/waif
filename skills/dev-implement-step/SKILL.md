---
name: dev-implement-step
description: >-
  Implement and review an accepted WAIF step; commit human-staged changes. Use
  only when explicitly invoked, the user requests WAIF, or work continues an
  existing WAIF task. Charter work alone does not count as task context.
---

# Implement Development Step

Implement one accepted step through a review-and-acceptance loop.

## Procedure

1. Read canonical [workflow.md](../../protocol/workflow.md), resolve the
   invoking workspace, local library, and current task, and identify the
   current step as the step directory with the greatest number. Stop on
   missing or unusable required context, or when that step is not
   `accepted`; a drafting or amending step is accepted first through
   `dev-plan-step` with `waif step accept`.
2. Read `goal.md`, `plan.md`, and the complete `step.md`. Require all three to
   have `Status: accepted`, verify exact goal-criterion and plan-item mappings,
   and verify the workspace branch.
3. Before the first implementation attempt, require a clean workspace unless
   the user explicitly identifies existing changes as part of this step.
4. Implement only the accepted step, following workspace instructions and
   existing code patterns. Add or update tests proportionate to the change.
5. Run every validation command in `step.md`, plus relevant repository checks.
   Fix failures caused by the step.
6. Spawn a fresh subagent and ask it to use `dev-review` on the current
   step. Pass the validation results and any REF or record path other than
   its defaults, but no suggested verdict or explanation of the changes. If
   subagents are unavailable, run `dev-review` in the current session.
7. Present the implementation with the record's verdict, findings, and path,
   only when nothing changed after the latest record was written; any later
   change needs a new record first.
8. Interpret the response:
   - On requested changes, revise the implementation, rerun validation, and
     obtain a new review record as in step 6.
   - Accept changes only when the human manually stages them. Inspect the staged
     diff and verify it contains only reviewed changes for this step. If the
     staged diff is empty, unrelated, or differs materially from the reviewed
     diff, do not commit and explain what the human must correct manually.
   - Rerun required validation when the staged content differs from the content
     last validated, then run `waif step commit -F -` with the commit message
     on standard input. It commits exactly the staged changes.
9. Read the `waif step commit` output. On success the helper has already
   recorded the full commit hash in `Source commit`, set the step
   `Status: done`, refreshed `Updated`, and marked plan items with `complete`
   coverage `done`; plan items with only `partial` coverage stay `pending`.
10. Report completion and provide the workflow guide:
    - use `dev-plan-step` when more implementation is needed;
    - use `dev-final-review` when implementation is complete.
    Do not ask for or infer the human's completion decision in this skill.

## Guardrails

- Never commit before human staging.
- Treat staging as the only source-change acceptance signal.
- Never run staging-related commands, including `git add`, `git reset`,
  `git restore --staged`, or equivalents. The human exclusively selects staged
  paths and content.
- Commit exactly the verified staged diff.
- Exclude unstaged and unrelated changes.
- Do not amend or rewrite earlier source commits unless the user requests it.
- Stop when new instructions would materially change the accepted step plan.
  Do not revise the accepted artifact or continue under changed scope.

## Done When

- Accepted source changes are committed on the recorded branch.
- `step.md` has `Status: done` and `Source commit: <full commit hash>`.
- Plan items with `complete` coverage are `done`.
- Plan items with only `partial` coverage remain `pending`.
- Every review record written by `dev-review` remains in the step
  directory.
