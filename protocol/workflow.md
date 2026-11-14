# AI-Assisted Development Workflow for a Single Task

All workflow skills must read and follow this file before acting. Terms in this
file are canonical.

## Workflow Overview

The workflow has five stage skills and four human acceptance gates. Each
stage skill owns one stage, persists its output in the workflow dir, and stops
at its gate instead of silently advancing. The supporting `dev-review` skill
writes every review record for `dev-implement-step` and `dev-final-review`.

```text
dev-draft-goal
  -> accepted goal
dev-draft-plan
  -> accepted general plan and workspace branch
dev-plan-step
  -> accepted detailed step
dev-implement-step
  -> reviewed and accepted source commit, or validated step without commit
  -> human decides: more steps or requests final review
     -> more steps: return to dev-plan-step
     -> final review: continue to dev-final-review
dev-final-review
  -> new task review record
```

### Milestones

| Stage               | Skill                | Requires                   | Milestone                                                 |
|---------------------|----------------------|----------------------------|-----------------------------------------------------------|
| Goal                | `dev-draft-goal`     | User request               | Accepted `goal.md` in a new or current task dir           |
| General plan        | `dev-draft-plan`     | Accepted goal              | Accepted `plan.md` and checked-out workspace task branch  |
| Step plan           | `dev-plan-step`      | Accepted goal and plan     | One accepted `steps/NN-name/step.md` mapped to plan items |
| Step implementation | `dev-implement-step` | One accepted step          | Validated, human-accepted commit (reviewed) or no commit  |
| Final review        | `dev-final-review`   | User requests final review | A new `reviews/reviewN.md` covering the whole task        |

### Human Gates and Loops

- Goal, plan, and step artifacts remain `drafting` until the human explicitly
  accepts them.
- Human-requested changes repeat the current stage.
- After each completed implementation step, the human decides whether the task
  needs another step.
- Requesting final task review, by name or in natural language, means the human
  considers implementation complete.

### Stage Boundaries

- `dev-workflow-setup` sets up the workflow dir and maintains the optional
  `library/charter.md`. It is not a task lifecycle stage or acceptance gate.
- `dev-draft-goal` creates the task dir, using `dev-workflow-setup` when needed.
- `dev-draft-plan` creates the general plan and workspace task branch after
  goal acceptance.
- `dev-plan-step` plans one coherent implementation step without changing
  the workspace source.
- `dev-implement-step` completes one accepted step.
- `dev-final-review` obtains and reports the task-wide review; it does not
  modify the workspace source.
- `dev-review` writes one review record of a step or the whole task, or
  records a waiver on one; it never modifies the workspace source.

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
A step that changes no source, such as one that only runs end-to-end tests,
makes no commit and needs no review record. It may consume all or part of
one or more general plan items; plan items and steps do not map one to one.

### Step Plan

A `step plan` is the detailed, executable plan for one step. It defines the
step's exact plan-item coverage, done conditions, key decisions, changed files,
tests, validation, and risks. It is persisted in `steps/NN-name/step.md`.

### Task Artifact

A `task artifact` is a durable Markdown record of task state, decisions,
plans, reviews, or results stored in the task dir. Task artifacts include
`goal.md`, `plan.md`, each `steps/NN-name/step.md`, each
`steps/NN-name/reviewN.md`, and each `reviews/reviewN.md`.

### Workspace Dir

The `workspace dir` is the root directory of the software project being
changed. It contains the source repository and its workspace instructions.
Resolve it with `git rev-parse --show-toplevel` from the invoking source
location, before entering `.waif`. In non-Git projects, use the established
project root. Run task-context helpers and source Git commands from this root.

### Workflow Dir

The `workflow dir` is the local instance at `<workspace-dir>/.waif`.
Each source worktree has its own instance and current-task pointer.

### Workflow Library

The `workflow library` is `<workflow-dir>/library`, containing `charter.md`
and `tasks/`. It may be a real directory or a symlink to a shared library.

### Charter

