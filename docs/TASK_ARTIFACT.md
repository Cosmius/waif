# Task Artifact Format

- Status: proposed
- Purpose: Define the structure and section types of workflow artifacts

## Overview

A task artifact is Markdown containing one title, structured metadata,
optional opaque pre-section prose, and level-two sections. An artifact contract
declares its recognized metadata, known sections, content types, and stable-ID
families.

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

An artifact contract declares every recognized metadata key. Every recognized
key is required and unique; contracts do not define optional recognized
metadata. Recognized keys may occur in any relative order while the parser is
inside structured metadata.

The first ordinary content or unknown metadata-looking entry ends structured
metadata and begins opaque pre-section prose. It and all following content
before the first level-two section remain prose, including later
recognized-key-looking entries. A misspelled key therefore begins prose and
causes the actual recognized key to be reported missing rather than
reinterpreting later source.

A metadata entry begins with `-` in column one and must have whitespace between
`-` and its key. Extra whitespace around the key, colon, and value is
insignificant. The first colon separates the key from its value, which may
contain further colons.

A metadata value may continue onto indented lines, including an indented fenced
code block. Parsed multiline values are immutable through the structured
artifact API. A programmatic replacement is supported only when the parsed
value and its replacement are both single-line.

## Section Types

An artifact contract maps exact level-two section names to these types:

- prose;
- itemised;
- mixed itemised;
- expanded itemised; or
- plan items.

Unknown sections are prose. Every section name, known or unknown, is unique.
Known sections have a declared relative order. Optional known sections may be
omitted, and unknown sections do not affect the order comparison. Empty
sections produce warnings but do not invalidate an artifact.

### Prose

A prose section contains opaque Markdown. Structured processing does not
validate, fill, or modify its body.

### Itemised

An itemised section contains stable-ID items in either form:

- compact: `- ID: content`;
- expanded: `### ID: title` followed by an opaque body.

Ordinary itemised sections use one form consistently. Expanded item bodies end
at the next level-three or level-two heading. No non-whitespace content may
occur outside items.

A mixed itemised section may declare distinct compact and expanded ID families.
All compact items precede all expanded items. An expanded itemised section
accepts only expanded items.

Stable-ID numeric suffixes are canonical positive decimal integers from 1
through `2^63 - 1`. Numbers are unique and increasing in document order within
their family. IDs and names are case-sensitive.

Every ordinary item ID consists of an artifact item prefix, a section-specific
family, and a final item number. The prefix owns its trailing dash; the family
does not include a leading separator. Goal and plan prefixes are the known
literals `G-` and `P-`. Step prefixes have the shape `S<n>-`, and
implementation-review prefixes have the shape `S<n>-R<m>-`, where every
numeric component is canonical and bounded as above. Plan-item headings retain
their independent, hard-coded `P<number>` contract and do not participate in
artifact-prefix inference.

A checker uses a known prefix when the artifact identity is available before
item validation. Known prefixes must match exactly. Otherwise a step or review
checker uses an unknown prefix shape: the first valid item establishes the
observed prefix, and every later item family in that artifact must use the same
value. Unknown mode establishes internal consistency only. The shared schema
retains each observed numeric component with its source span; artifact-specific
checking compares those observations with titles, filenames, or nominal paths
when that context is available. Prefix inference never crosses artifact
boundaries or uses opaque prose or neighboring files.

### Plan Items

The `Plan Items` section contains expanded nested records. Bare content outside
a record is invalid. Each plan item begins with `### P<number>: <title>` and
ends at the next plan item, level-two section, or end of file.

Only leading metadata keys recognized by the plan-item contract are structured.
The first unknown metadata-looking entry or ordinary content begins the opaque
plan-item prose body; later recognized-key-looking entries remain prose.

Level-four and deeper headings are permitted in opaque plan-item prose.
Level-one, level-two, and non-plan-item level-three headings are invalid because
they break the artifact hierarchy. Fenced heading-like text remains opaque.

## Goal Contract

A checked file named exactly `goal.md` receives this contract.

Required metadata is:

```text
Status    drafting | accepted | amending
Created   RFC 3339 timestamp with Z or a numeric UTC offset
Updated   RFC 3339 timestamp with Z or a numeric UTC offset
```

Known sections have this relative order:

| Section             | Type                       | Presence |
|---------------------|----------------------------|----------|
| Outcome             | prose                      | required |
| Acceptance Criteria | itemised; G-AC<n>          | required |
| Open Questions      | itemised; G-Q<n>           | optional |
| Assumptions         | itemised; G-A<n>           | optional |
| In Scope            | itemised; G-IN<n>          | optional |
| Out of Scope        | itemised; G-OUT<n>         | optional |
| Revisions           | expanded; G-REV<n>         | optional |

Ordinary goal itemised sections accept compact or expanded items consistently.
`Revisions` accepts only expanded items. Item content and expanded bodies are
opaque. Goal checking does not constrain title content or semantic quality.

## Plan Contract

A checked file named exactly `plan.md` receives this contract.

Required metadata is:

