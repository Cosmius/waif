# Plan: {{ task title }}

- Status: drafting
- Goal: ./goal.md
- Branch: {{ intended task branch name }}
- Base branch: {{ checked-out branch captured at initial drafting }}
- Base commit: {{ full commit hash captured at initial drafting }}
- Created: YYYY-MM-DDTHH:MM:SS+HH:MM
- Updated: YYYY-MM-DDTHH:MM:SS+HH:MM

## Technical Summary

{{ Approach and important design choices. }}

## Decisions

- P-D1: {{ Technical decision, rationale, and alternatives rejected }}
- P-D2: Vide infra, {{ crucial decision name }}.

### P-D2: {{ Crucial technical decision }}

{{ Further explanation for a decision that needs its own section. }}

## Open Questions

- P-Q1: {{ Major issue, why it matters, and options the human can adopt }}

## Assumptions

- P-A1: {{ Minor issue and the best-effort solution adopted }}

## Current System

{{ Relevant architecture, constraints, and existing behavior. }}

## Plan Items

### P1: {{ Broad implementation unit }}
- Status: pending
- Goal criteria: G-AC1
- Outcome: {{ technical result }}
- Areas: {{ components or subsystems involved }}
- Depends on: none
- Validation: `{{ command or observable check }}`

### P2: {{ Broad implementation unit }}
- Status: pending
- Goal criteria: G-AC1, G-AC2
- Outcome: {{ technical result }}
- Areas: {{ components or subsystems involved }}
- Depends on: P1
- Validation: `{{ command or observable check }}`

## Risks

- P-R1: {{ Risk and mitigation }}

## Cross-Cutting Validation

- P-V1: `{{ whole-task validation command }}`

## Revisions

### P-REV1: YYYY-MM-DDTHH:MM:SS+HH:MM - {{ amendment title }}
- Before: {{ What the accepted plan said before this amendment. }}
- Changed:
  - {{ Change made during this amendment cycle. }}
