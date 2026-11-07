<p align="center">
  <img src="assets/matis.png" alt="Matis, a friendly cartoon giant holding the world" width="200" height="200">
</p>

<h1 align="center">Matis</h1>

<p align="center">
  <a href="https://github.com/LeonFelipeCordero/matis/actions/workflows/rust-ci.yml"><img src="https://github.com/LeonFelipeCordero/matis/actions/workflows/rust-ci.yml/badge.svg?branch=main" alt="Rust CLI CI status"></a>
  <a href="https://github.com/LeonFelipeCordero/matis/actions/workflows/dashboard-ci.yml"><img src="https://github.com/LeonFelipeCordero/matis/actions/workflows/dashboard-ci.yml/badge.svg?branch=main" alt="Dashboard CI status"></a>
  <a href="https://github.com/LeonFelipeCordero/matis/actions/workflows/matis-ci.yml"><img src="https://github.com/LeonFelipeCordero/matis/actions/workflows/matis-ci.yml/badge.svg?branch=main" alt="Matis CI status"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-89718e.svg" alt="License: MIT"></a>
</p>

Matis maps a local codebase and measures complexity, size, parameters, circular imports, and dependency impact. Use `matis report` for human- or AI-readable measurements, `matis check` to enforce configurable limits, or its web UI to explore dependencies.

Rust parses files with Tree-sitter, resolves repository-local relationships, and builds a code graph. Analysis runs locally, on demand, without database access by default. Saved graphs use normalized SQLite tables rather than whole-graph JSON blobs; the Bun/React UI reads those facts directly. No cloud service, HTTP server for the importer, cloning, or background indexing.

## Install