```text
Status       drafting | accepted | amending
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

| Section                  | Type                         | Presence |
|--------------------------|------------------------------|----------|
| Technical Summary        | prose                        | required |
| Decisions                | mixed; P-D<n>, P-DD<n>       | required |
| Open Questions           | itemised; P-Q<n>             | optional |
| Assumptions              | itemised; P-A<n>             | optional |
| Current System           | prose                        | required |
| Plan Items               | plan items; P<n>             | required |
| Risks                    | itemised; P-R<n>             | required |
| Cross-Cutting Validation | itemised; P-V<n>             | required |
| Revisions                | expanded; P-REV<n>           | optional |

`Decisions` is the only mixed itemised section. Compact decisions use
`- P-D<number>: content`; expanded decisions use
`### P-DD<number>: title` and follow all compact decisions. Either form may be
absent. Other ordinary itemised sections use compact or expanded items
consistently. `Revisions` is expanded-only.

Each plan item has this authoring form:

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

`Status` is the only structurally recognized plan-item metadata key. It is
required and accepts `pending` or `done`. `Goal criteria`, `Outcome`, `Areas`,
`Depends on`, and `Validation` are semantic author-facing fields inside opaque
prose. They may span lines and are preserved exactly; deterministic checking
does not interpret their references or meaning.

Plan-item IDs use `P<number>`, are unique and increasing, and share the common
signed-64-bit numeric bound. A plan item must have a non-empty title.

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
Status         drafting | accepted | amending | done
Source commit  not-created | full 40-character hexadecimal Git commit hash
Created        RFC 3339 timestamp with Z or a numeric UTC offset
Updated        RFC 3339 timestamp with Z or a numeric UTC offset
```

`Source commit` is `not-created` for `drafting`, `accepted`, and `amending`
steps. A `done` step requires a full commit hash. Authoring metadata such as
`Plan items`, `Goal criteria`, and `Estimated non-test/doc changes` follows
the structural metadata as opaque pre-section prose and is not checked.

Known sections have this relative order:

| Section              | Type                     | Presence |
|----------------------|--------------------------|----------|
| Objective            | prose                    | required |
| Plan Item Coverage   | itemised; S<n>-PC<m>     | required |
| Context              | prose                    | required |
| Open Questions       | itemised; S<n>-Q<m>      | optional |
| Assumptions          | itemised; S<n>-A<m>      | optional |
| Done When            | itemised; S<n>-D<m>      | required |
| Changes              | itemised; S<n>-C<m>      | optional |
| Size and Coherence   | prose                    | optional |
| Tests                | itemised; S<n>-T<m>      | optional |
| Validation           | itemised; S<n>-V<m>      | optional |
| Risks and Edge Cases | itemised; S<n>-R<m>      | optional |
| Revisions            | expanded; S<n>-REV<m>    | optional |

Ordinary itemised sections accept compact or expanded items consistently.
`Done When` uses those ordinary forms; Markdown task-list checkbox syntax is
not a conforming substitute for an item ID.

Each coverage item has the form
`S<n>-PC<m>: P<m> - partial | complete - <non-empty description>`. The `PC`
suffix must equal the covered plan ID suffix, but coverage IDs need not be
consecutive. Checking does not compare coverage entries with the opaque
`Plan items` authoring field.

Item content and prose are opaque. Numeric bounds, complete identifier
consumption, item-form consistency, uniqueness, ordering, section order, and
empty-section warnings follow the common rules.

## Implementation Review Contract

A checked file named `reviewN.md` in an immediate step directory receives this
contract, where `N` is a positive decimal review number. Its title is
`Implementation Review N`. The filename, title, and review-number component of
every stable ID agree.

When the direct parent matches `NN-short-name`, its positive, at-least-two-digit
number supplies the canonical step identity. Otherwise checking warns, skips
comparisons requiring path identity, and still requires one internally
consistent `S<n>-R<m>-` prefix. Selected paths are interpreted nominally
rather than by canonicalizing symlink targets. Directory padding is omitted
from IDs: `steps/04-check/review1.md` uses `S4-R1-F1`.

Required metadata, in order, is:

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

| Section        | Type                            | Presence |
|----------------|---------------------------------|----------|
| Findings       | findings; S<n>-R<m>-F<k>        | required |
| Scope          | itemised; S<n>-R<m>-SC<k>       | optional |
| Validation     | itemised; S<n>-R<m>-V<k>        | optional |
| Residual Risks | itemised; S<n>-R<m>-RR<k>       | optional |

`Findings` contains either the exact prose `No findings.` or one or more
expanded findings headed `### S<n>-R<m>-F<k>: <opaque content>`. A `pass`
decision requires the sentinel and a `changes-requested` decision requires at
least one finding. Finding content and bodies are opaque; deterministic
checking does not interpret severity, title, location, or recommendations.

Ordinary itemised sections accept compact or expanded items consistently.
Review item numbers are canonical positive decimals through `2^63 - 1`,
unique and increasing within each family. Empty Findings is an error; other
empty sections retain the common warning behavior.

## Source Preservation

Parsing retains source locations for structured content and exact source for
opaque prose. Lifecycle operations locate metadata and exact plan-item IDs;
they do not normalize untouched Markdown, whitespace, UTF-8, or line endings.
