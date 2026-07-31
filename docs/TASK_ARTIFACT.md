# Task Artifact Format

- Status: proposed
- Purpose: Define the structure and section types of workflow artifacts

## Overview

A task artifact is Markdown containing one title, metadata, and level-two
sections. An artifact contract supplies its permitted fields, predefined
sections, and ID formats. This document is itself a valid task artifact.

## Authoring and Parser Scope

Task artifacts are generated and maintained by `waif`. Humans may review them,
but should request changes through `waif` rather than editing the files
directly.

The artifact parser is not a general-purpose Markdown or CommonMark parser. It
supports the canonical Markdown emitted by `waif` and preserves that source
exactly. It recognizes artifact headings and simple fenced code blocks using
line-oriented rules.

Markdown container interactions are outside the supported format. In
particular, generated artifacts must not place headings or fenced code blocks
inside list items, block quotes, or raw HTML blocks. Noncanonical or
hand-edited Markdown may be rejected or interpreted differently from a
full-featured Markdown renderer.

## Document Structure

An artifact contains, in order:

1. Exactly one level-one Markdown heading (`#`) with a non-empty, single-line
   title. It must be the first non-whitespace line.
2. A possibly empty metadata block whose entries have the logical form
   `- Key: value`.
3. Zero or more level-two sections (`##`).

Whitespace-only lines have no artifact-level meaning and may occur before the
title or between these constructs. Programmatic modifications must preserve
every existing whitespace character and line ending exactly.

A metadata entry may be indented and must have at least one whitespace
character between `-` and its key. Extra whitespace around the key, colon, and
value has no meaning. The first colon separates the key from the value, which
may contain further colons. Ignoring whitespace-only lines, no other content
may occur between the title, metadata, and first section. Front matter,
preambles, comments, footers, and all other unmentioned content are invalid.

## Section Types

An artifact contract maps exact level-two section names to three types:

- prose;
- numbered itemised; or
- plan items.

An unknown section name resolves to prose. The `plan items` type is reserved
for the exact section name `Plan Items`.

Section content ends at the next level-two heading or end of file.

### Prose

A prose section may contain arbitrary Markdown, including unnumbered lists and
subsections. Structured processing treats the entire section as opaque and
does not validate, fill, or modify its contents.

### Numbered Itemised

A numbered itemised section contains stable, uniquely identified items in one
of two forms:

- List form: `- ID: content`
- Subsection form: `### ID: title`

A list item's content is unrestricted Markdown and may span indented lines;
nested lists remain part of that item. A subsection body ends at the next
level-three or level-two heading and is otherwise opaque prose. No
non-whitespace content may occur outside the items.

IDs remain attached to their logical items and must not be renumbered, reused,
or inferred from position. An ordered-list marker such as `1.` is not an ID
unless the contract explicitly defines it as one. The contract defines every
ID's syntax and uniqueness scope.

A section must use one form. Mixing is allowed only for a limited, predefined
section whose contract explicitly permits it. The contract must give list and
subsection items different, disjoint ID namespaces. All list items must precede
all subsection items.

The plan artifact's `Decisions` section is such an exception:

```markdown
## Decisions

- P-DD1: Use an idempotency key for every payment request.
- P-DD2: Store provider request IDs with payment attempts.

### P-D1 - Reconcile an unknown result before retrying

Query the provider with the original idempotency key before issuing another
request, preventing a duplicate charge when the first request succeeded.
```

Here list items use `P-DD<N>` and subsections use the disjoint `P-D<N>`
namespace.

### Plan Items

The `Plan Items` section contains zero or more plan items. Each plan item is a
level-three subsection consisting only of:

1. A title beginning with its stable plan-item ID.
2. Its `- Key: value` metadata block.
3. One prose body, which may contain multiple paragraphs and non-heading
   Markdown.

No list item or bare prose may occur directly under `Plan Items`. A plan item
cannot contain a level-four or deeper heading. Its body ends at the next plan
item, level-two section, or end of file.

```markdown
## Plan Items

### P1 - Persist payment attempts

- Status: ready
- Depends on: none
- Goal criteria: G-AC1, G-AC2
- Validation: `cargo test payment_attempts`

Store every provider request, response, request ID, and idempotency key with
the corresponding order.
```

## Contracts and Validation

An artifact contract must define:

- the title form;
- permitted metadata keys, order, values, and requiredness;
- predefined section names, types, order, and cardinality;
- numbered-item ID syntax and uniqueness scope;
- permitted or required item source forms, including disjoint ID namespaces
  and source ordering for mixed sections; and
- plan-item title, metadata, and prose constraints for `Plan Items`.

Names and IDs are case-sensitive unless the contract says otherwise. Content
not permitted by this document or the applicable contract MUST NEVER appear.
A parser must reject violations in structured sections. Programmatic filling
and modification operate only on metadata and item IDs; prose is preserved
without interpretation.

## Goal Contract

A checked file named exactly `goal.md` receives the goal-specific contract in
addition to the common artifact envelope. Goal-specific checking does not
constrain title content or metadata and section ordering.

The required metadata is:

```text
Status      drafting | accepted | amending
Created     valid RFC 3339 timestamp with Z or a numeric UTC offset
Updated     valid RFC 3339 timestamp with Z or a numeric UTC offset
```

Every metadata key is unique. Additional unique metadata keys are allowed.

The required level-two sections are:

```text
Outcome                 prose
Acceptance Criteria     itemised; G-AC<n>
```

The optional level-two sections are:

```text
In Scope                itemised; G-IN<n>
Out of Scope            itemised; G-OUT<n>
Open Questions          itemised; G-Q<n>
Assumptions             itemised; G-A<n>
Revisions               expanded itemised; G-REV<n>
```

Every section name is unique, including names unknown to the goal contract. A
uniquely named unknown section is opaque prose. Empty sections produce
warnings and do not invalidate the artifact.

Ordinary itemised sections accept either compact or expanded items, with one
form per section. `Revisions` accepts only expanded items. Expanded headings
use `### <ID>: <title>`.

Every item ID uses the family declared for its section. Its `<n>` component is
a positive decimal integer matching `[1-9][0-9]*`, and its numeric value is
unique within that reference-ID namespace. List-form items use
`- G-<family><n>: content`. Item content, including empty content after a valid
ID and colon, and expanded item bodies are opaque to goal-specific validation.

These are structural rules only. The authoring AI agent, not `waif check`, is
responsible for the semantic quality, feasibility, and completeness of goal
content and for distinguishing substantive included behavior from exclusions
and non-goals.
