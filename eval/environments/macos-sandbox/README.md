# The macOS Sandbox

`run` runs a command under `/usr/bin/sandbox-exec` and through
`tinyproxy`, on the `PATH` or named by `TINYPROXY`.

```sh
run --work DIR --home DIR [--allow-host HOST]... [--env NAME[=VALUE]]... \
  [--path DIR]... [--tool DIR]... [--network-log FILE] -- COMMAND...
```

It guarantees that the command:

- writes only in the working directory and the home directory;
- reads and executes only those two, the system trees, `/nix/store` and
  the `--tool` directories, so not your home directory, this workspace or
  any other project;
- reaches only the `--allow-host` hosts, on port 443, through the proxy,
  and no local address, other port, plain HTTP or direct connection;
- gets only `HOME`, `TMPDIR`, `PATH`, the proxy variables and the `--env`
  variables. `--env NAME` passes your own value without showing it on the
  command line.

It does not guarantee:

- that names and sizes of files elsewhere stay hidden; only contents do;
- anything about the clipboard, the keychain or AppleScript, which are
  expected to be closed but are not checked;
- that the key stays secret from the allowed host's other users: the
  agent could send it anywhere on that host.

Tools from another store, such as Homebrew's, need `--tool` or a line in
`profile.sb`. `sandbox-exec` cannot start inside some other sandboxes.
