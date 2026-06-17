# AI-Assisted Development Workflow for a Single Task

All workflow skills must read and follow this file before acting. Terms in this
file are canonical.

## Workflow Overview

The workflow has five skills and four human acceptance gates. Each skill owns
one stage, persists its output in the workflow dir, and stops at its gate
instead of silently advancing.

```text
dev-draft-goal
  -> accepted goal
dev-draft-plan
  -> accepted general plan and workspace branch
dev-plan-step
  -> accepted detailed step
dev-implement-step
  -> reviewed and accepted source commit
  -> human decides: more steps or invokes final review
     -> more steps: return to dev-plan-step
     -> final review: continue to dev-final-review
dev-final-review
  -> final task review
```

### Milestones

| Stage               | Skill                | Requires                  | Milestone                                                 |
|---------------------|----------------------|---------------------------|-----------------------------------------------------------|
| Goal                | `dev-draft-goal`     | User request              | Accepted `goal.md` in a new or current task dir           |
| General plan        | `dev-draft-plan`     | Accepted goal             | Accepted `plan.md` and checked-out workspace task branch  |
| Step plan           | `dev-plan-step`      | Accepted goal and plan    | One accepted `steps/NN-name/step.md` mapped to plan items |
| Step implementation | `dev-implement-step` | One accepted step         | Validated, reviewed, human-accepted source commit         |
| Final review        | `dev-final-review`   | User invokes final review | `review.md` covering the whole task                       |

### Human Gates and Loops

- Goal, plan, and step artifacts remain `drafting` until the human explicitly
  accepts them.
- Human-requested changes repeat the current stage.
- After each completed implementation step, the human decides whether the task
  needs another step.
- Invoking `dev-final-review` means the human considers implementation
  complete.

### Stage Boundaries

- `dev-draft-goal` alone may establish the workflow dir and create the task
  dir.
- `dev-draft-plan` creates the general plan and workspace task branch after
  goal acceptance.
- `dev-plan-step` plans one coherent implementation step without changing
  the workspace source.
- `dev-implement-step` completes one accepted step.
- `dev-final-review` audits and reports; it does not modify the workspace
  source.

## Terms

### Goal

A `goal` is an accepted product outcome and scope for one development task.
It describes required behavior, constraints, and success criteria without
prescribing the technical implementation. It is persisted in `goal.md`.

### General Plan

A `general plan` is an accepted technical approach for achieving the goal.
It divides the work into stable plan items while leaving detailed source edits
and command sequences to step planning. It is persisted in `plan.md`.

### Step

A `step` is one coherent implementation unit suitable for one source commit.
It may consume all or part of one or more general plan items; plan items and
steps do not map one to one.

### Step Plan

A `step plan` is the detailed, executable plan for one step. It defines the
step's exact plan-item coverage, source changes, tests, validation, risks, and
done conditions. It is persisted in `steps/NN-name/step.md`.

### Task Artifact

A `task artifact` is a durable Markdown record of task state, decisions,
plans, reviews, or results stored in the task dir. Task artifacts include
`goal.md`, `plan.md`, each `step.md`, each `reviewN.md`, and `review.md`.

### Workspace Dir

The `workspace dir` is the root directory of the software project being
changed. It contains the source repository and its workspace instructions.

### Workflow Dir

The `workflow dir` is the directory that stores durable workflow artifacts.

Resolve it from the most specific explicit instruction available to the agent.
The instruction may come from:

1. the user's current message or prior explicit direction;
2. a settings file the user asks the agent to read;
3. workspace instructions already provided to the agent, such as `AGENTS.md`.

Recognize settings like `Workflow dir:`. Resolve a relative setting from the
workspace dir.

After goal drafting has established it, the workflow dir:

- exists as a directory;
- is a proper descendant of the workspace dir.

All stages after goal drafting must stop if the setting is absent, the path is
missing, or the directory invariant is violated. They must not create or repair
the workflow dir.

### Task Dir

The `task dir` is the directory for one development task. It is a direct child
of `<workflow-dir>/tasks/`, is created during goal drafting, and is managed
under the workflow dir.

Name a task dir `YYYYMMDD-N-short-name`, for example
`20260613-2-add-session-timeout`.

- Use the user's local date.
- Determine `N` by scanning sibling task dirs with the same date prefix.
- Use a concise lowercase-hyphenated short name.

### Current Task

`<workflow-dir>/tasks/current` is a relative symlink to the active task dir,
for example:

```text
current -> ./20260613-2-add-session-timeout
```

All stages after initial goal drafting resolve the task dir through this
symlink. Stop if it is missing, broken, ambiguous, or points outside
`<workflow-dir>/tasks/`.

## Directory Structure

```text
<workspace-dir>/
├── <workflow-dir>/
│   └── tasks/
│       ├── 20260501-1-task-name/
│       │   └── ...
│       ├── 20260601-1-task-name/
│       │   └── ...
│       ├── 20260601-2-task-name/
│       │   ├── goal.md
│       │   ├── plan.md
│       │   ├── steps/
│       │   │   ├── 01-step-name/
│       │   │   │   └── ...
│       │   │   └── 02-step-name/
│       │   │       ├── step.md
│       │   │       ├── review1.md
│       │   │       └── review2.md
│       │   └── review.md
│       └── current -> ./20260601-2-task-name
└── <workspace files>
```

