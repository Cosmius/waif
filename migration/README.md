# Migration notes

The library and each task have a manifest holding their schema version:
`library/waif.toml` for the library and `task.toml` in each task dir. Read
`schema_version` from each manifest to find the version to migrate from. A
missing manifest means the library or task predates versioning: a workflow
dir without `library/` is v0.1, and any other library or task without a
manifest is v0.2.

Each note is named after the `MAJOR.MINOR` schema version it migrates from,
`v<schema_version>.md`, such as `v0.2.md`. A note explains how to read the
layout and forms written for its version; the current protocol and templates
define what to write. Skip notes for versions older than the ones you find.
A manifest newer than the current schema was written by a newer WAIF; update
WAIF instead of migrating.

- Migrate only after the user agrees; never migrate automatically.
- Migrate the library and each task separately, since each has its own
  manifest. Migrate each as a whole, applying the note for its version and
  each later note in version order.
- Afterwards, set `schema_version = "<current>"` in the migrated manifest,
  using the `Schema version` in the Debug Information of the packaged
  [helper guide](../protocol/waif-helper.md). Create the manifest when it
  was missing; keep any other keys it holds.
- Keep the meaning of every artifact: IDs, statuses, and history stay as they
  were unless a note says otherwise.