`<workflow-dir>/library/charter.md` is an optional, human-authored set of project
rules that applies to every task in the library. It is not a task artifact and
has no required format or lifecycle metadata.

After resolving the workflow dir, every workflow skill must read the complete
charter when it exists and apply it throughout that skill. If a charter rule
conflicts with a workflow skill, the charter rule prevails. The charter does
not override this canonical workflow protocol.

Workflow skills must not create, modify, or delete the charter unless the user
explicitly asks for that change.

### Task Dir

The `task dir` is the directory for one development task. It is a direct child
of `<workflow-dir>/library/tasks/` and is created during goal drafting.

Name a task dir `YYYYMMDD-N-short-name`, for example
`20260613-2-add-session-timeout`.

- Use the user's local date.
- Determine `N` by scanning sibling task dirs with the same date prefix.
- Use a concise lowercase-hyphenated short name.

### Manifests

The library and each task dir have a manifest, `library/waif.toml` and
`task.toml`, holding the `MAJOR.MINOR` version of the WAIF release that
introduced their schema:

```toml
schema_version = "0.3"
```

### Current Task

`<workflow-dir>/current_task` is a relative symlink to the active task dir,
for example:

```text
current_task -> library/tasks/20260613-2-add-session-timeout
```

Use `<workflow-dir>/current_task` as the current task's path. Stages
requiring a current task stop if it is missing or unusable.

## Directory Structure

```text
<workspace-dir>/
├── .waif/
│   ├── current_task -> library/tasks/20260601-2-task-name
│   └── library/                 # real directory or linked library symlink
│       ├── charter.md           # optional
│       ├── waif.toml            # library manifest
│       └── tasks/
│           ├── 20260501-1-task-name/
│           │   └── ...
│           ├── 20260601-1-task-name/
│           │   └── ...
│           └── 20260601-2-task-name/
│               ├── task.toml    # task manifest
│               ├── goal.md
│               ├── plan.md
│               ├── steps/
│               │   ├── 01-step-name/
│               │   │   └── ...
│               │   └── 02-step-name/
│               │       ├── step.md
│               │       ├── review1.md
│               │       └── review2.md
│               └── reviews/
│                   └── review1.md
└── <workspace files>
```

## Task Artifacts

Task artifacts are the authoritative, human-readable workflow record. Each
skill-specific contract defines the required content of the artifacts it owns,
starting from the artifact's template in [template/](template/).

### Format

- Use Markdown.
- Put the title first.
- Put structured top-level metadata after the title as unordered-list items in
  the form `- Field: value`. Every key recognized by the artifact contract is
  required and unique.
- Use the exact field names, section names, allowed values, and ordering
  defined by the owning skill's artifact contract.
- Omit optional sections when they contain no content.
- Use task-dir-relative paths when one task artifact refers to another.
- Write an item that needs its own section as `### ID: title`, with its
  `- Key: value` metadata directly below the heading. When the section also
  has `- ID: content` items, put it after them and keep a one-line
  `Vide infra` placeholder with the same ID in its place among them.
- Indent each continuation line of a wrapped `- ID: content` item so its
  content begins in the same column as the content after `ID: `.

```markdown
- G-AC1: First line of an acceptance criterion that wraps.
         Continuation aligned with the first content character.
```

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
- `S<NN>-R<N>-` for step review content;
- `R<N>-` for task review content.

Review records use `Verdict` rather than a lifecycle `Status`. Do not add
fields that imply unsupported workflow state.

### Timestamps

An artifact timestamp records the local date, time, and UTC offset in RFC 3339
form, for example `2026-06-14T18:30:00+09:00`. Use this format for all artifact
creation, update, review, report, and revision timestamps. Keep `Created`
unchanged and refresh `Updated` whenever an artifact changes.

### Status

`goal.md`, `plan.md`, and each step plan persist their lifecycle in a
top-level `Status` field.

The goal, general plan, and each step plan share five statuses. The normal
lifecycle is:

```text
drafting -> accepted <-> amending
            accepted -> done
```

`done` and `aborted` close an artifact. Transitions into and out of them are
unrestricted.

Status meanings:

