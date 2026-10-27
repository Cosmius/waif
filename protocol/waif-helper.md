# Waif Helper Usage

This document is for AI agents using Waif skills.

## Rules

Use `waif` for deterministic workflow queries when available.
Read the command output directly; it is intended to be self-describing.

Run task-context commands from the source workspace root. The helper uses
its invocation directory and does not discover Git worktrees:

```sh
waif <COMMAND> [ARGUMENTS]
```

Discover commands with `waif --help` or `waif <COMMAND> --help`.

## Commands

- `waif init` creates an independent `.waif/library/tasks` directory.
- `waif init --link <ROOT_LIBRARY_PATH>` creates a local library symlink to
  the supplied library's resolved absolute path, requiring accessible `tasks/`.
  Missing or broken supplied libraries fail with root initialization/repair
  guidance. Either init form skips any existing local library entry, even a
  broken link, without checking mode or target. Setup does not select a task
  or change source branches; partial I/O failures are not rolled back.
- `waif inspect` reports current goal and plan context, mechanical readiness,
  observed Git facts, and the next action without mutation, including local
  workflow and library paths. Readiness does not establish human approval.
  Mechanically blocked plans can be inspected successfully; BAD artifacts
  and read/query failures fail. Missing setup or selection is a failure.
- `waif new <TASK_SLUG>` creates and selects the next date-sequenced task.
  The slug uses lowercase letters, digits, and hyphens. It requires explicit
  setup first and never initializes a workflow.
  Invalid input and collisions select nothing.
- `waif switch <TASK_NAME>` selects an existing direct task child by complete
  directory name in `library/tasks/`, writing only the local `current_task`
  pointer with target `library/tasks/<TASK_NAME>`. It does not read or validate
  the old pointer. Rejected task targets leave it unchanged; source branches
  are unchanged.
- `waif check [PATH]` checks one artifact, a task directory, or the current
  task. It reports all findings, then `GOOD`, `INCOMPLETE`, or `BAD`. The first
  two exit 0; `BAD` exits 1. Warnings do not affect classification. A macro in
  metadata skips only that value's format check. With no PATH it uses the
  local instance's current task; explicit PATH is caller-relative and needs
  no configured instance or selection.
- `waif fill <PATH>` expands known plain-text macros. `__NOW__` uses one local
  RFC 3339 timestamp; immediately enclosing backticks exclude an occurrence.
  Recognized artifacts are checked before overwrite. A `BAD` candidate is
  saved separately and leaves the target unchanged. Unrecognized files skip
  checking and recovery copies. No occurrence leaves the file unchanged.
  PATH is caller-relative and needs no configured instance or selection.
- `waif goal accept` requires explicit human approval, a `GOOD` drafting or
  amending goal, and no open questions. It changes only `Status` and `Updated`;
  failed preconditions leave the goal unchanged.
- `waif plan accept` requires explicit human approval, a GOOD plan without
  open questions, and a GOOD accepted goal. It establishes the recorded branch
  before changing only Status and Updated; accepted plans retain every byte.
  Assess untracked files before acceptance. After failure or interruption,
  inspect branch, HEAD, and plan metadata, then repair under the existing
  approval while preserving unrelated edits and branch history. Review
  changed semantic scope with the human.

Ordinary task access preserves nominal library and task paths and reports
native filesystem failures. A low-level mutation failure may leave partial
state and is not rolled back. Task selection is local; edits to shared tasks
are visible through every linked instance.
