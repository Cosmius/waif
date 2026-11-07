---
name: dev-final-review
description: >-
  Perform the final task-wide review of a completed WAIF implementation. Use
  only when explicitly invoked, the user requests WAIF, or work continues an
  existing WAIF task. Charter work alone does not count as task context.
---

# Final Task Review

Obtain a new task-wide review record and present it.

## Procedure

1. Read [workflow.md](../../protocol/workflow.md), resolve the invoking
   workspace, local library, and current task, and stop if required context is
   missing or unusable.
2. Require an accepted `goal.md` and an accepted `plan.md`.
3. Spawn a fresh subagent and ask it to use `dev-review` on the whole task.
   Pass only REFs the user gave in place of its defaults, and no suggested
   verdict or explanation. If subagents are unavailable, run `dev-review` in
   the current session.
4. Present the verdict, a summary of the findings, and the
   `reviews/reviewN.md` path.
5. Route a request to waive a finding to `dev-review`, with the record path,
   the finding, and the user's reason.

## Guardrails

- Do not change or commit workspace source files.
- Do not hide failed validation or unmet acceptance criteria.
- Do not delete or retarget the local `current_task` pointer.

## Done When

- A new `GOOD` task review record exists in the task's `reviews/` directory.
- Its verdict, findings summary, and path are presented.
