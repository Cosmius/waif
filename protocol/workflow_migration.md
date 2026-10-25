# Workflow migration notes for AI agents

Use these notes to recognize the previous schema and work out the migration
for the workspace you find. The current schema is defined by the workflow
protocol and helper reference.

## Previous schema

```text
.waif/
├── .git/                    # Optional workflow repository
├── .gitignore               # Usually ignores /tasks/current
├── charter.md               # Optional project rules
└── tasks/
    ├── YYYYMMDD-N-task-name/
    │   ├── goal.md
    │   ├── plan.md
    │   └── steps/...
    └── current -> ./YYYYMMDD-N-task-name
```

The charter and task records lived directly under `.waif/`. The selected task
was a relative symlink at `.waif/tasks/current`, inside the task collection.
There was no separate library or instance-local selection outside it.

## Migration outcome

Move the charter and task collection into `.waif/library/`. Replace the old
selection with `.waif/current_task -> library/tasks/<task-name>`, preserving
which task was selected, or leaving selection absent if there was none.
Keep task names, artifact bytes, and each task's internal structure unchanged.
The library must contain no current-task pointer.

Keep existing workflow Git metadata and history at `.waif/.git`. Update ignore
rules to exclude `/current_task` while allowing `library/` to be versioned;
account for an old pointer already tracked in the index. Preserve unrelated
ignore rules, pending changes, and workflow-root documents. `ROADMAP.md` and
`deterministic-helper-roadmap.md` are outside this migration.

Inspect the actual state and choose appropriate operations and verification.
Preserve recoverable state before moving records, coordinate paused writers,
and resolve ambiguous or partial layouts without overwriting records. Verify
preservation of the charter, artifacts, selection, and workflow history, and
check that the migrated task is usable by the current helper.

Migrate the main workspace before setting up linked instances. Discover the
source worktrees through source Git metadata, then retain the user's choice
of a shared or independent library and keep each instance's selection local.
Existing setup commands do not migrate records or repair an existing library.
