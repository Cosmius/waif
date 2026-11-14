# Waif: AI-Assisted Development Workflow

Waif is a file-backed workflow for using an AI agent on one development task.
It turns a coding conversation into a small set of durable Markdown artifacts:
a goal, a plan, step plans, step reviews, and a final review.

The aim is not to give the AI more hidden state, but to help humans understand
and review what it is doing. Waif makes the agent write down its interpretation
of the goal, its design plan, the intended implementation slices, the review
evidence, and the completion decision.

The name is an anagram of the capital letters in
`AI-assisted development WorkFlow`.

## Caution

AI output is inherently non-deterministic. Use Waif at your own risk.

### Verified Agents and Models

Waif is known to work with these agent and model combinations:

| Agent       | Model       |
| ----------- | ----------- |
| Claude Code | Opus 5.5    |
| Codex       | GPT-6.1 Sol |
| Codex       | GPT-6 Astra |
| Codex       | GPT-6 Luna  |
| Codex       | GPT-5.6 Sol |
| Codex       | GPT-5.5     |

Other sufficiently capable agents and models may also work, but they have not
been tested.

### Known Problematic Agents and Models

Waif is known to work poorly with these combinations:

| Agent | Model         |
| ----- | ------------- |
| Codex | GPT-6.1 Terra |

## Purpose

AI-assisted development can produce useful code quickly, but that speed creates
a review problem. A human may be handed a large diff without a stable record of
the intended behaviour, design choices, review scope, or validation evidence.

Waif adds structure around that work:

- agree on the product goal before technical planning;
- agree on the technical plan before branching;
- split implementation into reviewable steps;
- review uncommitted source changes before the human stages them;
- audit the whole branch before calling the task complete.

Waif suits mid-sized development tasks: changes large enough to benefit from
planning, traceability, and multiple commits, but not so large that they would
span hundreds of steps.

For trivial edits, direct execution or an agent's built-in plan mode is
lighter. Very large projects need a fuller project-management process; Waif is
too small for them.

## Design

Waif is intentionally simple. It does not impose a programming language,
framework, issue tracker, or repository layout. It sits beside an existing
software project as a process layer for one task.

The workflow has four core design ideas.

**Human-readable task artifacts.** The artifacts are Markdown files written for
human inspection. They record what the task is, why the agent is taking a
particular approach, what changed, what was reviewed, and what evidence
supports completion.

**Explicit acceptance gates.** The agent stops for human acceptance at the
goal, plan, and step-plan stages. During implementation, the human manually
stages the source changes they accept before the agent commits them. A step
that changes no source, such as one that only runs tests, closes without a
commit once the human accepts it.

**Step-sized implementation.** A task is implemented as one or more coherent
steps. Each step should be small enough to review as a focused unit and commit
as a meaningful source change, or, when it changes no source, to verify as one
unit.

**Task-wide final review.** The workflow ends with a final review of the whole
branch against the accepted goal, accepted plan, completed steps, prior
reviews, and validation evidence.

## Workflow

The normal lifecycle is:

```text
dev-draft-goal
  -> accepted goal
dev-draft-plan
  -> accepted plan and task branch
dev-plan-step
  -> accepted step plan
dev-implement-step
  -> reviewed, human-staged, committed step, or validated step without commit
  -> repeat step planning and implementation as needed
dev-final-review
  -> final task review
```

Each skill owns one stage and writes that stage's artifact. Later skills read
the accepted artifacts from earlier stages rather than relying on chat
history.

## Installation

The `waif` binary embeds the skills and protocol documents and builds every
distributable package. Install it from the repository root:

```sh
cargo install --path .
```

Then build a package for your agent:

```sh
waif package --target claude-plugin
waif package --target codex-plugin
waif package --target pi-package
waif package --target skill-zip
```

Each command writes its package under `dist/` in the current directory,
replacing any previous build of that target, and prints how to install it.
To trace which build produced a package, the packaged
`protocol/waif-helper.md` ends with the version, Git commit, and SHA-1 of the
`waif` binary that built it. The commit has a `-dirty` suffix when the binary
was built from uncommitted changes, and is `unknown` outside a Git checkout.

## Usage

Start in the repository where the source change should happen.

Waif skills apply only when at least one of these holds:

1. You explicitly invoke a skill.
2. You explicitly request Waif.
3. You are already working within a Waif task.

Otherwise the agent works as usual:

