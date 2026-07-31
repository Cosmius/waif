# Waif Helper Usage

This document is for AI agents using Waif skills.

## Rule

Use `waif` for deterministic workflow queries when available.
Read the command output directly; it is intended to be self-describing.

Run commands from the workspace directory:

```sh
waif <subcommand-and-flags>
```

## Subcommand

### Check task artifacts:

```sh
waif check [PATH]
```

With a file path, check that artifact. With a directory path, check task
artifacts in that task directory and its immediate step directories. Without
a path, check those files in the current task. Recognized task files are
`goal.md`, `plan.md`, and `review.md`; recognized step files are `step.md` and
`reviewN.md` where `N` contains only decimal digits.

Each selected artifact receives its applicable structural checks. Files named
exactly `goal.md` also receive goal-specific validation.

The command reports all discovered errors and warnings for every selected
artifact before printing the aggregate summary. Errors invalidate an artifact
and make the command exit with status 1. Warnings, including warnings for
empty sections, are reported but do not invalidate an otherwise conforming
artifact. Raw HTML blocks are unsupported; heading-like lines inside them are
interpreted as artifact headings.

### Create a new workflow task:

```sh
waif new <task-slug>
```

The task slug must be lowercase letters, numbers, and hyphens.

### Inspect the current workflow task:

```sh
waif inspect
```
