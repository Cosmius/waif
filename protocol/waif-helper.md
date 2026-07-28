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

With a file path, check that artifact. With a directory path, recursively
check files named `goal.md`, `plan.md`, `step.md`, `reviewN.md`, and
`review.md`. Without a path, check those files in the current task.

At this stage, the command checks only the common artifact envelope: the
title, metadata block, and level-two sections. Except for locating artifact
headings and recognizing fenced code blocks, it treats every section body as
opaque prose. Raw HTML blocks are not supported; heading-like lines inside
them are interpreted as artifact headings. The command does not yet apply
artifact-specific contracts or validate structured sections.

The command reports all envelope errors it can find and exits with status 1
when any artifact has an invalid envelope.

### Create a new workflow task:

```sh
waif new <task-slug>
```

The task slug must be lowercase letters, numbers, and hyphens.

### Inspect the current workflow task:

```sh
waif inspect
```
