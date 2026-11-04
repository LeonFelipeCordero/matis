# Rust test structure and timing

## Structure

The deleted Python suite separated unit tests by production module. The Rust suite now uses the same folder structure without adding a mocking dependency or exposing private APIs.

```text
matis-rust/tests/
  unit/
    cli/                  # Direct schema and option parsing
    parsing/adapters/     # Record assembly, policy, and syntax contracts
    metrics/adapters/     # In-memory metrics and resolution contracts
    mutations/            # Config, matching, and outcome logic
  integration/
    cli/                  # Binary output and service handoff
    parsing/              # Real discovery failures
    persistence/          # SQLite, migrations, and reindexing
    services/             # Navigation and budgets
    mutations/            # Subprocesses, signals, restoration, and manifests
    pipeline/             # Full sample and snapshot parity
    support/              # Shared isolated project and process helpers
  unit.rs                 # Public API unit-test entry point
  integration.rs          # Public API integration-test entry point
  code_samples/           # Unchanged analyzer inputs
  snapshots/              # Unchanged reference outputs
```

Rust modules include private tests with `#[path]`. These tests retain access to private functions without public test APIs. Small existing inline tests remain beside their functions. Library integration modules contain `integration_tests` in their names. The fast command excludes those modules. The full command excludes nothing.

CLI option matrices call Clap and `GraphOptions` in-process. Binary tests retain representative usage errors, database checks, numeric handoff, source output, and overflow behavior. The three CLI integration files now start 29 processes instead of 107. Parsing policy tests use paths and rule records without walking a project. Parser assembly tests accept native-shaped records instead of processing a complete sample. Grammar-specific tests still use real syntax trees where tree-sitter nodes are the contract.

Mutation config and outcome tests are separate from real process and snapshot tests. SQLite migration and storage tests remain real integration tests. Replacing these boundaries with mocks would lose the behavior they verify.

The former 15 external test binaries are now two. This reduces separate compilation and linking, shares helper code, and lets integration groups run concurrently. Temporary projects and databases remain isolated per test. No snapshots changed. Production behavior and dependencies are unchanged.

## Commands

Run from the repository root. Fast development suite:

```sh
cargo test --manifest-path matis-rust/Cargo.toml --locked --lib --test unit -- --skip integration_tests
```

Focused CLI or parsing unit tests:

```sh
cargo test --manifest-path matis-rust/Cargo.toml --locked --lib cli:: -- --skip integration_tests
cargo test --manifest-path matis-rust/Cargo.toml --locked --lib parsing:: -- --skip integration_tests
```

Public integration contracts, optionally filtered to one layer:

```sh
cargo test --manifest-path matis-rust/Cargo.toml --locked --test integration
cargo test --manifest-path matis-rust/Cargo.toml --locked --test integration -- services::
```

`--test integration` does not include private library integration tests. Run the complete suite before completion:

```sh
cargo test --manifest-path matis-rust/Cargo.toml --locked --all-targets
```

Mutation targets in `matis.json` use the new entry points and module filters. They retain their previous integration groups. Their commands also run library tests; they do not use the fast suite's integration exclusion.

## Measurements

Measured sequentially on the same machine, with warm build artifacts and default Rust test parallelism. Compilation warm-up was excluded. The baseline used the unchanged `HEAD` source in a temporary directory, with the same Cargo target directory. These are wall-clock observations, not timing assertions.

| Suite | Before | After |
| --- | ---: | ---: |
| Complete suite | 24.77 s | 18.75–19.73 s; median 19.59 s |
| Fast isolated suite | Not separate | 0.69 s; 235 tests |
| Focused CLI unit tests | Not measured | 0.15 s |
| Focused parsing unit tests | Not measured | 0.37 s |

The full-suite median is about 21% below the single baseline run. The complete suite passes 378 tests, with no ignored tests. The fast suite passes 203 library tests and 32 public API tests. It excludes 39 library integration tests and the 104 external integration tests.

Timing varies with machine load. The fast command deliberately runs less than the complete suite. It is not a replacement for integration validation.

## Validation

Passed: all Rust targets, release binaries, formatting, Clippy with warnings denied, and whole-repository Matis analysis. An independent review checked the moved assertions and mutation filters. No mutation campaign ran. Dashboard checks were not needed because dashboard code and shared output contracts did not change.
