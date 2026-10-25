---
name: dev-workflow-setup
description: >-
  Initialize a local WAIF instance or create or revise its optional shared
  project charter. Also used by dev-draft-goal when workflow setup is needed.
---

# Set Up Workflow

Set up the local workflow library and maintain its optional project charter.

## Procedure

1. Read the request, workspace instructions,
   [workflow.md](../../protocol/workflow.md), and
   [waif-helper.md](../../protocol/waif-helper.md). Resolve the source workspace
   before accessing its local library; read the existing charter when present.
2. Preserve an existing library entry, including a broken link. Report access
   failures for repair rather than replacing it or choosing a new setup mode.
3. If the library is missing, initialize it from the workspace root:
   - In Git projects, identify the main worktree from the first record of
     `git worktree list --porcelain -z`.
   - In a linked worktree, present the main worktree's `.waif/library` and ask
     for linked or independent setup. Run `waif init --link <ROOT_LIBRARY_PATH>`
     or `waif init` accordingly. If the root library is unavailable, explain
     that linked setup requires root initialization or repair; independent
     setup remains available.
   - In the main worktree or a non-Git project, run `waif init`.
4. Read the resulting library charter when present. Create or revise
   `library/charter.md` only when requested. Preserve unmentioned rules; use
   concise Markdown without lifecycle metadata or task state. Do not duplicate
   the workflow protocol.
5. Present the library path and any charter changes for review.

Apart from helper setup and requested charter edits, change nothing. Do not
create or select tasks, modify source files, stage, or commit.
