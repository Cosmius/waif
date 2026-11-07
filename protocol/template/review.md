# Review 2

- Step: ./step.md
- Date: __NOW__
- Reviewer: independent subagent
- Verdict: concerns
- Reviewed: __LAST_COMMIT__-WORKING
- Reviewed against: __LAST_COMMIT__ (HEAD, __CURRENT_BRANCH__)

## Summary

{{ One or two sentences delivering the result: verdict and why. }}

{{ What the reviewed changes do, in more detail. }}

## Findings

### S1-R2-F1: {{ short description, still open from S1-R1-F1 }}
- Severity: medium
- Location: {{ `path/to/file.rs:42` }}
- Problem: {{ what is wrong }}
- Impact: {{ what goes wrong because of it }}
- Recommendation: {{ how to fix it }}

### S1-R2-F3: {{ short description of a new finding }}
- Severity: low
- Location: {{ `path/to/file.rs:57` }}
- Problem: {{ what is wrong }}
- Impact: {{ what goes wrong because of it }}
- Recommendation: {{ how to fix it }}

## Assessment

- S1-DC1: met - {{ evidence: test, file, or behaviour }}
- S1-DC2: unmet - {{ what is missing; see S1-R2-F1 }}

## Review History

- `review1.md`: concerns - S1-R1-F2 fixed; S1-R1-F1 still open as S1-R2-F1

## Validation

- S1-R2-V1: `{{ command }}`: passed

## Residual Risks

- S1-R2-RR1: {{ risk that remains after this review }}
