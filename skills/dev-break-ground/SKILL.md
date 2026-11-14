---
name: dev-break-ground
description: >-
  Run an accepted WAIF plan in autonomous mode, step by step with subagents,
  for whole-task review. Use only when the human explicitly invokes it or
  clearly asks to start autonomous mode; never on the agent's own judgement.
---

# Break Ground

The human reviews whole tasks instead of individual steps. The main session
acts as the Commissioner and works through an accepted plan step by step with
subagents, without stopping at each step gate. Each step is still committed
separately.

## Starting the Mode

- Autonomous mode starts only when the human clearly asks, in their own
  words, to begin it for the current task, or when the Commissioner asks
  whether to begin it, naming the task and its accepted plan, and the human
  confirms. No fixed phrase starts it, and it is never inferred from an
  acceptance, approval or other short reply; when unsure, ask.
- The mode lasts only for the session in which it started. It is not
  recorded in any artifact; a new session starts it again only as above.
- Any intervention by the human ends the mode at once; a pure status check
  or request for advice is not an intervention. The Commissioner commits
  nothing further until the human starts the mode again.
- The goal and the general plan are always accepted by the human in the
  normal way before the mode can start, and the mandate must exist.
- While the mode is on, the Commissioner's decisions count as the human's
  approval wherever the workflow requires it within the mandate and the
  accepted goal and plan: accepting step plans, accepting implementations,
  staging the reviewed changes, committing each step, and amending the
  plan where this skill allows. This releases the commit authorization
  rules for those commits only.

## The Mandate

- `<workflow-dir>/library/commissioner-mandate.md` is the human's written
  grant of authority to the Commissioner: what it may decide and have done
  on the human's behalf, and what it must always bring back. The human
  writes or explicitly accepts it; no agent creates or changes it unless
  the human asks.
- It applies only in autonomous mode. The Commissioner reads it when the
  mode starts and gives each subagent the parts that bind it; outside the
  mode no skill reads or applies it.
- Where it conflicts with the charter or with this skill, the mandate
  prevails.
- Without the file the mode cannot start; the agent then creates it with
  the human, following `mandate-setup.md` beside this skill.
- Anything the mandate does not clearly cover goes back to the human; when
  the Commissioner is unsure whether something is covered, it is not.

## Roles

- **Commissioner** (the main session) represents the human and runs the
  workflow: it starts each subagent, gives it its inputs, accepts or returns
  its work, stages and commits each step, and keeps the working log. It
  resolves disagreements and may overrule an objection, but cannot widen the
  goal, amends the plan only where this skill allows, and cannot authorise
  anything that requires escalation. It never edits source code, step
  artifacts or review artifacts: every source change goes through the
  Contractor and is checked by an Inspector. It may glance at code, but
  sends any question needing more than a file or two to a Clerk, to keep
  its context on running the task.
- **Architect** (one subagent per step) drafts the next `step.md` with the
  installed `dev-plan-step` skill and revises it on the Commissioner's
  feedback. It records what its investigation found (versions, APIs,
  observed behaviour, probe results) in the step's `Context`, so the
  Contractor does not repeat it.
- **Contractor** (one subagent per step) implements the accepted step, adds
  tests, and runs the step's validation. It needs the Commissioner's
  approval for any significant deviation from the step plan. It does not
  stage or commit.
- **Inspector** (a new subagent for every review round) reviews the
  current step with the installed `dev-review` skill. It never receives
  other agents' conversations, explanations or earlier review conclusions,
  cannot change acceptance criteria, and cannot pass its own findings.
- **Senior Inspector** (a new subagent for every task review) reviews the
  whole task with `dev-review`, under the Inspector's rules.
- **Clerk** (a new subagent for every request) finds the truth about the
  code for the Commissioner: whether a claimed defect is real, how something
  behaves, where something is. It gets the question without the
  Commissioner's view of the answer, answers with file and line evidence,
  may run commands that change nothing, never edits a file, proposes no fix
  and starts no agents.
- **Surveyor** (optional, a new subagent for every request) operates a
  running user interface that must be seen to be checked. Any role that
  needs one starts its own Surveyor rather than operating the interface
  itself, and reports the run to the Commissioner. It takes one written
  request free of other agents' conclusions, returns figures and
  observations as text, cites screenshots by path, never edits source,
  starts no agents, restores everything it changed, and uses a device only
  when no other Surveyor does.
- When starting a subagent, the Commissioner tells it the rules of this
  skill it must follow. It prefers writing a request to a file in the task
  directory and asking the subagent to read it over sending the text
  directly, so that rules common to every subagent are written once, at
  the start of the task.
- When agents disagree, the Commissioner rules only after each party has
  handed in its own answer. Agents may challenge the Commissioner's
  decisions but follow its final ruling.

