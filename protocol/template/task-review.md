# Task Review 2

- Goal: ../goal.md
- Plan: ../plan.md
- Date: __NOW__
- Reviewer: independent subagent
- Verdict: concerns
- Reviewed: __LAST_COMMIT__ (HEAD, __CURRENT_BRANCH__)
- Reviewed against: {{ plan base commit }} ({{ base branch }})

## Summary

{{ One or two sentences delivering the result: verdict and why. }}

{{ What the task implemented and the resulting behaviour, in more detail. }}

## Findings

### R2-F1: {{ short description, still open from R1-F1 }}
- Severity: medium
- Location: {{ `path/to/file.rs:42` }}
- Problem: {{ what is wrong }}
- Impact: {{ what goes wrong because of it }}
- Recommendation: {{ how to fix it }}

### R2-F3: {{ short description of a new finding }}
- Severity: low
- Location: {{ `path/to/file.rs:57` }}
- Problem: {{ what is wrong }}
- Impact: {{ what goes wrong because of it }}
- Recommendation: {{ how to fix it }}

## Goal Assessment

- G-AC1: met - {{ evidence: test, file, or behaviour }}
- G-AC2: unmet - {{ what is missing; see R2-F1 }}

## Plan Completion

- P1: done - `steps/01-name` at `{{ full commit hash }}`
- P2: done - `steps/02-name` at `{{ full commit hash }}`, `steps/03-name`
      at `{{ full commit hash }}`

## Integration

- R2-I1: {{ cross-step behaviour, consistency, or out-of-plan change checked,
         and what was observed }}

## Review History

- `review1.md`: concerns - R1-F2 fixed; R1-F1 still open as R2-F1

## Validation

- R2-V1: `{{ command }}`: passed

## Deferred Work

- R2-DW1: {{ work consciously left for a later task, and why }}

## Residual Risks

- R2-RR1: {{ risk that remains after this review }}
