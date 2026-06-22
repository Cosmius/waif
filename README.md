# Waif: AI-Assisted Development Workflow

Waif is a file-backed workflow for using an AI agent on one development task.
It turns a coding conversation into a small set of durable Markdown artifacts:
a goal, a plan, step plans, step reviews, and a final review.

The point is not to give the AI more hidden state. The point is to help humans
understand and review what the AI is doing. Waif makes the agent write down its
interpretation of the goal, its design plan, the intended implementation
slices, the review evidence, and the completion decision.

Waif stands for AI-Assisted Development Workflow, formed from a permutation of
the capital letters in `AI-assisted development WorkFlow`.

## Caution

Due to the inherent uncertainty of AI outputs, use Waif at your own risk.
This skill set has been tested with Codex GPT-5.5 Medium. Other capable enough
AI models should also work, but they are not tested.

## Purpose

AI-assisted development can produce useful code quickly, but that speed creates
a review problem. A human may be handed a large diff without a stable record of
the intended behavior, design choices, review scope, or validation evidence.

Waif adds structure around that work:

- agree on the product goal before technical planning;
- agree on the technical plan before branching;
- split implementation into reviewable steps;
- review uncommitted source changes before the human stages them;
- audit the whole branch before calling the task complete.

Use Waif for mid-sized development tasks: changes large enough to benefit from
planning, traceability, and multiple commits, but not so large that they would
span hundreds of steps.

For trivial edits, direct execution or an agent's normal plan mode is usually
lighter. For very large projects, use a larger project-management process and
treat Waif as too small for the job.

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
stages the source changes they accept before the agent commits them.

**Step-sized implementation.** A task is implemented as one or more coherent
steps. Each step should be small enough to review as a focused unit and commit
as a meaningful source change.

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
  -> reviewed, human-staged, committed step
  -> repeat step planning and implementation as needed
dev-final-review
  -> final task review
```

Each skill owns one stage and writes the artifact for that stage. Later skills
read the accepted artifacts from earlier stages instead of relying on chat
memory.

## Usage

Start in the repository where the source change should happen.

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
agent verifies the staged diff and commits that step.

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

For a multi-person project, `dev-conclude-task` can optionally create a
self-contained teammate handoff after the final review. It is usually of
little value for a single-person project and is not part of the normal
workflow. The user decides whether to stage and commit the document:

```text
Use dev-conclude-task to document the completed task for other contributors.
```

## Workflow Directory

Task artifacts live in the workflow directory located at
`<workspace-dir>/.waif`.

`dev-draft-goal` is the only skill that establishes the workflow directory.
Later skills expect the workflow directory and `tasks/current` symlink to
already exist.

When the workflow directory is initialized as its own Git repository, that Git
repository is only a convenience for humans to audit artifact changes. Waif
does not depend on that repository structure, and its history is not used as
workflow protocol state.

## Repository Contents

- `protocol/workflow.md` is the canonical workflow specification.
- `skills/dev-draft-goal/` drafts or revises the task goal.
- `skills/dev-draft-plan/` drafts or revises the technical plan.
- `skills/dev-plan-step/` drafts one implementation step.
- `skills/dev-implement-step/` implements, reviews, and commits one step.
- `skills/dev-final-review/` performs the task-wide final review.
- `skills/dev-conclude-task/` creates the self-contained task handoff.