Each role runs on the model and reasoning effort the mandate records. The
human starts the Commissioner's session, so its entry is a reference for
the human. Whoever starts a subagent passes both model and effort; no agent
sets or changes its own.

## Context and Handover

- The Commissioner keeps one continuous context for the whole run. The
  Inspector, Senior Inspector, Clerk and Surveyor start fresh for every
  round or request.
- The Architect and Contractor each keep one context for one step: the
  Commissioner resumes the same pair for the step's plan, implementation,
  revisions, fix rounds and amendments.
- When a step is committed, each writes a handover note in the task
  directory, `handover-architect.md` and `handover-contractor.md`, after
  reading the next plan item, replacing its previous note and keeping what
  still holds. Outline: code map, conventions and pitfalls, tools and
  recipes, open risks and threads, and what was tried and did not work.
- The next step's Architect and Contractor start new, briefed with their
  note, the step files and the working log.

## Working Through the Plan

- Each step passes through these stages, in this order:
  - **Plan:** the Architect drafts `step.md`; the Commissioner reviews it,
    returns it for revision if needed, and accepts it with
    `waif step accept` only if it has no open questions, covers only
    existing `pending` plan items, and needs no change to the goal or plan.
  - **Implement:** the Contractor implements and validates the step.
  - **Review:** a fresh Inspector writes the next `reviewN.md` itself, at
    the path the Commissioner names; the Commissioner checks it with
    `waif check` before ruling on it.
  - **Resolve:** if the review passes, the Commissioner stages exactly the
    reviewed and validated changes and commits them with
    `waif step commit`. Otherwise the Contractor fixes the defects and
    revalidates, and a new Inspector reviews again.
  - **Next:** the Commissioner starts the next step without waiting, until
    every plan item is `done`.
- One step is one commit. Nobody amends, rebases or squashes the task's
  commits, so the human can review the task commit by commit.
- An override never removes a defect or turns a failed review into a passed
  one: the review stays failed in its `reviewN.md`, and the override is
  listed in the task summary.
- The Commissioner may amend the plan itself only when:
  - it judges the change a clarification that stays within the plan;
  - it judges that the plan cannot be finished as written, and the change
    is small and stays within the goal; such a change only makes the plan
    finishable and never improves its quality;
  - it adds fix plan items after a task review that does not end the mode.
- Before its first amendment only, the Commissioner saves an exact copy of
  the accepted plan as `plan-as-accepted.md` in the task directory and
  never changes it; later amendments save no further copies.
- The Commissioner keeps a freeform `worklog.md` in the task directory,
  recording what the mandate requires.

## Escalation and Safety

- The Commissioner stops and asks the human, leaving the current step's
  changes uncommitted and saying why it stopped, what has been done so far
  and what decision it needs, when:
  - the mandate says the matter must come back;
  - a step would change the goal, the plan, the charter, the mandate or
    any accepted artifact, other than an allowed plan amendment;
  - a step has an open question or a defect the Commissioner cannot resolve
    or reasonably overrule;
  - further fix-and-review rounds seem unlikely to resolve a failing
    validation or blocking defect; it decides after each failed round;
  - work would go outside the plan, such as unrelated files or dependencies
    the plan does not name;
  - an action is destructive or outward-facing, such as pushing,
    publishing, deleting data or changing settings outside the repository.
- No agent pushes, rewrites history, deletes files outside the task's
  changes, or writes outside the repository, its library and temporary
  directories. Tool permissions enforce this where the agent supports it;
  only the human can lift such a block.
- The human can pause the task at any time. The Commissioner then finishes
  or abandons the current step, leaving no partial commit, and stops.

## Task Review

- When every plan item is `done`, the Commissioner has a new Senior
  Inspector review the whole task with `dev-review`, writing the task's
  next `reviews/reviewN.md`.
- Task reviews repeat until the mode ends:
  - if a task review passes, or yields no fix item, the mode ends;
  - otherwise the Commissioner adds plan items for the findings that
    qualify, works through them as further steps, and another task review
    follows by another new Senior Inspector.
- A finding gets a fix item only when a Clerk has confirmed it; it matters
  (it breaks an acceptance criterion or a charter rule, high or medium
  severity); it needs no decision from the human (it stays within the goal
  and the plan's decisions, involves no trade-off between two reasonable
  behaviours, and touches no escalation case); and it fits one coherent
  step without redesigning earlier steps. Qualifying findings are grouped
  into as few steps as possible.
- When the mode ends, the Commissioner presents a task summary: the commits
  in order, what each step did, review results per step, each task review's
  verdict and findings, every objection and override from the working log,
  and anything deferred.
