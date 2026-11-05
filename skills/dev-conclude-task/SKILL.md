---
name: dev-conclude-task
description: >-
  Write a self-contained handoff for a completed WAIF task. Use only when
  explicitly invoked, the user requests WAIF, or work continues an existing
  WAIF task. Charter work alone does not count as task context.
---

# Conclude Development Task

Explain the completed task for who knows the project but not the goal.

## Procedure

1. Read [workflow.md](../../protocol/workflow.md) and try to resolve the
   invoking workspace, local library, and current task. Treat structure and
   statuses as sanity checks, not prerequisites.
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
7. Write a self-contained document using the
   [Task Conclusion Contract](#task-conclusion-contract). Verify facts against
   consulted sources and state evidence gaps instead of guessing.
8. Change only the document. Do not stage or commit anything. Present its path
   and a concise summary.

## Task Conclusion Contract

Follow the
[task conclusion template](../../protocol/template/task-conclusion.md).

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
