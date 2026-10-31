# Design Goals

## Single Power User

`waif` expects a single power user working on a task at a time. The user is
responsible for preventing concurrent access to the same task, for example
by not running several agents or helper commands against one task at once.
Concurrency safety is not a goal: commands read, check, and rewrite
artifacts without locking, and concurrent changes may be lost or produce
inconsistent state.

## Deterministic Work With Reasonable Expectations

The helper performs most deterministic workflow operations, such as
creating tasks, checking artifacts, and changing lifecycle state. It
assumes reasonable inputs and a reasonably healthy workspace and
repository.

`waif` is meant to be used together with an AI agent. Handling every minor
edge case inside the helper would cost more than it is worth, so the helper
covers the expected paths, refuses clearly when its preconditions are not
met, and reports enough context to diagnose a failure. When something goes
wrong, such as an interrupted operation or an artifact left inconsistent
with the repository, ask the AI agent to repair it.
