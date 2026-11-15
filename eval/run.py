#!/usr/bin/env python3
"""Runs scenarios with an agent inside the macOS sandbox and keeps the records.

    eval/run.py --model PROVIDER/MODEL/EFFORT [--agent NAME] [--limit SECONDS]
        SCENARIO_DIR...
"""
import argparse
import json
import os
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

EVAL = Path(__file__).parent.absolute()
REPO = EVAL.parent
SANDBOX = EVAL / "environments" / "macos-sandbox"
AGENTS = EVAL / "agents"
SCENARIOS = EVAL / "scenarios"
MODELS = {
    "anthropic": ("fable-5.2", "opus-5.5", "sonnet-5.5", "haiku-5.5"),
}
EFFORTS = ("off", "minimal", "low", "medium", "high", "xhigh", "max")


def main():
    args = parse_arguments()
    args.model = normalise_model(args.model)
    args.agent = AGENTS / args.agent
    if not (args.agent / "setup").exists():
        known = ", ".join(sorted(path.parent.name for path in AGENTS.glob("*/setup")))
        stop(f"unknown agent {args.agent.name!r}; known: {known}")
    scenarios = [path.absolute() for path in args.scenarios]
    run = time.strftime("%Y%m%d-%H%M%S")
    for scenario_dir in scenarios:
        run_scenario(args, run, scenario_dir)


def run_scenario(args, run, scenario_dir):
    combination = f"{SANDBOX.name}-{args.agent.name}-{args.model.replace('/', '-')}"
    name = scenario_name(scenario_dir)
    record = REPO / "tmp" / "eval-runs" / run / combination / name
    home, work = record / "agent-home", record / "working-home"
    home.mkdir(parents=True)
    prepare_work(scenario_dir, work)
    needs = agent_setup(args.agent, home, args.model)
    command = sandbox_command(args.agent, record, home, work, needs)
    outcome = run_agent(command, scenario_dir / "prompt.md", record / "output.txt", args.limit)
    fetch_out(home / "out", record)
    print(f"Scenario: {name}")
    print(f"Run:      {outcome}")
    print(f"Record:   {record}")


def prepare_work(scenario_dir, work):
    """The scenario's files, then its setup, outside the sandbox."""
    files = scenario_dir / "files"
    if files.exists():
        shutil.copytree(files, work)
    else:
        work.mkdir(parents=True)
    setup = scenario_dir / "setup"
    if setup.exists():
        subprocess.run([setup], cwd=work, check=True)


def agent_setup(agent, home, model):
    """Runs the adapter's setup; returns the hosts and variables the agent needs."""
    answer = subprocess.run([agent / "setup", home, model, REPO], stdout=subprocess.PIPE, text=True)
    if answer.returncode != 0:
        sys.exit(answer.returncode)
    return json.loads(answer.stdout)


def fetch_out(out, record):
    """Copies each item of the agent's out/ to a new name in the record,
    with links copied as links, so nothing outside is read or written."""
    for item in out.glob("*"):
        target = record / item.name
        if target.exists() or target.is_symlink():
            continue
        if item.is_dir() and not item.is_symlink():
            shutil.copytree(item, target, symlinks=True)
        else:
            shutil.copy2(item, target, follow_symlinks=False)


def run_agent(command, prompt, output, limit):
    """Runs the command with the prompt on stdin; returns how it ended."""
    with open(prompt, "rb") as stdin, open(output, "wb") as stdout:
        process = subprocess.Popen(command, stdin=stdin, stdout=stdout, start_new_session=True)
        try:
            status = process.wait(timeout=limit)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGTERM)
            process.wait()
            return "cut off at the time limit"
    return "finished" if status == 0 else f"ended with status {status}"


def sandbox_command(agent, record, home, work, needs):
    command = [SANDBOX / "run", "--work", work, "--home", home]
    for host in needs["hosts"]:
        command += ["--allow-host", host]
    for name in needs["env"]:
        command += ["--env", name]
    command += ["--network-log", record / "network.log", "--tool", agent]
    return command + ["--", agent / "run", home, work]


def parse_arguments():
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True, help="provider/model/effort")
    parser.add_argument("--agent", default="pi", help="an adapter in eval/agents/")
    parser.add_argument("scenarios", nargs="+", type=Path, metavar="SCENARIO_DIR")
    parser.add_argument("--limit", type=int, default=600, help="seconds")
    return parser.parse_args()


def normalise_model(text):
    """A recognised provider/model/effort, in lower case."""
    parts = text.lower().split("/")
    if len(parts) != 3:
        stop(f"--model must be provider/model/effort, not {text!r}")
    provider, model, effort = parts
    if model not in MODELS.get(provider, ()):
        known = ", ".join(f"{p}/{m}" for p, models in MODELS.items() for m in models)
        stop(f"unknown model {provider}/{model}; known: {known}")
    if effort not in EFFORTS:
        stop(f"unknown effort {effort!r}; known: {', '.join(EFFORTS)}")
    return f"{provider}/{model}/{effort}"


def scenario_name(scenario_dir):
    """The path below eval/scenarios/, or the directory's own name."""
    if scenario_dir.is_relative_to(SCENARIOS):
        return scenario_dir.relative_to(SCENARIOS)
    return Path(scenario_dir.name)


def stop(message):
    sys.exit(f"run.py: {message}")


if __name__ == "__main__":
    main()
