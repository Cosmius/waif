---
name: dev-draft-charter
description: >-
  Use only when the user explicitly invokes `dev-draft-charter`. Create or
  revise the workflow-wide project rules in the workflow charter.
---

# Draft Project Charter

Create or revise `<workflow-dir>/charter.md` as concise, actionable,
project-wide rules, then present its path and a change summary for review.

## Rules

- Read the user's request, workspace instructions,
  [workflow.md](../../protocol/workflow.md), and the complete existing charter
  when present.
- Establish a missing `<workspace-dir>/.waif` and initialize it as a Git
  repository. Do not inspect or change existing workflow repository metadata.
- Preserve unmentioned rules. Use Markdown without lifecycle or task state,
  and do not duplicate the workflow protocol.
- Change only the charter. Do not modify task artifacts, workspace source,
  `.gitignore`, or workflow repository metadata. Do not stage or commit.
