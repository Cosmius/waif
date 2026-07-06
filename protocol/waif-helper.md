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

### Inspect the current workflow task:

```sh
waif inspect
```