## Task Artifacts

Task artifacts are the authoritative, human-readable workflow record. Each
skill-specific contract defines the required content of the artifacts it owns.

### Format

- Use Markdown.
- Put the title first.
- Put top-level metadata immediately after the title as unordered-list items
  in the form `- Field: value`.
- Use the exact field names, section names, allowed values, and ordering
  defined by the owning skill's artifact contract.
- Omit optional sections when they contain no content.
- Use task-dir-relative paths when one task artifact refers to another.

### Item IDs

Give every independently referable content item a stable ID. Content items
include checklist entries, list entries, numbered actions, table rows,
findings, and revision records. Top-level metadata fields and narrative
paragraphs do not need IDs.

- Format IDs as defined by the owning artifact contract.
- Keep IDs unique within the task and qualified by artifact kind.
- Preserve an item's ID when editing it.
- Never renumber or reuse an ID after removing or superseding an item.
- Refer to items by ID in artifacts and chat instead of relying on position or
  heading text.
- Prefer itemized content over compound narrative text when separate statements
  may be reviewed, accepted, implemented, or discussed independently.

Use these artifact prefixes:

- `G-` for goal items;
- `P-` and `P<number>` for plan content and plan items;
- `S<NN>-` for step content;
- `S<NN>-R<N>-` for implementation-review content;
- `FR-` for final-review content.

Review records and the final review use `Decision` or `Verdict` rather than a
lifecycle `Status`. Do not add fields that imply unsupported workflow state.

### Timestamps

An artifact timestamp records the local date, time, and UTC offset in RFC 3339
form, for example `2026-06-14T18:30:00+09:00`. Use this format for all artifact
creation, update, review, report, and revision timestamps. Keep `Created`
unchanged and refresh `Updated` whenever an artifact changes.

### Status

`goal.md`, `plan.md`, and each step plan persist their lifecycle in a
top-level `Status` field.

The goal and general plan have three statuses:

```text
drafting -> accepted <-> amending
```

Each step plan has three statuses:

```text
drafting -> accepted -> done
```

Status meanings:

- `drafting`: the artifact is being prepared for human review.
- `accepted`: the human has accepted the artifact.
- `amending`: an accepted goal or general plan is being revised.
- `done`: implementation of the step plan is complete.

Only an `accepted` goal and general plan may be consumed by later skills.
`amending` returns to `accepted` only after explicit human acceptance.

Goals and general plans may repeat the `accepted -> amending -> accepted`
cycle any number of times. Each amendment requires a new explicit acceptance.

Do not reset an accepted or amending goal or general plan to `drafting`.
Only step plans follow an accept-once rule. After a step plan changes from
`drafting` to `accepted`, its planned content is immutable. Only lifecycle
metadata written by `dev-implement-step`, such as `Status`, `Updated`, and the
source commit, may change.

### Ambiguity Triage

Before presenting any artifact for human acceptance, triage unresolved
ambiguity into `Open Questions` and `Assumptions`.

- Put major issues in `Open Questions`. Explain each issue in detail, describe
  why it matters, and provide several concrete options the human can adopt.
  The artifact must not be accepted until these questions are resolved.
- Put minor issues in `Assumptions`. Record a best-effort solution so the human
  can review the choice without blocking acceptance.

An issue is major when its resolution could materially change the artifact's
outcome, scope, approach, risk, or acceptance criteria. Other ambiguity is
minor unless the human marks it as blocking.

This rule applies to every artifact that requires human acceptance, including
the goal, general plan, and step plan. Omit `Open Questions` or `Assumptions`
when that section would be empty.

### Revision Rules

The `Revisions` section applies only to goals and general plans changed after
acceptance.

When modifying an accepted goal or general plan:

1. Change its status from `accepted` to `amending`.
2. Create `## Revisions` if absent.
3. Add one subsection for the amendment cycle.
4. Keep all further changes in that cycle in the same subsection.
5. On explicit human acceptance, return the status to `accepted`.

Do not add another revision subsection until a later amendment cycle begins.
Capture `Before` once at the start of the cycle. Accumulate every change made
in that cycle under `Changed`; the human accepts them together.

A status-only lifecycle transition does not require a revision entry.

Each revision record must have a stable ID and state:

- when the revision was made;
- what the artifact said or required before the revision;
- what changed.

Preserve all earlier revision subsections.

Example:

```markdown
## Revisions

### G-REV1 - 2026-06-14T18:30:00+09:00 - Extend timeout scope

- Before: The goal covered configurable timeouts for browser sessions only.
- Changed:
  - Added API sessions to the required timeout behavior.
  - Clarified that existing sessions keep their current timeout.
```

### State Rules

- Markdown artifacts are authoritative workflow state.
- Persist decisions to artifacts instead of relying on chat history.
- Never commit workflow artifacts to the workspace repository.
- Preserve completed artifacts and source commits as history when upstream
  artifacts are revised.