- Creating or amending the charter does not by itself start a task.
- Installing Waif, or a task's size, never triggers it.
- Standalone small fixes use direct execution unless you request Waif or invoke
  a skill.

Within an existing Waif task, requests such as "plan the next step" select the
matching skill without naming it. Every acceptance gate still applies,
including human staging before source commits.

The examples below name skills to make each stage clear.

Optionally, initialise the workflow and set project rules:

```text
Use dev-workflow-setup to require tests for every behaviour change.
```

Draft the goal:

```text
Use dev-draft-goal to implement password reset by email.
```

Review the generated `goal.md`. Ask for revisions until the outcome is right,
then accept it:

```text
accept the goal
```

Draft and accept the plan:

```text
Use dev-draft-plan to plan the accepted goal.
accept the plan
```

Plan and accept the next implementation step:

```text
Use dev-plan-step to plan the next step.
accept the step
```

Implement the accepted step:

```text
Use dev-implement-step to implement the accepted step.
```

After implementation review, manually stage the source changes you accept. The
agent verifies the staged diff and commits that step. A step that changes no
source skips the review and closes without a commit when you accept its
validation results.

Repeat step planning and implementation until the task is complete:

```text
Use dev-plan-step to plan the next step.
accept the step
Use dev-implement-step to implement the accepted step.
```

Run the final review when no more implementation steps remain:

```text
Implementation is complete. Use dev-final-review.
```

On a multi-person project, `dev-conclude-task` can create a self-contained
handoff for teammates after the final review. It is outside the normal workflow
and of little value on a solo project. You decide whether to stage and commit
the document:

```text
Use dev-conclude-task to document the completed task for other contributors.
```

## Autonomous Mode

Autonomous mode is an optional way to run an accepted plan, outside the
normal workflow. You accept the goal and the plan as usual; the agent then
works through every step with subagents, commits each step itself, and you
review the task as a whole. It trades your step-by-step approval for your
attention, and costs more than the normal workflow.

The agent acts as your Commissioner within a mandate, a file you write with
it at first use. The mandate records which model runs each role, what the
agent may decide for you, what it must bring back, and its limits. It
outranks the charter and applies only in this mode, so read it carefully
before accepting it.

The mode never starts by itself: no other skill leads to it, and the agent
cannot pick it on its own judgement. Start it explicitly, for each session:

```text
Use dev-break-ground to start autonomous mode for the current task.
```

The agent commits without waiting for you in this mode. Run it in an
isolated environment, such as a container or a separate account, if you can.

## Workflow Directory

Each source workspace has a local `.waif/`:

- `library/charter.md`: optional project rules read by every skill.
- `library/commissioner-mandate.md`: your grant of authority for autonomous
  mode, read only in that mode.
- `library/waif.toml`: the library manifest, holding its schema version.
- `library/tasks/`: task records, shared when `library` is a symlink. Each
  task dir holds a `task.toml` manifest with its schema version.
- `current_task`: local selection pointing to `library/tasks/<task-name>`.

`dev-workflow-setup` initialises the library and maintains the charter;
`dev-draft-goal` runs it first if the library is missing. In a linked worktree,
setup asks whether to share the main worktree's library or create an
independent one. Later stages require an established current task. Selecting a
task never switches source branches.

Removing a linked worktree preserves shared records; removing an independent
instance removes its local library. When moving or removing the main project,
keep its library and repair the links that depend on it. Exclude `.waif/` from
source Git.

See [helper usage](protocol/waif-helper.md) for CLI commands.

## Repository Contents

- `src/` and `tests/`: the `waif` helper CLI, which also builds packages.
- `protocol/workflow.md`: the canonical workflow specification.
- `protocol/waif-helper.md`: deterministic CLI commands for agents.
- `protocol/template/`: artifact templates that the skills start from.
- `skills/dev-workflow-setup/`: initialises Waif and maintains project rules.
- `skills/dev-draft-goal/`: drafts or revises the task goal.
- `skills/dev-draft-plan/`: drafts or revises the technical plan.
- `skills/dev-plan-step/`: plans one implementation step.
- `skills/dev-implement-step/`: implements and commits one step with a
  `dev-review` record.
- `skills/dev-review/`: reviews a step or the whole task and writes the review
  record.
- `skills/dev-final-review/`: runs the task-wide review.
- `skills/dev-conclude-task/`: writes the optional teammate handoff.
- `skills/dev-break-ground/`: runs an accepted plan in autonomous mode.
