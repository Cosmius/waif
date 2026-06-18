---
name: dev-final-review
description: >-
  Use only when the user explicitly invokes `dev-final-review`. Produce the
  final task-wide implementation review after all intended steps are done.
---

# Final Task Review

Audit the complete task and write the durable final review.

## Procedure

1. Read [workflow.md](../../protocol/workflow.md), resolve the established
   workflow dir and current task dir, and stop if either is missing.
2. Read the complete accepted `goal.md`, accepted `plan.md`, every
   `steps/*/step.md`, and every `steps/*/review*.md`. Read step artifacts in
   step sequence order and review files in review-number order within each
   step.
3. Read workflow artifacts as context for the code review. Do not raise
   workflow or procedure findings unless they are necessary to explain how a
   substantive code, behavior, validation, or integration risk arose. Check
   only enough task-level completeness to understand the intended code result:
   - every plan item is `done`;
   - every `steps/*/step.md` has `Status: done` and a non-`not-created`
     `Source commit`;
   - every acceptance criterion has evidence;
   - final validation passes.
   Do not over-verify routine workflow metadata such as commit existence,
   hash-to-step mapping, or reachability. Investigate procedure details only
   when doing so helps identify the cause or impact of a substantive code risk.
4. Review the aggregate diff from the recorded base commit through the current
   tip of the plan branch for substantive cross-step regressions, inconsistent
   behavior, missing integration coverage, unrelated code changes, and
   divergence from the goal or plan.
5. Write or replace `review.md` according to the
   [Final Review Contract](#final-review-contract).
6. Do not change or commit workspace source files during the final review.
7. Present the verdict, findings, and review path to the user.

## Final Review Contract

```markdown
# Final Review: <task title>

- Date: YYYY-MM-DDTHH:MM:SS+HH:MM
- Goal: ./goal.md
- Plan: ./plan.md
- Branch: <workspace branch>
- Base commit: <full commit hash>
- Reviewed tip: <full commit hash>
- Verdict: pass

## Summary

<What was implemented and the resulting behavior.>

## Goal Assessment

| ID    | Acceptance criterion | Result | Evidence                  |
|-------|----------------------|--------|---------------------------|
| G-AC1 | <criterion>          | pass   | <test, file, or behavior> |

## Plan Completion

| ID | Item    | Result | Steps                   | Commits  |
|----|---------|--------|-------------------------|----------|
| P1 | <title> | done   | `steps/01-name/step.md` | `<hash>` |

## Validation

- FR-V1: `<command>`: passed

## Review History

- FR-H1: `steps/01-name/review1.md`: pass

## Findings

No findings.

## Residual Risks

- FR-R1: None.
```

Populate the `Commits` column from each done step's top-level `Source commit`
field.

Verdicts are `pass` and `concerns`. Use `concerns` for:
- failed validation;
- unmet acceptance criteria;
- an invalid Git range;
- material cross-step findings.

When findings exist, use stable IDs (`FR-F1`, `FR-F2`, ...) and include
severity, evidence, impact, and recommendation. Preserve source IDs in
assessment and completion tables. Give every other report list or table item a
stable `FR-<series><number>` ID.

The user may explicitly ignore or waive a finding only by providing a reason.
Keep the finding in `Findings` and record the user's rationale with it.
Do not block the waiver because the rationale seems weak or incorrect. If the
reviewer disagrees with the rationale, add a concise counter-opinion to the
finding, but still exclude that ignored finding from the final verdict.

## Guardrails

- Report evidence and review exactly the recorded base commit through the
  reviewed branch tip.
- Do not hide failed validation or unmet acceptance criteria.
- Treat procedure issues as context, not final-review findings, unless they
  explain a substantive code, behavior, validation, or integration risk.
- Use `concerns` when a material finding remains, even if individual step
  reviews passed.
- Do not let user-ignored findings affect the verdict, but keep them in the
  review with the user's required reason and any reviewer counter-opinion.
- Keep final-review findings separate from historical
  `steps/*/review*.md` records.
- Do not delete or retarget `tasks/current`.

## Done When

- `review.md` accounts for the goal, plan, steps, reviews, commits, and
  validation.
- Every review content item has a stable ID or preserves its source item ID.
- The workflow dir contains the review.
- The workspace remains unchanged by this skill.
