# Autonomous mutation execution

Configure trusted test commands in the project-root `matis.json`. Matis does not
create a workspace, discover targets, or run unchanged tests first. Prepare your
environment and confirm that your selected tests pass before starting.

```json
{
  "mutations": {
    "rules": ["comparison.less-to-less-equal"],
    "targets": {
      "module-a": {
        "sources": ["src/module_a/**"],
        "cwd": ".",
        "command": ["cargo", "test", "module_a"],
        "timeoutSeconds": 120
      }
    }
  }
}
```

Source patterns match project-relative POSIX paths with shell-style wildcards.
Only `**` crosses directory separators. Use `src/package/**` for a subtree.
Patterns select source paths, not test paths. Multiple matching targets all run.
Unmatched mutations produce `unmapped` results without changing source.

`cwd` defaults to the project root and must resolve to a directory inside it.
`timeoutSeconds` defaults to 120 and must be a positive integer. Commands are
argument arrays, not shell strings. Invoke a shell explicitly if one is needed.
Commands inherit the environment. Python commands get an empty bytecode-cache
path and disabled bytecode writes to prevent stale imports. Other build and test
caches remain the user's responsibility.

Inspect manifests without execution:

```sh
matis mutations --folder .
```

Run all configured targets, or select comma-separated target names:

```sh
matis mutations run --folder . --target all --output mutation-results.jsonl
matis mutations run --folder . --target complexity,parsing --output selected.jsonl
```

Omit `--target` to display available targets and choose interactively. Enter one
name, comma-separated names, or `all`. Unknown names prompt again. In CI or when
stdin is not a terminal, `--target` is required and no prompt is shown. Empty
names and combining `all` with other names are errors. Duplicate names run once.
Target names in configuration cannot contain commas or surrounding whitespace,
and `all` is reserved.

Selection limits both the mutated source paths and the commands that run. Even
if an unselected target covers the same source, its command does not run. `all`
runs every configured target, not every supported source in the repository.
Unrelated source paths are excluded before manifest generation and produce no
`unmapped` records. Manifest inspection without `run` still shows all enabled
replacements.

Progress logs stream to stderr during planning and execution. They show the
mapped/unmapped counts, mutation position and edit, target command, working
directory, timeout, command-log path, outcome, duration, and confirmed source
restoration. The final JSON summary stays on stdout. To save progress separately:

```sh
matis mutations run --folder . --target all --output results.jsonl 2>progress.log
```

`--output` resolves from the project root. Each result is written immediately as
one JSON object per line. Command output goes to separate files in
`<output>.logs/`. Each run replaces the existing JSONL result file and overwrites
command logs with matching names. Logs not referenced by the new result file
remain in the directory. Use a different output path to keep previous results.
Parent output directories must already exist. Matis rejects result paths that
would overwrite a mutation source or `matis.json`, and rejects symlinked command
logs and log directories.

Each record includes the manifest ID, source path, rule ID, outcome, and command
results. Command results include target name, command, exit code, duration, and
log path. Outcomes are:

| Outcome | Meaning |
| --- | --- |
| `survived` | All matching commands returned zero. |
| `killed` | At least one matching command returned nonzero. |
| `timeout` | A command exceeded its timeout, with no kill or execution error. |
| `error` | Source validation or command execution failed, with no kill. |
| `unmapped` | No configured target matched the source path. |

A nonzero exit is not proof of a test assertion failure. Compilation errors,
empty test collections, and pre-existing failures can count as kills. The CLI
returns success when execution completes, even if mutations survive or records
contain errors. Inspect the result records for CI policy decisions.

Matis verifies source hashes before editing and restores the exact original
bytes after each mutation. A restoration failure aborts execution. SIGINT and
SIGTERM restore the active mutation; SIGKILL, power loss, and machine failure
cannot guarantee restoration. Do not edit source or run another mutation process
in the same workspace during execution. Commands can change other files or
external services; Matis does not roll those changes back. On POSIX systems,
Matis stops each command's process group, including children, before restoration.
On other systems, it stops only the direct command process.

The repository configuration targets only `matis-rust/src/matis/`. Targets run
`cargo test --locked --all-targets` from `matis-rust/`. Dashboard and test
sources are not executed as mutants. Analysis covers the entire project, but
execution generates manifests only for sources matched by the selected targets.