Requires the stable [Rust toolchain](https://rustup.rs/). Install from this checkout:

```sh
git clone https://github.com/LeonFelipeCordero/matis.git
cd matis
cargo install --path matis-rust --locked --bins
matis report -f /path/to/project
```

Ensure Cargo's binary directory (usually `~/.cargo/bin`) is on your `PATH`. When upgrading from the previous project name, rename your project config to `matis.json` and your analysis ignore file to `.matisignore`. Move the existing database to `$XDG_DATA_HOME/matis/matis.sqlite` (default `~/.local/share/matis/matis.sqlite`), or set `DATABASE_PATH` to its current location for both CLI and dashboard. The database schema is unchanged; old names are not detected automatically. Update scripts to use `matis` and `matis-migrate`.

Alternatively, run without installing the CLI:

```sh
cargo run --manifest-path matis-rust/Cargo.toml --locked --bin matis -- report -f /path/to/project
```

## What it analyzes

| Feature | What you get |
| --- | --- |
| Code structure | Folders, files, types, functions, and methods in a navigable hierarchy. |
| Relationships | Imports, calls, callable references, inheritance, and containment; dependency views at module, file, type, and callable levels. |
| Complexity | Cyclomatic/weighted complexity, decision depth, nested loops, and per-file totals. |
| Size and parameters | Physical line counts for files and functions/methods, plus declared input parameter counts for supported callable syntax. |
| Circular imports | Groups of files in runtime import cycles, with an example cycle and source evidence; type-only imports do not establish cycles. |
| Dependency metrics | Fan-in (direct dependents), fan-out (direct dependencies), transitive change impact (all direct/indirect dependents), and internal instability, counting distinct internal peers. |
| Parse diagnostics | Parsing failures retained in the graph, alongside unresolved external targets and relationship confidence. |

Reports show measurements, not pass/fail judgments. Unavailable values are `N/A` (or `null` in JSON), not zero. Lines include comments and blanks; parameters count declared inputs. Dependency metrics include tests and type-only dependencies.

Matis supports grammars recognized by its pinned Tree-sitter language pack, **except C, C++, C#, PHP (including PHPDoc), and Ruby (including RBS)**. Python, JavaScript/JSX, TypeScript/TSX, Java, Kotlin, Go, Rust, and Elixir have specialized adapters; other supported grammars use generic extraction. Resolution is static and conservative, not compiler/type inference. Unresolved imports cannot establish cycles, and passing checks does not prove the code is problem-free.

### Project checks

`matis check` enforces the defaults below. Override them in the analyzed folder's root `matis.json`, for example `{"maxParameters": 5}`. Omitted keys keep their defaults; `null` disables a check.

| Config key | Default maximum | Applies to |
| --- | --- | --- |
| `maxFileLines` | 600 | Files |
| `maxFunctionLines` | 100 | Functions/methods |
| `maxParameters` | 3 | Functions/methods |
| `maxWeightedComplexity` | 8 | Functions/methods |
| `maxDecisionDepth` | 3 | Functions/methods |
| `maxNestedLoops` | 0 | Functions/methods |
| `maxCircularImportGroups` | 0 | Whole project |
| `maxChangeImpact` | `null` (disabled) | Files/modules |
| `maxInstability` | `null` (disabled) | Files/modules |

Checks fail when limits are exceeded or analysis is incomplete. Each failure links to its section in the [fix guide](docs/check-fixes.md).

#### Use your own fix guide

Add `helpLinks` to the project's `matis.json` to override individual fix links:

```json
{
  "helpLinks": {
    "maxParameters": "https://example.com/team/parameter-objects",
    "maxFileLines": "https://example.com/team/module-structure"
  }
}
```

Keys must be rule names from the [fix guide](docs/check-fixes.md), including `analysis`. Values must be HTTP or HTTPS links without whitespace, angle brackets, or backslashes. Omitted rules keep the default guide. Links appear as `helpUrl` in JSON and as `How to fix` in Markdown. Matis prints links; it does not fetch them. Clickable IPv6 links in Markdown require a renderer that supports inline HTML.

### Mutation manifests

`matis mutations` generates JSON manifests for independent single-operator edits; it does not modify source or execute mutations. Opt in with rule IDs in the analyzed folder's root `matis.json`:

```json
{
  "mutations": {
    "rules": ["arithmetic.add-to-subtract", "comparison.equal-to-not-equal"]
  }
}
```

Omitting `mutations.rules` or using `[]` disables generation. Duplicate IDs are harmless; each selected ID enables all its applicable language mappings. The old rule-to-replacement dictionary is rejected.

```sh
matis mutations --rules          # List versioned rules and language/token mappings
matis mutations -f .             # Analyze current sources without saving
matis index -f .                 # Save current candidates (requires migrations)
matis mutations -f . --saved     # Use saved candidates; verify current source
```

Saved mode never refreshes the index: missing or stale source fails the whole request; reindex after source changes. Each manifest is an alternative against the original file, not a patch to apply sequentially. See [the full catalog and manifest contract](matis-rust/MUTATIONS.md).

### Mutation testing

`matis mutations run` applies independent mutations, runs the configured test commands, and restores source after each mutation. No AI agent is needed. Configure source-to-test targets in `matis.json`; see [the mutation runner guide](matis-rust/MUTATION_RUNNER.md).

Mutation testing is slow because it starts the selected Rust test commands for every mutation. Runtime depends on hardware, selected tests, and timeouts. The previous Python campaign estimate does not apply to the Rust importer.

For routine changes, run only mutation targets that cover the changed code and affected consumers. Check each target's `sources` and `command` in `matis.json`. For example, from this repository's root:

```sh
cargo run --manifest-path matis-rust/Cargo.toml --locked --bin matis -- mutations run --folder . --target complexity,parsing
```

Replace those example targets with the ones relevant to your changes. Omit `--target` to choose interactively; CI must pass it explicitly. Use `--target all` only for a requested full campaign or a justified broad audit. Target selection limits mutation execution, not whole-project analysis. Keep running the normal tests and `matis check` as required.

### Exclude files

Matis honors `.gitignore`. Add `.matisignore` for analysis-only exclusions:

```gitignore
*.generated.ts
/vendor/
/scratch/
```

Nested ignore files and Git-style patterns, including `!` exceptions, are supported. Exceptions cannot restore Git-ignored files or files inside an excluded parent. Without a root `.gitignore`, built-in dependency/build/generated-directory exclusions apply. Reanalyze to update saved results.

## Web UI

Requires [Bun](https://bun.sh/). From the checkout root, apply migrations before analyzing and saving a project, then start the dashboard:

```sh
cargo run --manifest-path matis-rust/Cargo.toml --locked --bin matis-migrate
cargo run --manifest-path matis-rust/Cargo.toml --locked --bin matis -- index -f /path/to/project
cd dashboard
bun install
bun run dev
```

Open **http://localhost:3000**. Drill from folders into files and functions, navigate with breadcrumbs, search the contents list, and inspect incoming/outgoing connections, complexity, line/parameter counts, and dependency metrics. The graph supports pan/zoom and hides test files visually; metrics still use the full index, including tests and type-only dependencies.

Re-run `matis index -f /path/to/project` and refresh the UI after source changes. Projects are automatically registered or updated by their local folder location.

Rust owns the SQLite schema and migrations. The default database is `$XDG_DATA_HOME/matis/matis.sqlite`, falling back to `~/.local/share/matis/matis.sqlite`. Set the same absolute `DATABASE_PATH` for migrations, CLI, and UI to share a different database.

Migrations are explicit: CLI commands and the UI do not apply them automatically. After installation or upgrades, run `matis-migrate` before using `--save`, indexing, listing projects, or starting the UI. From the checkout root, use `cargo run --manifest-path matis-rust/Cargo.toml --locked --bin matis-migrate`; from `dashboard/`, `bun run db:migrate` runs the same migration command. Back up existing databases before upgrading. The normalized-storage upgrade preserves project registrations but drops legacy graph blobs without backfilling; reindex those projects after migrating. Reports and checks without `--save` need no database or migrations.

The UI has **no authentication**; keep it on a trusted local network.

## CLI and AI-agent integration

Run against the whole target project, not just changed files:

```sh
matis report -f .   # Show measurements
matis check -f .    # Enforce limits; use in CI
matis-migrate      # Migrate before database-backed commands
matis index -f .    # Register/update and save the graph
matis projects     # List registered projects as JSON
```

Reports and checks analyze current sources without saving. Both default to Markdown; add `--format json` for structured output or `--save` to store the graph. The folder defaults to `.`.

`matis check` exits 0 when checks pass and 1 when findings exist. `matis report` does not enforce limits.

For an AI coding agent with shell access, add this instruction to its project rules (for example, `AGENTS.md`):

```text
After changes, run `matis report -f .` and `matis check -f .` from the target repository root.
Review measurements and configured-check findings; fix relevant findings and rerun.
Analyze the entire project. Do not treat passing checks as a substitute for tests.
```

Agents use the same CLI; no native agent plugin or MCP server is provided.

## Contribute

Keep changes focused, add regression tests for analyzer behavior, and open a pull request against `main`.

`matis-rust/src/matis/` separates parsing, metrics/configured checks, domain contracts, CLI, services, and persistence. `dashboard/` contains the Bun/React UI and server routes. Rust owns database migrations; Bun reads normalized graph facts and registers repositories in the shared database.

From the repository root:

```sh
(cd dashboard && bun install)

cargo test --manifest-path matis-rust/Cargo.toml --locked --all-targets
cargo build --manifest-path matis-rust/Cargo.toml --release --locked --bins
cargo fmt --manifest-path matis-rust/Cargo.toml --check
cargo clippy --manifest-path matis-rust/Cargo.toml --locked --all-targets -- -D warnings
(cd dashboard && bun run build)
(cd dashboard && bun test)
(cd dashboard && bun run typecheck)
(cd dashboard && bun lint --fix)
matis-rust/target/release/matis report -f .
matis-rust/target/release/matis check -f .
```

Rust tests run in parallel by default. Tests must isolate files, databases, and ports. Dashboard integration tests require Cargo and build the Rust CLI plus a test-only artifact helper once per test process.

CI runs Rust checks for `matis-rust/` changes and dashboard checks for changes to either `dashboard/` or `matis-rust/`. Component workflows also run when their own files change. Matis CI runs a full-project `matis check --f . --format json` on pushes and pull requests to `main`; its exit status enforces configured limits. `.matisignore` excludes dashboard sources, which have separate TypeScript checks. Architecture tests enforce persistence boundaries.

The dashboard uses TypeScript 7 for `tsc` via the `@typescript/native` alias, while ESLint uses the TypeScript 6 compatibility API via the `typescript` alias. This follows Microsoft's [side-by-side setup](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/#running-side-by-side-with-typescript-6.0) until typescript-eslint supports the new compiler API.

Coffee samples in `matis-rust/tests/code_samples/` intentionally contain complexity/size/parameter violations and import cycles; do not simplify them or remove their reference-note ledgers. Tests compare graphs and reports with checked-in snapshots in `matis-rust/tests/snapshots/`. Review fixture and analyzer changes before updating snapshots. Python samples remain supported analysis inputs; the Python importer has been removed.

For schema changes, update Rust persistence models and add a SQL revision in `matis-rust/src/matis/persistence/migrations/versions/`. Register the revision in `matis-rust/src/matis/persistence/migrations/mod.rs`, review it, and apply it with `matis-migrate`. Report security issues as described in [SECURITY.md](SECURITY.md).
