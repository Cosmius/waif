# The pi Adapter

Runs the pi coding agent. `pi` 1.5.2 or newer must be on the `PATH`, in a
directory the sandbox can read.

- `setup` builds and installs WAIF, and configures pi with telemetry off.
  Providers: `anthropic`, with its key in `ANTHROPIC_API_KEY`.
- `run` starts `pi -p`. It saves pi's session as `pi-session.jsonl` and
  `pi-session.html`.

```sh
ANTHROPIC_API_KEY=<key> eval/run.py --model anthropic/haiku-5.5/medium
```
