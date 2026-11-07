# Task Artifact Format

- Status: proposed
- Purpose: Define the structure and section types of workflow artifacts

## Overview

A task artifact is Markdown containing one title, structured metadata,
optional opaque pre-section prose, and level-two sections. An artifact contract
declares its recognized metadata, its known sections and their types, and the
configuration of each itemised section.

## Authoring and Parser Scope

Task artifacts are generated and maintained by `waif`. Humans may review them,
but should request changes through `waif` rather than editing files directly.

The parser supports the canonical Markdown emitted by `waif`; it is not a
general CommonMark parser. It recognizes artifact headings and simple fenced
code blocks using line-oriented rules and preserves original source exactly.
Markdown container interactions such as headings or fences inside list items,
block quotes, or raw HTML are outside the supported format.

Artifact-specific checking accumulates parser and schema diagnostics against a
partially parsed artifact. Independent structural problems can therefore be
reported together instead of stopping at the first parser error.

## Format Rules

This reference defines two kinds of format rule:

- A checked rule is enforced by `waif check`. Breaking it makes the artifact
  invalid.
- A convention is followed by templates, examples, and agents by default.
  Breaking it is acceptable unless the human objects.

In the [Sections](#sections), [Items](#items), [Revisions](#revisions), and
[Questions and Assumptions](#questions-and-assumptions) rules, a rule is
checked unless it is called a convention. Each rule of the items format comes
with a checked example, which shows what checking accepts or rejects, and a
conventional example, which shows the usual form.

## Common Artifact Envelope

An artifact contains, in order:

1. Exactly one level-one heading with a non-empty, single-line title. It begins
   in column one and is the first non-whitespace line.
2. A leading structured metadata block whose entries have the logical form
   `- Key: value`.
3. Optional opaque prose.
4. Zero or more level-two sections.

Whitespace-only lines have no artifact-level meaning. Programmatic changes
preserve existing whitespace and line endings.

Every syntactically valid leading metadata entry is structured. A contract
declares the keys it requires and validates; other keys are accepted. All
top-level metadata keys must be unique and may normally occur in any order.

The first ordinary content or malformed metadata-looking entry ends structured
metadata and begins opaque pre-section prose. It and all following content
before the first level-two section remain prose.

A metadata entry begins with `-` in column one and must have whitespace between
`-` and its key. Extra whitespace around the key, colon, and value is
insignificant. The first colon separates the key from its value, which may
contain further colons.

A metadata value may continue onto indented lines, including an indented fenced
code block. Parsed multiline values are immutable through the structured
artifact API. A programmatic replacement is supported only when the parsed
value and its replacement are both single-line.

## Sections

An artifact contract maps exact level-two section names to two types:

- A prose section contains opaque Markdown. Structured processing does not
  validate, fill, or modify its body.
- An itemised section contains stable-ID items and follows the
  [items rules](#items). Its contract configures the section's ID family; the
  metadata keys its items require, each with a value check or a presence-only
  check; an optional sentinel; and, for plan items only, an opt-out from the
  artifact prefix.

Contract tables show an itemised section's ID form, then any required item
metadata and any sentinel.

Unknown sections are prose. Every section name, known or unknown, is unique.
Known sections have a declared relative order. Optional known sections may be
omitted, and unknown sections do not affect the order comparison.

### Empty Sections

Checked rule: a required itemised section with neither items nor its declared
sentinel is an error. Any other empty section produces a warning but does not
invalidate the artifact.

Convention: an optional section with no content is omitted rather than left
empty.

Checked example: the empty required `Done Conditions` is an error.

```markdown
## Done Conditions

## Context

Coverage is parsed by the step schema.
```

Conventional example: `Changes` has no items, so it is omitted.

```markdown
## Key Decisions

- S1-K1: Read coverage from item metadata.

## Tests

- S1-T1: Add a coverage mismatch test.
```

## Items

An itemised section holds compact items followed by expanded items, or only
its declared sentinel. These rules apply to every itemised section; each
contract adds its sections' configuration.

### Items Only

Checked rule: an itemised section contains only items, or its declared
sentinel. Any other non-whitespace content outside an item, including a
fenced code block, is an error.

Convention: explanation sits inside the item it belongs to.

Checked example: the paragraph between the items is an error.

```markdown
## Risks

- P-R1: The parser rejects old plans.

See the plan for details.

- P-R2: Templates drift from the checker.
```

Conventional example: the explanation sits inside an item.

```markdown
## Risks

- P-R1: The parser rejects old plans. Each error names the line to
        change.
- P-R2: Templates drift from the checker.
```

### Compact Items

Checked rule: a compact item is `- ID: content` starting in column one. Its
indented continuation lines and nested bullets are opaque content, and it
carries no metadata.

Convention: continuation lines align with the first content character after
`ID: `.

Checked example: one accepted item, although its continuation line is not
aligned; the nested `- Owner: parser` line is content, not metadata.

```markdown
- G-AC1: First line of a criterion
  that continues here.
  - Owner: parser
```

Conventional example:

```markdown
- G-AC1: First line of a criterion
         that continues here.
```

### Expanded Items

Checked rule: an expanded item is `### ID: title` with a non-empty title. Its
leading `- Key: value` entries are its metadata, parsed like artifact metadata,
including indented continuation lines. Blank lines before and between these
entries are ignored, so a `- Key: value` line after a blank line is still
metadata, and the opaque body begins at the first line that is neither blank
nor such an entry. The body ends at the next level-three or level-two heading.
Level-four and deeper headings belong to the body. Level-one headings and
level-three headings that are not items are errors, and heading-like text
inside a fenced block is opaque.

Convention: an item that needs its own section is written as an expanded
item. Its metadata starts on the line directly below the heading, and a blank
line separates the metadata from the body.

Checked example: the repeated `Reason` key and the `### Notes` heading are
errors.

```markdown
### P-D2: Storage format
- Reason: speed
- Reason: size

### Notes
```

Accepted, but not conventional: a blank line between the heading and its
metadata.

```markdown
### P-D2: Storage format

- Reason: speed
```

Conventional example:

```markdown
### P-D2: Storage format
- Reason: speed

Body text.

#### Notes

More body text.
```

### Item Order

Checked rule: compact items come before expanded items, and either group may
be absent. A compact-looking line after the first expanded item belongs to
the nearest expanded item above it: it is a metadata entry when it is among
that item's leading `- Key: value` entries, and body text otherwise.

Convention: when a section has both forms, every expanded item has a
placeholder among the compact items, so the compact items list every ID in
numeric order.

Checked example: accepted, although `P-D2` has no placeholder.

```markdown
- P-D1: Keep the parser line-oriented.
- P-D3: Reject unknown section names.

### P-D2: Storage format
```

Conventional example:

```markdown
- P-D1: Keep the parser line-oriented.
- P-D2: Vide infra, storage format.
- P-D3: Reject unknown section names.

### P-D2: Storage format
```

### Item IDs

Checked rule: every item has an ID in its section's family.

Stable-ID numeric suffixes are canonical positive decimal integers from 1
through `2^63 - 1`. Numbers increase within the compact items and within the
expanded items of a section independently. An ID is unique within its group,
and a compact item may share an expanded item's ID only as a
[placeholder](#placeholders). IDs and names are case-sensitive.

Every item ID consists of an artifact item prefix, a section-specific family,
and a final item number. The prefix owns its trailing dash; the family does
not include a leading separator. Goal and plan prefixes are the known literals
`G-` and `P-`. Step prefixes have the shape `S<n>-`, and implementation-review
prefixes have the shape `S<n>-R<m>-`, where every numeric component is
canonical and bounded as above. Plan items opt out of the artifact prefix:
their IDs are `P<number>`, and they do not participate in artifact-prefix
inference.

A checker uses a known prefix when the artifact identity is available before
item validation. Known prefixes must match exactly. Otherwise a step or review
checker uses an unknown prefix shape: the first valid item establishes the
observed prefix, and every later item family in that artifact must use the same
value. Unknown mode establishes internal consistency only. The shared schema
retains each observed numeric component with its source span; artifact-specific
checking compares those observations with titles, filenames, or nominal paths
when that context is available. Prefix inference never crosses artifact
boundaries or uses opaque prose or neighboring files.

Convention: items are numbered in the order they are added, and the number of
a removed item stays unused.

Checked example: `P-D1` after `P-D2` and the `P-R` family in `Decisions` are
errors.

```markdown
## Decisions

- P-D2: Keep the parser line-oriented.
- P-D1: Reject unknown section names.
- P-R3: The parser rejects old plans.
```

Conventional example: `P-D2` was removed, and its number stays unused.

```markdown
## Decisions

- P-D1: Keep the parser line-oriented.
- P-D3: Reject unknown section names.
```

### Placeholders

Checked rule: a compact item may share its ID with an expanded item in the
same section only as a placeholder, whose content starts with `Vide infra` or
`vide infra` followed by the end of the content or a character other than a
letter, digit, or underscore. A placeholder carries no metadata, even in a
section that requires metadata. An expanded item needs no placeholder.

Convention: the placeholder is one line, `Vide infra.` or
`Vide infra, <description>.`, whose description only names the expanded item.

Checked example: accepted; `- P-D2: See below.` in its place would be an
error.

```markdown
- P-D2: vide infra; see the trade-offs below
  - covers storage and caching

### P-D2: Storage format
```

Conventional example:

```markdown
- P-D2: Vide infra, storage format.

### P-D2: Storage format
```

### Item Metadata

Checked rule: a section may require metadata keys, each with a value check or
a presence-only check. Every expanded item and every compact item that is not
a placeholder must carry each required key; a compact item has no metadata, so
in such a section it is an error unless it is a placeholder. Keys are unique
within an item. Keys a section does not require are accepted and preserved
exactly. A value containing a macro such as `__NOW__` skips its value check,
and `waif check` reports the artifact INCOMPLETE, not GOOD, until `waif fill`
replaces the macro.

Convention: required keys come first, followed by any other keys in the order
the section's convention or template lists them.

Checked example: `Plan Items` requires `Status`, so this compact plan item is
an error.

```markdown
- P1: Parse unified items.
```

Conventional example:

```markdown
### P1: Parse unified items
- Status: pending
- Goal criteria: G-AC1
- Outcome: Every itemised section parses with one rule.
- Areas: parser
- Depends on: none
- Validation: `cargo test`
```

### Sentinels

Checked rule: a section may declare a sentinel, text that stands for the whole
body when the section has no items. The body is the sentinel when it holds
only that text, ignoring blank lines around it and spaces or tabs on its line.
The sentinel together with any item is an error, and in a section without a
declared sentinel the same text is ordinary content outside an item. Only the
implementation review's `Findings` declares a sentinel, `No findings.`.

Convention: the sentinel stands alone on its line, without surrounding spaces.

Checked example: the sentinel together with a finding is an error.

```markdown
## Findings

No findings.

### S1-R1-F1: High - Missing regression test
```

Conventional example:

```markdown
## Findings

No findings.
```

## Revisions

The goal, plan, and step contracts declare an optional `Revisions` section
whose items use the artifact's `REV` family.

Checked rule: revisions follow the [items rules](#items) and require no
metadata, so a compact revision is accepted.

Convention: each revision is an expanded item titled with its timestamp and a
short title, with `Before` and `Changed` metadata directly below its heading.

Checked example: accepted, although the revision is compact.

```markdown
- P-REV1: Added API sessions to the timeout work.
```

Conventional example:

```markdown
### P-REV1: 2026-06-14T18:30:00+09:00 - Extend timeout scope
- Before: The plan covered browser sessions only.
- Changed:
  - Added API sessions to the timeout work.
```

## Questions and Assumptions

The goal, plan, and step contracts declare optional `Questions` and
`Assumptions` sections, whose items use the artifact's `Q` and `A` families.
A question records a major issue the human must settle before the artifact is
accepted. An assumption records a minor issue resolved by a best-effort choice
that the human accepts with the artifact.

Checked rule: every question that is not a placeholder carries `Status`
metadata, which is `open` or `settled`. A compact question is therefore an
error unless it is a placeholder. `waif goal accept`, `waif plan accept`, and
`waif step accept` reject an artifact with an `open` question. Assumptions
follow the [items rules](#items) and require no metadata.

Convention: both sections hold only expanded items, so they have no
placeholders. A settled question stays in place; only its `Status` changes.

Checked example: the compact question and `Status: resolved` are errors; the
compact assumption is accepted.

```markdown
## Questions

- P-Q1: Which branch name do linked worktrees use?

### P-Q2: Migration of existing tasks
- Status: resolved

## Assumptions

- P-A1: Timeouts are in seconds.
```

Convention: a question carries `Status`, `Affects`, `Detail`, `Impact`,
`Options`, and `Recommendation` metadata, in that order. `Affects` lists the
IDs whose content depends on the answer, or `none`. A settled question adds
`Resolution`, the chosen option and the human's reason. The items its answer
changes, such as an acceptance criterion, decision, key decision, or done
condition, cite the question's ID.

Conventional example: `P-Q1` is settled, and `P-D3` cites it.

```markdown
## Decisions

- P-D3: Linked worktrees use the task branch name (settles P-Q1).

## Questions

### P-Q1: Branch naming for linked worktrees
- Status: settled
- Affects: P2, P-D3
- Detail: Linked worktrees need a branch, and the plan does not name one.
- Impact: A wrong name breaks `waif plan accept` in linked worktrees.
- Options:
  1. Reuse the task branch name.
  2. Add a worktree suffix.
- Recommendation: 1, because the task owns one branch.
- Resolution: 1, because worktrees never share a task.
```

Convention: an assumption carries `Affects`, `Detail`, `Choice`, and `Risk`
metadata, in that order. `Choice` states the single resolution adopted, and
`Risk` what goes wrong if it is wrong. A disputed assumption is replaced by a
new question, and its ID stays unused.

Conventional example:

```markdown
## Assumptions

### P-A1: Default timeout unit
- Affects: P3
- Detail: The goal gives timeouts without a unit.
- Choice: Timeouts are in seconds.
- Risk: Existing configuration in milliseconds times out 1000 times later.
```

## Goal Contract

A checked file named exactly `goal.md` receives this contract.

Required metadata is:

```text
Status    drafting | accepted | amending | done | aborted
Created   RFC 3339 timestamp with Z or a numeric UTC offset
Updated   RFC 3339 timestamp with Z or a numeric UTC offset
```

Known sections have this relative order:

| Section             | Type                              | Presence |
|---------------------|-----------------------------------|----------|
| Outcome             | prose                             | required |
| Acceptance Criteria | itemised; G-AC<n>                 | required |
| Questions           | itemised; G-Q<n>; requires Status | optional |
| Assumptions         | itemised; G-A<n>                  | optional |
| In Scope            | itemised; G-IN<n>                 | optional |
| Out of Scope        | itemised; G-OUT<n>                | optional |
| Revisions           | itemised; G-REV<n>                | optional |

Every goal itemised section follows the [items rules](#items). Item content and
expanded bodies are opaque. Goal checking does not constrain title content or
semantic quality.

`Questions` and `Assumptions` follow the
[question and assumption rules](#questions-and-assumptions), and `Revisions`
follows the [revision rules](#revisions).

## Plan Contract

A checked file named exactly `plan.md` receives this contract.

Required metadata is:

```text
Status       drafting | accepted | amending | done | aborted
Goal         ./goal.md
Branch       non-empty intended task branch
Base branch  non-empty branch from which the task branch is created
Base commit  full 40-character hexadecimal Git commit hash
Created      RFC 3339 timestamp with Z or a numeric UTC offset
Updated      RFC 3339 timestamp with Z or a numeric UTC offset
```

`Branch` is recorded while the plan is drafting and need not exist yet. The
user may change it before branch creation; after creation it is retained for
the task. `Base branch` and `Base commit` are immutable authoring provenance.
Initial plan drafting requires a checked-out base branch and rejects detached
`HEAD`; that authoring rule does not require `waif check` to inspect repository
state.

Known sections have this relative order:

| Section                  | Type                              | Presence |
|--------------------------|-----------------------------------|----------|
| Technical Summary        | prose                             | required |
| Decisions                | itemised; P-D<n>                  | required |
| Questions                | itemised; P-Q<n>; requires Status | optional |
| Assumptions              | itemised; P-A<n>                  | optional |
| Current System           | prose                             | required |
| Plan Items               | itemised; P<n>; requires Status   | required |
| Risks                    | itemised; P-R<n>                  | required |
| Cross-Cutting Validation | itemised; P-V<n>                  | required |
| Revisions                | itemised; P-REV<n>                | optional |

Every plan itemised section follows the [items rules](#items).

Convention: a decision that needs its own section follows the compact
decisions as `### P-D<number>`, with a `Vide infra` placeholder at its number
among them.

Conventional example:

```markdown
- P-D1: Keep the parser line-oriented.
- P-D2: Vide infra, storage format.

### P-D2: Storage format
```

Checked rule: `Status` is the only plan-item metadata key required and
validated by the schema. It must be unique and accepts `pending` or `done`.
Other keys are accepted and preserved exactly. A compact plan item that is not
a placeholder has no `Status`, so it is an error. Plan-item IDs use `P<number>`
without the artifact prefix and otherwise follow the [ID rules](#item-ids).

Convention: each plan item is an expanded item with `Status` first, followed
by `Goal criteria`, `Outcome`, `Areas`, `Depends on`, and `Validation`.

Checked example: `Status: started` is an error, and `Areas` is kept as
written.

```markdown
### P1: Parse unified items
- Areas: parser
- Status: started
```

Conventional example: each plan item has this authoring form.

```markdown
### P1: Implement a coherent outcome
- Status: pending
- Goal criteria: G-AC1, G-AC2
- Outcome: The technical result.
- Areas: Affected components.
- Depends on: none
- Validation: `cargo test`

Optional explanatory prose.

#### Implementation detail

Deeper headings are valid opaque prose.
```

`Goal criteria`, `Outcome`, `Areas`, `Depends on`, and `Validation` are
semantic author-facing metadata. They may span lines and are preserved
exactly; deterministic checking does not interpret their references or
meaning.

`Questions` and `Assumptions` follow the
[question and assumption rules](#questions-and-assumptions), and `Revisions`
follows the [revision rules](#revisions).

Plan checking validates structure, not the technical quality of decisions,
outcomes, affected areas, risks, dependencies, or validation commands.

## Step Contract

A checked file named exactly `step.md` receives this contract. The portion of
its direct parent's name before the first hyphen supplies the canonical step
identity when it is a positive decimal with at least two digits. The suffix
after that hyphen is opaque to the checker. An empty suffix produces a warning.
If the numeric prefix cannot be resolved, checking warns, skips comparisons
requiring path identity, and still requires one internally consistent `S<n>-`
prefix. Selected paths are interpreted nominally rather than by canonicalizing
symlink targets.

The title is `Step N: <non-empty title>`, where `N` is an unpadded positive
decimal. Its number agrees with a resolvable containing directory and with the
unpadded number in all stable-ID prefixes. Thus step directory
`03-check-steps` uses title `Step 3: Check steps` and IDs such as `S3-C1`.

Structurally required metadata is:

```text
Status         drafting | accepted | amending | done | aborted
Source commit  not-created | full 40-character hexadecimal Git commit hash
Created        RFC 3339 timestamp with Z or a numeric UTC offset
Updated        RFC 3339 timestamp with Z or a numeric UTC offset
```

`Source commit` is `not-created` for `drafting`, `accepted`, and `amending`
steps. A `done` step requires a full commit hash. An `aborted` step accepts
either value. Authoring metadata such as `Plan items`, `Goal criteria`, and
`Estimated non-test/doc changes` follows the validated metadata. It remains
structured metadata, but its values are not checked.

Known sections have this relative order:

| Section              | Type                                               | Presence |
|----------------------|----------------------------------------------------|----------|
| Objective            | prose                                              | required |
| Plan Item Coverage   | itemised; S<n>-PC<m>; requires Plan item, Coverage | required |
| Done Conditions      | itemised; S<n>-DC<m>                               | required |
| Context              | prose                                              | required |
| Key Decisions        | itemised; S<n>-K<m>                                | optional |
| Questions            | itemised; S<n>-Q<m>; requires Status               | optional |
| Assumptions          | itemised; S<n>-A<m>                                | optional |
| Changes              | itemised; S<n>-C<m>                                | optional |
| Tests                | itemised; S<n>-T<m>                                | optional |
| Validation           | itemised; S<n>-V<m>                                | optional |
| Size and Coherence   | prose                                              | optional |
| Risks and Edge Cases | itemised; S<n>-R<m>                                | optional |
| Revisions            | itemised; S<n>-REV<m>                              | optional |

Agents often cite IDs without their artifact prefix while working in another
artifact, so the step's families avoid the letters of the goal's and plan's:
step done conditions use `DC` rather than `AC`, which the goal's acceptance
criteria use, and step decisions are key decisions, `K`, rather than `D`, which
the plan's decisions use. A bare `AC2`, `D2`, `DC2`, or `K2` therefore names
one kind of item.

Every step itemised section follows the [items rules](#items).

Checked rule: every coverage entry that is not a placeholder carries
`Plan item` and `Coverage` metadata. `Coverage` is `partial` or `complete`, and
the `Plan item` of `S<n>-PC<m>` is `P<m>`. Other keys are accepted. Coverage
IDs need not be consecutive. Checking does not compare coverage entries with
the opaque `Plan items` authoring field.

Convention: a coverage entry is an expanded item titled with the part of the
plan item the step consumes, with only `Plan item` and `Coverage` metadata and
an optional prose body.

Checked example: `P4` does not match `PC2` and `most` is not a coverage value,
so both are errors; the extra `Note` key is accepted.

```markdown
### S3-PC2: Parser support
- Plan item: P4
- Coverage: most
- Note: parsing only
```

Conventional example:

```markdown
### S3-PC2: Parser support for gap detection
- Plan item: P2
- Coverage: partial

Covers parsing only; checker diagnostics follow in a later step.
```

`Questions` and `Assumptions` follow the
[question and assumption rules](#questions-and-assumptions), and `Revisions`
follows the [revision rules](#revisions).

Item content and prose are opaque. Numeric bounds, complete identifier
consumption, uniqueness, ordering, section order, and empty sections follow
the common rules.

## Implementation Review Contract

A checked file named `reviewN.md` receives this contract, where `N` is a
positive decimal review number. Its title is `Implementation Review N`. The
filename, title, and review-number component of every stable ID agree.

When the containing directory supplies a step identity, it must agree with
every stable ID. Otherwise checking warns and still requires one internally
consistent `S<n>-R<m>-` prefix. Paths are interpreted nominally. Directory
padding is omitted from IDs: `steps/04-check/review1.md` uses `S4-R1-F1`.

Required metadata is:

```text
Step             ./step.md
Decision         pass | changes-requested
Date             RFC 3339 timestamp with Z or a numeric UTC offset
Reviewer         independent subagent |
                 current session - subagent unavailable
Workspace state  uncommitted
```

Implementation reviews do not have lifecycle `Status` metadata. Known sections
have this relative order:

| Section        | Type                                              | Presence |
|----------------|---------------------------------------------------|----------|
| Findings       | itemised; S<n>-R<m>-F<k>; sentinel `No findings.` | required |
| Scope          | itemised; S<n>-R<m>-SC<k>                         | optional |
| Validation     | itemised; S<n>-R<m>-V<k>                          | optional |
| Residual Risks | itemised; S<n>-R<m>-RR<k>                         | optional |

`Findings` contains either its [sentinel](#sentinels), `No findings.`, or one
or more findings with IDs `S<n>-R<m>-F<k>`.

Checked rule: a `pass` decision requires the sentinel and a
`changes-requested` decision requires at least one finding. Finding content,
metadata values, and bodies are opaque; deterministic checking does not
interpret severity, title, location, or recommendations.

Convention: each finding is an expanded item titled with its severity and a
short description, with `Location`, `Problem`, `Impact`, and `Recommendation`
metadata directly below its heading.

Checked example: with `Decision: pass`, the finding is an error, because
`pass` requires the sentinel.

```markdown
## Findings

### S1-R1-F1: Low - Missing sentinel test
```

Conventional example: a `changes-requested` review lists its findings in this
form.

```markdown
## Findings

### S1-R1-F1: Medium - Missing sentinel test
- Location: `src/parser/tests.rs`
- Problem: No test covers a sentinel followed by a finding.
- Impact: A regression could drop later sections unnoticed.
- Recommendation: Add a parser test for that case.
```

Every review itemised section follows the [items rules](#items). Review IDs,
numeric bounds, and empty sections follow the common rules.

## Source Preservation

Parsing retains source locations for structured content and exact source for
opaque prose. Lifecycle operations locate artifact metadata, and the metadata
of expanded items by section and exact ID; they do not normalize untouched
Markdown, whitespace, UTF-8, or line endings.
