# Rust CLI testing strategy

Put isolated logic tests in `matis-rust/tests/unit/`, with folders that mirror `src/matis/`. Use in-memory records and fake boundary outputs. CLI option tests must call the parser directly, not start the binary or index a project. Keep small inline Rust unit tests beside their functions.

Put real process, filesystem, database, and pipeline contracts in `matis-rust/tests/integration/`. Module-private integration tests use `#[path]` and module names containing `integration_tests`, so the fast command can exclude them. Do not add that name to unit-test modules. The external integration tests share one test binary and common support helpers.

Use code samples and snapshot tests to cover language features without many separate tests. Add new language features and metrics to the samples in `matis-rust/tests/code_samples/`. Keep the checked-in snapshots in `matis-rust/tests/snapshots/`. Python samples are analyzer inputs, not a Python importer.

# Checks

Run commands from the repository root. Run only checks for changed components and affected consumers.

- For importer changes (`matis-rust/`), run the Rust checks.
- For dashboard changes (`dashboard/`), run the dashboard checks.
- For shared schemas, output contracts, fixtures, or migrations, run both sets of checks when both components are affected. Dashboard integration tests build and use the Rust importer.
- For build, dependency, or CI changes, run checks for the affected components.
- For documentation-only changes, check text and command accuracy. Do not run runtime suites or Matis analysis.

## Rust importer

Use the fast suite during development. It excludes real integration contracts, not the full validation requirement:

```sh
cargo test --manifest-path matis-rust/Cargo.toml --locked --lib --test unit -- --skip integration_tests
```

For one layer, use a library filter such as `--lib cli::` or `--lib parsing::`. For integration tests, use `--test integration` and an optional filter such as `services::`. Database and process tests that need private APIs remain in the library test binary; `--test integration` alone does not run every integration test.

Run all checks before completion:

```sh
cargo test --manifest-path matis-rust/Cargo.toml --locked --all-targets
cargo build --manifest-path matis-rust/Cargo.toml --release --locked --bins
cargo fmt --manifest-path matis-rust/Cargo.toml --check
cargo clippy --manifest-path matis-rust/Cargo.toml --locked --all-targets -- -D warnings
```

Tests must isolate temporary files, databases, and ports. Keep the language samples' intentional complexity, size, parameter, and import-cycle violations. Review snapshot changes against the sample and analyzer changes.

## Mutation testing

Mutation testing is slow because each mutation starts its selected test commands. Inspect `mutations.targets` in root `matis.json`, including each target's `sources` and `command`. Select only targets that cover changed code and affected consumers. Do not run a full campaign by default.

Build the CLI first, then run it from the repository root:

```sh
cargo build --manifest-path matis-rust/Cargo.toml --release --locked --bins
matis-rust/target/release/matis mutations run --folder . --target complexity,parsing
```

Replace the example targets with relevant targets. `--target` accepts comma-separated names or `all`. Omit it for interactive selection; non-interactive runs must pass it explicitly. Use `--target all` only when requested or when a broad audit is justified. Mutation targets run Rust tests. They do not replace the component checks or whole-project Matis analysis.

## Dashboard

Requires Bun and the Rust toolchain. Test setup builds the Rust CLI and a test-only artifact helper once per test process.

```sh
(cd dashboard && bun run build)
(cd dashboard && bun test)
(cd dashboard && bun run typecheck)
(cd dashboard && bun lint --fix)
```

# Matis

For executable changes, run Matis once against the entire repository, not just changed files. Skip this for documentation-only changes. The dashboard uses separate TypeScript checks and is excluded by `.matisignore`.

```sh
cargo run --manifest-path matis-rust/Cargo.toml --locked --bin matis -- check --f .
```
