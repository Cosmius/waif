# Mandate Setup

Used only when `<workflow-dir>/library/commissioner-mandate.md` does not
exist. The agent interviews the human, writes the answers as the mandate,
and shows it for acceptance.

## Interview

- Ask a few questions at a time.
- The agent may suggest an answer to any question, but records only what
  the human clearly states or clearly accepts. An empty or unclear reply
  accepts no suggestion; ask again. Whatever the human leaves undecided
  is not delegated.
- Before the mandate takes effect, ask the human to read the whole of it
  carefully and accept it.
- Cover at least:
  - the model and reasoning effort of each role, in whatever model family
    the human uses;
  - how subagents are started in the human's tool, and whether a Surveyor
    is available;
  - which decisions and actions are delegated beyond the skill's, and what
    must always come back or never be done;
  - what the worklog records. Suggest, as the skill author's suggestion,
    the rulings and the issues the Commissioner thinks worth recording.
  - the limits: how many fix attempts one defect gets in a step's
    reviews before the Commissioner stops and asks, and how many task
    reviews a task gets before the mode ends whatever the verdict. The
    skill sets neither, and without them fix rounds and task reviews can
    go on indefinitely, so suggest three fix attempts and two task
    reviews, as the skill author's suggestion, and keep asking until the
    human has clearly set both. Write each limit as a complete rule, as
    in the example, since the skill gives a bare number no meaning.

## Model Mapping

Present these recommended capability tiers, which rank models against each
other and are not reasoning efforts, and propose several mappings to models
of the human's model family for the human to choose from or replace:

- Commissioner: the best model the human can afford
- Architect: high
- Contractor: medium
- Inspector: high
- Senior Inspector: xhigh
- Clerk: medium
- Surveyor: low

## Example

The example shows the shape of a mandate. Its values are one person's
choices and are not suggestions.

```markdown
# Commissioner Mandate

## Models

- Commissioner: Fable 5.2, medium effort
- Architect: Opus 5.5, medium effort
- Contractor: Sonnet 5.5, medium effort
- Inspector: Opus 5.5, medium effort
- Senior Inspector: Fable 5.2, medium effort
- Clerk: Sonnet 5.5, medium effort
- Surveyor: Haiku 5.5, medium effort

## Subagents

- Start every subagent with the tool's subagent feature, passing its
  model and effort.
- A Surveyor is available for the desktop app only.

## Authority

- The Commissioner may add a development dependency the step plan names.

## Always Bring Back

- Any change to the data file format.
- Any new runtime dependency.

## Never

- Never run the app against real user data.

## Worklog

- The Commissioner records its rulings and the issues it thinks worth
  recording.

## Limits

- When a step's reviews still report the same defect after three fix
  attempts, the Commissioner stops and asks the human.
- A task gets at most two task reviews. The mode ends after the second,
  whatever its verdict.
```
