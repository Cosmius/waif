# Waif Helper Usage

This document is for AI agents using Waif skills.

## Rules

Use `waif` for deterministic workflow queries when available.
Read the command output directly; it is intended to be self-describing.

Run from the workspace directory:

```sh
waif <COMMAND> [ARGUMENTS]
```

Discover commands with `waif --help` or `waif <COMMAND> --help`.

## Commands

- `waif inspect` reports current goal-stage context and the next mechanical
  action without mutation. It succeeds only for usable context.
- `waif new <TASK_SLUG>` creates and selects the next date-sequenced task.
  The slug uses lowercase letters, digits, and hyphens. It initializes a
  missing workflow but never changes existing repository metadata. Invalid
  input and collisions select nothing.
- `waif switch <TASK_NAME>` selects an existing direct task child by complete
  directory name. Rejected targets leave `current` unchanged.
- `waif check [PATH]` checks one artifact, a task directory, or the current
  task. It reports all findings, then `GOOD`, `INCOMPLETE`, or `BAD`. The first
  two exit 0; `BAD` exits 1. Warnings do not affect classification. A macro in
  metadata skips only that value's format check.
- `waif fill <PATH>` expands known plain-text macros. `__NOW__` uses one local
  RFC 3339 timestamp; immediately enclosing backticks exclude an occurrence.
  Recognized artifacts are checked before overwrite. A `BAD` candidate is
  saved separately and leaves the target unchanged. Unrecognized files skip
  checking and recovery copies. No occurrence leaves the file unchanged.
- `waif goal accept` requires explicit human approval, a `GOOD` drafting or
  amending goal, and no open questions. It changes only `Status` and `Updated`;
  failed preconditions leave the goal unchanged.

Mutations validate preconditions first. A low-level overwrite failure is
reported, may leave a partial target, and is not rolled back.
