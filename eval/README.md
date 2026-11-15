# Evaluating WAIF's Skills

An evaluation hands WAIF's skills and a prepared situation to a coding
agent in an isolated place, and keeps a record of what the agent did.

The framework has three kinds of parts, each in its own directory with a
README of what it needs:

- scenarios, in `eval/scenarios/<category>/<scenario>/`;
- an agent adapter, in `eval/agents/<agent>/`;
- a sandbox, in `eval/environments/<sandbox>/`.

## Running Scenarios

```sh
eval/run.py --model PROVIDER/MODEL/EFFORT [--agent NAME] SCENARIO_DIR...
```

- `--agent` names an adapter in `eval/agents/`, `pi` by default.
- `--model` is one of the models `MODELS` in `eval/run.py` recognises, such
  as `anthropic/haiku-5.5/medium`.
- Each `SCENARIO_DIR` is one scenario, such as
  `eval/scenarios/dev-workflow-setup/new-project`.
- `--limit SECONDS` stops the agent after that time in each scenario, 600
  by default.
- `--jobs N` runs up to N scenarios at once, 1 by default.

Each scenario prints how it ended, and leaves in its run directory,
`RUN_HOME`:

- `agent-home/`: `AGENT_HOME`;
- `working-home/`: `WORKING_HOME`, as the agent left it;
- `output.txt`: what `run` wrote to standard output;
- `network.log`: the sandbox's log of every host and port asked for;
- every file the adapter left in `AGENT_HOME/out/`.

## A Complete Run

For each complete run, the runner:

1. creates `AGENT_HOME` and `WORKING_HOME` in
   `tmp/eval-runs/<datetime>/<sandbox>-<agent>-<model>/<scenario>`, named by
   its path below `eval/scenarios/`, or its directory name outside it;
2. sets up `WORKING_HOME` with the scenario;
3. sets up `AGENT_HOME` with the adapter's `setup`;
4. runs the adapter's `run` in the sandbox, as
   `<sandbox>/run -- <adapter>/run AGENT_HOME WORKING_HOME`.

## Scenarios

For each scenario, the runner:

1. copies `files/` as the agent's working area;
2. runs `setup` there, outside the sandbox;
3. sends `prompt.md` to the agent.

## Agent Adapters

An adapter is a directory with two executables:

- `setup AGENT_HOME PROVIDER/MODEL/EFFORT WAIF_SOURCE` maps the
  recognised model to the agent's own name, installs the WAIF in
  `WAIF_SOURCE` and makes `AGENT_HOME` ready to use; it prints, as
  JSON, the `hosts` the agent needs and the `env` variables to pass it;
- `run AGENT_HOME WORKING_HOME` reads the prompt from standard input,
  runs the agent on it in `WORKING_HOME`, and leaves what to keep in
  `AGENT_HOME/out/`.

## Sandboxes

A sandbox is a directory with an executable `run`:

```sh
run --work DIR --home DIR [--allow-host HOST]... [--env NAME[=VALUE]]... \
  [--path DIR]... [--tool DIR]... [--network-log FILE] -- COMMAND...
```
