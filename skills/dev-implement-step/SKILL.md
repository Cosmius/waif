---
name: dev-implement-step
description: >-
  Use only when the user explicitly invokes `dev-implement-step`. Implement the
  current accepted step, validate it, review changes, and commit human-staged
  changes.
---

# Implement Development Step

Implement one accepted step through a review-and-acceptance loop.

## Procedure

1. Read canonical [workflow.md](../../protocol/workflow.md), resolve the
   established workflow dir and current task dir, and identify the one step
   whose status is `accepted`. Stop when a required directory is missing or
   when there are zero or multiple candidate steps.
2. Read `goal.md`, `plan.md`, and the complete `step.md`. Require all three to
   have `Status: accepted`, verify exact goal-criterion and plan-item mappings,
   and verify the workspace branch.
3. Before the first implementation attempt, require a clean workspace unless
   the user explicitly identifies existing changes as part of this step.
4. Implement only the accepted step, following workspace instructions and
   existing code patterns. Add or update tests proportionate to the change.
5. Run every validation command in `step.md`, plus relevant repository checks.
   Fix failures caused by the step.
6. Spawn a new independent subagent to review the uncommitted workspace diff.
   Give it the goal, plan, step, repository instructions, diff, and validation
   results, but not a desired verdict. If subagents are unavailable, perform
   the review in the current session and record that fallback in
   `steps/NN-short-name/reviewN.md`.
7. Save the next gapless `reviewN.md` in the current step directory using the
   [Implementation Review Contract](#implementation-review-contract). Scope
   review numbering to that step directory.
8. Present the implementation and review findings to the user.
9. Interpret the response:
   - On requested changes, revise the implementation, rerun validation, run a
     fresh review, and create `reviewN+1.md` in the same step directory.
   - Accept changes only when the human manually stages them. Inspect the staged
     diff and verify it contains only reviewed changes for this step. If the
     staged diff is empty, unrelated, or differs materially from the reviewed
     diff, do not commit and explain what the human must correct manually.
   - Rerun required validation when the staged content differs from the content
     last validated, then commit exactly the staged changes.
10. After the workspace commit succeeds:
    - replace `Source commit: not-created` in `step.md` with the full commit
      hash;
    - set the step `Status: done`;
    - refresh the step `Updated` timestamp;
    - mark mapped plan items `done` only when cumulative coverage from done
      steps completes their outcomes;
    - otherwise leave mapped plan items `pending`.
11. Report completion and provide the workflow guide:
    - use `dev-plan-step` when more implementation is needed;
    - use `dev-final-review` when implementation is complete.
    Do not ask for or infer the human's completion decision in this skill.

## Implementation Review Contract

Review the current uncommitted workspace diff against:

- the accepted goal;
- the accepted general plan;
- the current accepted step;
- applicable repository instructions;
- executed validation and its actual output.

Use a newly spawned subagent for every iteration when available. Do not suggest
a verdict or reveal previous conclusions. Include prior review files only when
checking whether findings were addressed. If subagents are unavailable, review
in the current session with the same inputs and priorities, and mark the
fallback in the artifact.

Use this structure for `steps/NN-short-name/reviewN.md`:

```markdown
# Implementation Review N

- Step: ./step.md
- Decision: pass
- Date: YYYY-MM-DDTHH:MM:SS+HH:MM
- Reviewer: independent subagent
- Workspace state: uncommitted

## Findings

No findings.

## Scope

- S1-R1-SC1: <Files and behavior reviewed>

## Validation

- S1-R1-V1: `<command>`: passed

## Residual Risks

- S1-R1-RR1: None.
```

Decisions are `pass` and `changes-requested`.

Reviewers are:
- `independent subagent`;
- `current session - subagent unavailable`.

Always identify which reviewer produced the artifact.

When findings exist, order them by severity:

```markdown
## Findings

### S1-R1-F1: High - <short title>

- Location: `path/to/file:line`
- Problem: <observable defect or risk>
- Impact: <why it matters>
- Recommendation: <specific correction>
```

Do not add an artifact `Status`. Review records are immutable observations.
- Create `review1.md`, `review2.md`, and so on in the current step directory
  without gaps or overwrites.
- Scope the review number `N` to the current step directory. Do not number
  implementation reviews globally across the task.
- Prefix review content IDs with unpadded step and review numbers, such as
  `S1-R2-F1`.
- Preserve source artifact IDs when referring to goal criteria, plan items, or
  step items.

## Review Priorities

The reviewer must prioritize correctness, regressions, security, data loss,
concurrency, compatibility, and missing tests. Findings must cite files and
lines where possible. A review is evidence for the human, not an automatic
acceptance gate.

## Guardrails

- Never commit before human staging.
- Treat staging as the only source-change acceptance signal.
- Never run staging-related commands, including `git add`, `git reset`,
  `git restore --staged`, or equivalents. The human exclusively selects staged
  paths and content.
- Commit exactly the verified staged diff.
- Exclude unstaged and unrelated changes.
- Never mark a step or plan item done before the workspace commit succeeds, or
  mark a plan item done merely because one mapped step is done.
- Do not amend or rewrite earlier source commits unless the user requests it.
- Do not reuse or overwrite review files.
- Do not write review files outside the current step directory.
- Stop when new instructions would materially change the accepted step plan.
  Do not revise the accepted artifact or continue under changed scope.

## Done When

- Accepted source changes are committed on the recorded branch.
- `step.md` has `Status: done` and `Source commit: <full commit hash>`.
- Fully consumed plan items are `done`.
- Partially consumed plan items remain `pending`.
- All review iterations remain available as
  `steps/NN-short-name/reviewN.md`.
- Every review content item has a stable, step-and-review-qualified ID.
