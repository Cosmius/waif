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

Recognize `Workflow dir:` or `workflow_dir:` settings. Resolve a relative path
from the workspace dir.

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
