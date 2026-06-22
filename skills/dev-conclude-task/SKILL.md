---
name: dev-conclude-task
description: >-
  Use only when the user explicitly invokes `dev-conclude-task`. Create a
  self-contained, repository-owned handoff from completed task artifacts, Git
  history, and relevant source, leaving acceptance and Git actions to the user.
---

# Conclude Development Task

Explain the completed task for who knows the project but not the goal.

## Procedure

1. Read [workflow.md](../../protocol/workflow.md) and try to resolve the
   workflow and current task dirs. Treat structure and statuses as sanity
   checks, not prerequisites.
2. If task state is missing, broken, or inconsistent, explain the problem and
   ask whether to continue. If the user continues, use the identified dir and
   available evidence without requiring accepted or done statuses.
3. Read available `goal.md`, `plan.md`, and every `steps/*/step.md`. For each
   step, read only its highest-numbered `reviewN.md`. Read task-level
   `review.md` when present.
4. Resolve the Git range from `plan.md`: use its `Base commit` through the tip
   of its recorded `Branch`. If `plan.md` is missing or the range cannot be
   resolved, ask the user for the Git range and pause. Read the log for that
   range to determine what changed. Do not read every commit diff unless
   requested; inspect selected diffs or current source only when needed.
5. Reconcile artifact intent with Git and source evidence. Account for hybrid,
   skipped, repeated, incomplete, or out-of-plan work. Describe divergences
   neutrally; do not perform a new review or rerun validation.
6. Use the user's repository-relative destination, or default to
   `docs/task-reports/<task-dir-name>.md`. If no task dir resolves, derive a
   task slug. Do not overwrite uncommitted content without permission.
7. Write a self-contained document using the contract below. Verify facts
   against consulted sources and state evidence gaps instead of guessing.
8. Change only the document. Do not stage or commit anything. Present its path
   and a concise summary.

## Task Conclusion Contract

```markdown
# Task: <task title>

- Date: YYYY-MM-DDTHH:MM:SS+HH:MM
- Task: <plain-language task name>
- Branch: <workspace branch>

## Executive Summary

- <What changed and why it matters.>
- <Resulting behavior or outcome.>

## Original Goal

- Outcome:
  - <Original intended outcome.>
- Acceptance criteria:
  - <Criterion in plain language.>
- Scope:
  - <Included or excluded behavior needed to understand the task.>
- Amendments:
  - <Material change to the original goal, when any.>

## Adopted Approach

- Approach:
  - <Technical approach used.>
- Decisions:
  - <Important decision and rationale.>
- Constraints or rejected alternatives:
  - <Relevant constraint or alternative.>

## Implementation

- <Behavior or subsystem>:
  - Behavior: <What it does.>
  - Components: <Important paths, interfaces, or data flow.>
  - History: <Relevant Git history or departure from the plan.>
  - Tests: <Relevant tests.>

## Validation and Outcome

- Validation:
  - <Recorded command or observable evidence and its result.>
- Goal outcome:
  - <Acceptance-criterion result supported by available evidence.>
- Review outcome:
  - <Latest recorded review result, when available.>

## Important Context

- Limitations and residual risks:
  - <Known limitation or risk, or `None recorded.`>
- Operational and compatibility notes:
  - <Important note, or `None recorded.`>
- Follow-up work:
  - <Known follow-up, or `None recorded.`>
```

## Guardrails

- Prefer lists and tables. Use IDs only for internal cross-references.
- Include all context needed to understand the task. Do not link to or name
  workflow artifacts in the document.
- Use repository-relative source paths and commit hashes only when useful.
- Report recorded reviews as history; do not issue a new verdict.
- Distinguish artifact facts from Git or source inference.
- Tell the user separately about material discrepancies discovered while
  inspecting source.
- Exclude secrets, personal data, irrelevant logs, and unsupported claims.
- Do not modify workflow artifacts, source code, or the Git index.

## Done When

- The document covers the goal, approach, implementation, evidence, and
  important context without requiring workflow history.
- Only the document changed, and it is ready for human review.