- `drafting`: the artifact is being prepared for human review.
- `accepted`: the human has accepted the artifact.
- `amending`: an accepted goal, general plan, or step plan is being revised.
- `done`: the work the artifact describes is finished.
- `aborted`: the work the artifact describes was abandoned.

Only an `accepted` goal and general plan may be consumed by later skills.
`amending` returns to `accepted` only after explicit human acceptance.

Goals and general plans may repeat the `accepted -> amending -> accepted`
cycle any number of times. Each amendment requires a new explicit acceptance.

Set `done` or `aborted` only on the human's explicit request, except that
`waif step commit` sets a step `done`: change `Status` and refresh `Updated`
without adding a revision. Any workflow skill may record this by editing the
artifact directly. Likewise, move an artifact out of `done` or `aborted` only
on the human's explicit request.

A step plan normally becomes `done` through `waif step commit`, as described
below. A step set to `done` on request also needs its full commit hash in
`Source commit`, or `none` when the step made no commit; when the commit is
unclear, ask the human for the hash or whether to record `none` or `aborted`
instead. An `aborted` step keeps its `Source commit`.
When a step moves from `done` or `aborted` back to `drafting`, `accepted`, or
`amending`, reset `Source commit` to `not-created`.

In the normal lifecycle, do not reset an accepted or amending goal or general
plan to `drafting`. An accepted step plan changes only through an amendment
cycle until it is `done`, and a `done` step plan is not revised.
`waif step commit` writes the step's `Status`, `Updated`, and
`Source commit` when the step's source commit succeeds, or with
`--no-commit` when the human accepts a step without a commit, and marks plan
items with `complete` coverage `done`.
Marking a plan item `done` this way is not a plan amendment: the plan's
`Status` stays `accepted` and no revision is added.

### Ambiguity Triage

Before presenting any artifact for human acceptance, triage unresolved
ambiguity into `Questions` and `Assumptions`. Write both as expanded items
with the fields of the artifact's template.

- Put major issues in `Questions` with `Status: open`. Explain each issue in
  detail, describe why it matters, provide several concrete options the human
  can adopt, and recommend one. The artifact must not be accepted while any
  question is `open`.
- When the human resolves a question, set its `Status` to `settled`, add its
  `Resolution`, and keep it in place. Carry the answer into the items it
  changes, such as acceptance criteria, decisions, key decisions, or done
  conditions, and cite the question's ID there.
- Put minor issues in `Assumptions`. Record a best-effort choice so the human
  can review it without blocking acceptance. Replace a disputed assumption
  with a new question.

An issue is major when its resolution could materially change the artifact's
outcome, scope, approach, risk, or acceptance criteria. Other ambiguity is
minor unless the human marks it as blocking.

This rule applies to every artifact that requires human acceptance, including
the goal, general plan, and step plan. Omit `Questions` or `Assumptions`
when that section would be empty.

### Revision Rules

The `Revisions` section applies only to goals, general plans, and step plans
changed after acceptance.

When modifying an accepted goal, general plan, or step plan:

1. Change its status from `accepted` to `amending`.
2. Create `## Revisions` if absent.
3. Add one subsection for the amendment cycle.
4. Keep all further changes in that cycle in the same subsection.
5. On explicit human acceptance, return the status to `accepted`.

Do not add another revision subsection until a later amendment cycle begins.
Capture `Before` once at the start of the cycle. Accumulate every change made
in that cycle under `Changed`; the human accepts them together.

A status-only lifecycle transition does not require a revision entry.

Metadata edits that preserve meaning may retain accepted status without an
amendment cycle.

Each revision record must have a stable ID and state:

- when the revision was made;
- what the artifact said or required before the revision;
- what changed.

Preserve all earlier revision subsections.

Example:

```markdown
## Revisions

### G-REV1: 2026-06-14T18:30:00+09:00 - Extend timeout scope
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

## Migration

When a manifest is missing or has another schema version, or the workflow dir
or current task uses a layout or artifact form this protocol no longer
describes, follow the [migration notes](../migration/README.md).
