---
name: dev-draft-goal
description: >-
  Use only when the user explicitly invokes `dev-draft-goal`. Create or revise
  the durable, non-technical goal for a software-development task.
---

# Draft Development Goal

Create the task dir and keep `goal.md` as the product-level contract.

## Procedure

1. Read the user's request, workspace instructions, and
   [workflow.md](../../protocol/workflow.md).
2. Resolve or create the workflow dir using
   [Workflow Dir Setup](#workflow-dir-setup), then resolve the task:
   - revise `tasks/current` only when the user is clearly continuing it;
   - otherwise create the next task dir and update `tasks/current`.
3. Draft or revise `goal.md` from the latest user input using the
   [`goal.md` Contract](#goalmd-contract).
4. Keep the goal product-level: outcome, scope, and stable acceptance
   criteria, with no implementation choices.
5. Keep `Status: drafting` before initial acceptance.
   When changing an accepted goal:
   - set `Status: amending`;
   - maintain one revision subsection for the amendment cycle;
   - preserve history;
   - tell the user which downstream artifacts need reassessment.
6. Show the `goal.md` path and ask the user to review it.
7. On explicit acceptance, require no blocking open questions and set
   `Status: accepted`. Treat unqualified `accept`, `approve`, or equivalent as
   acceptance of the current goal.
8. Apart from optional default-setting persistence below, do not modify or
   commit workspace files.

## Workflow Dir Setup

Goal drafting owns workflow dir resolution and initialization.

1. Use the most specific available workflow-dir instruction from the user,
   prior explicit direction, a requested settings file, or workspace
   instructions. Recognize settings like `Workflow dir:`.
2. If no setting exists:
   - use `<workspace-dir>/.ai-dev-workflow`;
   - commonly persist `- Workflow dir: .ai-dev-workflow` in the workspace-root
     `AGENTS.md` so future agents are likely to receive the setting;
   - when doing so, create the file if needed or append a
     `## Development Workflow` section without replacing unrelated
     instructions;
   - do not add a duplicate setting.
3. Resolve a relative setting from the workspace dir. Validate the resolved
   path using the shared workflow-dir containment rule.
4. If the workflow dir is missing, create it and run `git init` there. If it
   already exists, use it without inspecting or changing repository metadata.

Creating a missing workflow dir and running `git init` there are setup
operations, not approval gates. Persisting the default in `AGENTS.md` is a
compatibility measure, not a requirement. Do not edit `.gitignore`.

## `goal.md` Contract

```markdown
# Goal: <short title>

- Status: drafting
- Created: YYYY-MM-DDTHH:MM:SS+HH:MM
- Updated: YYYY-MM-DDTHH:MM:SS+HH:MM

## Outcome

<What should be true for the user or system when the task succeeds.>

## Acceptance Criteria

- [ ] G-AC1: <Observable, technology-independent result>

## Scope

### In Scope

- G-IN1: <Included behavior>

### Out of Scope

- G-OUT1: <Explicit exclusion>

## Open Questions

- G-Q1: <Major issue, why it matters, and several options the human can adopt>

## Assumptions

- G-A1: <Minor issue and the best-effort solution adopted>

## Revisions

### G-REV1 - YYYY-MM-DDTHH:MM:SS+HH:MM - <amendment title>

- Before: <What the accepted goal said before this amendment.>
- Changed:
  - <Change made during this amendment cycle.>
```

`Outcome` and `Acceptance Criteria` are required. Omit empty optional sections
and empty `In Scope` or `Out of Scope` subsections.

Use monotonically increasing IDs:
- preserve IDs;
- never reuse retired IDs.

Statuses are `drafting`, `accepted`, and `amending`. Do not accept a goal with
blocking `Open Questions`. Follow shared amendment and revision rules.

## Guardrails

- Preserve useful existing content, active task dirs, and downstream history.
- Ask only questions that materially change outcome or scope.
- Do not add architecture, libraries, file paths, APIs, sequencing, or other
  technical planning. That belongs to `dev-draft-plan`.

## Done When

- The workflow dir, task dir, and `current` symlink are valid.
- `goal.md` matches the contract and latest user direction.
- Every goal content item has a stable ID.
- Accepted goals have no unresolved blocking questions.
