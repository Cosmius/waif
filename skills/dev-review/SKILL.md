---
name: dev-review
description: >-
  Review a WAIF step, a whole task, or given targets and write one new checked
  review record artifact. Use only when explicitly invoked, the user requests
  WAIF, or work continues an existing WAIF task. Charter work alone does not
  count as task context.
---

# Review Development Work

Judge the change from the `Reviewed` REF against the `Reviewed against` REF,
and leave one new, immutable review record artifact that a human can act
on.

## Context

The review works within the workspace, local library, and task that
[workflow.md](../../protocol/workflow.md) resolves, and within the project
charter when one exists. Missing or unusable required context ends the
review with a report instead of a record artifact.

## Inputs

Each input comes from the prompt, or else takes its default:

- Target: by default the current step, the step directory with the greatest
  number.
- `Reviewed`: by default `<HEAD>-WORKING` for a step, and
  `<HEAD> (HEAD, <task branch>)` for a task.
- `Reviewed against`: by default `<HEAD> (HEAD, <task branch>)` for a step,
  and for a task the plan's `Base commit` with its `Base branch` in
  parentheses.
- Record artifact path: by default the next gapless `reviewN.md` in the step
  directory, or in the task's `reviews/` directory.
- Validation: results given by the prompt; otherwise the reviewer's own
  runs.

A target is a step, the whole task, or targets given directly. A step
supplies its done conditions; the task supplies the goal's acceptance
criteria and the plan items. An existing record artifact path is never
reused.

## Evidence

The judgement rests on the reviewer's own reading of:

- the goal, plan, and step that apply, and the repository instructions;
- the diff between the two REFs, including untracked files for a `-WORKING`
  REF;
- validation results, run by the reviewer when missing or in doubt.

Earlier record artifacts of the same target, the step's other `reviewN.md`
files or the earlier `reviews/reviewN.md` files, are read only once the
reviewer's own assessment and findings are formed. They serve finding
numbers and `Review History`, never the judgement.

## What the Review Establishes

- Whether each target is met, or done, with its evidence.
- Every problem worth the human's attention, as a finding. Correctness,
  regressions, security, data loss, concurrency, compatibility, and missing
  tests come first; files and lines are cited where possible.
- For a task review, also cross-step regressions, inconsistent behaviour,
  missing integration coverage, unrelated changes, and divergence from the
  goal or plan.
- Workflow and procedure issues are context, not findings, unless they
  explain a substantive code, behaviour, validation, or integration risk.

The review is evidence for the human, not an automatic acceptance gate.

## The Record Artifact

A `reviews/reviewN.md` path follows the
[task review template](../../protocol/template/task-review.md); any other
path follows the [review template](../../protocol/template/review.md). The
finished record artifact is `GOOD` under `waif check <RECORD_PATH>` run from
the workspace root, with every macro expanded by `waif fill`. `waif check`
covers the structure and the `Date`, `Verdict`, and REF values; the record
artifact also meets these conventions, which it does not check:

- `Reviewer` is `independent subagent` only for a fresh session that
  inherited no conversation history; otherwise `current session`.
- The verdict is the reviewer's overall judgement from the whole context.
  Failed validation and unmet or undone targets are recorded as findings.
  With no findings, or only `low` ones, the reviewer may decide `pass`; any
  finding above `low` makes it `concerns`.
- Each finding has `Severity`, `Location`, `Problem`, `Impact`, and
  `Recommendation`. Severity reflects the problem's impact, and findings run
  from the most severe.
- A finding still open from the previous review keeps its number under the
  new review prefix. A new finding takes the next number never used for the
  same target, and a fixed finding's number is never reused.
- Assessment entries are keyed by the target's ID, such as `S1-DC1`,
  `G-AC1`, or `P1`, and read `met` or `unmet`, or `done` or `not done`, with
  evidence. A plan item names the steps and commits that delivered it, from
  each done step's `Source commit`; an aborted step delivers nothing. A
  target given directly has no ID, so its entry restates the target before
  its result.
- `Review History` appears from the second review on, with one entry per
  earlier record artifact, keyed by its path such as `review1.md`, giving
  its verdict and what became of its findings.
- `Summary` opens with a very concise paragraph delivering the verdict and
  its reason.
- `__LAST_COMMIT__` resolves to the workspace's `HEAD`; a REF naming another
  commit is written out in place of the macros. In a task review,
  `Reviewed against` comes from the plan.

The review ends by reporting the verdict and the record artifact path.

## Waivers

When the prompt asks to record a waiver, the outcome is a change to an
existing record artifact instead of a new one. The human may waive a finding
only by giving a reason, which the finding gains as `Waived`, however weak it
seems. A reviewer who disagrees appends a concise `Counter-opinion` after it.
Nothing else in the record artifact changes, the recorded verdict included,
and it stays `GOOD` under `waif check`.

## Constraints

- The review stands without the implementing session's history,
  explanations, or suggested verdict.
- No record artifact is overwritten or edited, except to record a waiver.
- No source file is modified, staged, or committed.

## Done When

- One new `GOOD` record artifact exists at the record artifact path, or the
  waived record artifact is `GOOD`.
- The verdict or waiver result and the record artifact path are reported.
