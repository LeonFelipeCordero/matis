# Architecture decision: short workflows, explicit ownership

## Decision

Keep one Rust crate. Organize execution by capability: analysis, mutations, saved navigation, and repository management. Use plain functions for computation and existing repository ports for storage. Construct storage-backed services only in `bootstrap.rs`.

Matis discovers source files, extracts facts, resolves relationships, calculates metrics, and optionally saves a normalized SQLite graph. The dashboard reads that graph directly through Bun. It does not call the Rust CLI in production. Rust owns database migrations.

The analysis stages have real dependencies. Removing those dependencies or hiding them behind a pipeline framework would not make the system easier to understand. Instead, commands must run only the workflow they need.

## Alternatives explored

Four independent inspections covered feature modules, flatter execution, a functional core with explicit IO, and test ownership. The implementation combines the useful parts without a repository-wide move.

| Option | Finding code | Making a change | Required context | Decision |
| --- | --- | --- | --- | --- |
| Move every layer into feature folders | Better feature locality | Large import and test churn; shared analysis still crosses features | Fewer directory jumps, same underlying dependencies | Defer broad moves; useful for a later navigation-only change |
| Flatten workflows and remove forwarding | Clear execution entry points | Small changes at the actual owner | Mutation work no longer requires understanding graph analysis | Implement now |
| Functional core with explicit IO | Clear calculation/storage distinction | Good for isolated rules; path resolution needs filesystem semantics | Plain inputs reduce context | Keep as a boundary rule; defer resolver and dashboard extraction |
| Full hexagonal rewrite or separate crates | More explicit interfaces | More ports, models, composition, and compatibility work | More files to read for one operation | Reject without a concrete need |

These are qualitative code-inspection findings, not measured token or performance scores.

## Execution map

```text
CLI
 ├─ index / report / check
 │    indexer::analyze_repository
 │      discovery → parsing → callable metrics → resolution → artifact
 │      ├─ metrics::report → metrics::check
 │      └─ RepositoryService → RepositoryProtocol → SQLite
 │
 ├─ mutations
 │    mutations::candidates::collect_candidates
 │      discovery → candidate extraction
 │      → verified manifests → optional restoring runner
 │
 ├─ mutations --saved
 │    RepositoryService::saved_folder_candidates
 │      → repository port → graph existence + ordered candidate rows
 │      → verified manifests → optional restoring runner
 │
 └─ graph
      NavigationService → NavigationReader → read-only revision snapshot

bootstrap: opens concrete stores and constructs services
services: select/validate requests; never construct storage
persistence: owns queries, transactions, atomic writes, and migrations
```

Mutation generation no longer builds declarations, callable metrics, resolved relationships, graph artifacts, or metrics reports. Saved generation does not load unrelated nodes, edges, diagnostics, or metrics. Candidate verification still reads current sources and rejects stale or invalid sites, including when no rules are enabled.

Graph analysis still extracts candidates for persisted graphs and reports. The candidate-only workflow uses the same discovery and extraction functions, source bytes, ignore rules, ordering, and identity format.

## Where to start

Paths below are under `matis-rust/src/matis/`.

| Change | Start here | Follow only when needed |
| --- | --- | --- |
| Command options or output | `cli/schema.rs`, the command module | Feature entry point |
| Analysis execution | `indexer.rs` | `parsing/`, `metrics/` |
| Mutation discovery | `mutations/candidates.rs` | `parsing/mutation_candidates.rs` |
| Candidate identity and records | `domain/mutation_candidates.rs` | Extraction and manifest consumers |
| Mutation replacements or safety | `mutations/rules.rs`, `mutations/manifests.rs` | `workspace.rs`, `runner.rs`, `process.rs` |
| Saved candidates or reports | `services/repositories.rs` | `domain/repositories.rs`, `persistence/graph_queries.rs` |
| Service construction or database selection | `bootstrap.rs` | Concrete stores |
| Saved graph traversal | `services/navigation.rs` | Walk/source helpers and `NavigationReader` |
| Dashboard graph behavior | `dashboard/src/lib/project-graph*.ts` | Graph components and graph store |

Candidate identity helpers live with candidate records, below extraction and mutation execution. Parsing must not import the mutation workflow. `services/factories.rs`, `services::repositories::analyze_folder`, and `mutations::{canonical_json,digest}` remain compatibility reexports, not alternate implementations. New analysis callers use `indexer::analyze_repository` directly.

## Compatibility

CLI arguments, manifest JSON, candidate hashes, reports from valid normalized graphs, SQLite schema, and dashboard contracts stay unchanged. Service construction does not migrate storage implicitly. Atomic writes, snapshot lifetimes, bounded navigation, containment checks, locks, and mutation restoration stay in their current owners.

The Rust free function `services::repositories::saved_folder_report(root)` becomes an explicit service operation. Library callers must select their database at the composition boundary:

```rust
let mut service = matis::bootstrap::repository_service(database_path)?;
let report = service.saved_folder_report(root)?;
```

`RepositoryProtocol::mutation_candidates` has a report-facts fallback so existing implementations still compile. An absent candidate field means empty legacy data; a present non-array returns an error. The SQLite store overrides it with a candidate-only transaction. Fresh mutation commands intentionally stop depending on unrelated graph-analysis failures. Saved mutation commands still reject absent or unrevisioned graphs.

Saved reports use the shared normalizer. Library-provided facts must have a node array. Missing or empty `edges` can fall back to legacy `imports`; explicitly malformed edge fields return an error instead of hiding relationships. Normalized SQLite reads already meet these requirements.

## Test ownership

A repeated assertion is not automatically a duplicate test. The same value can cross different failure boundaries. Keep tests that prove different contracts; remove repeated scenarios at the same boundary.

| Behavior | Authoritative evidence |
| --- | --- |
| CLI syntax and validation | Direct parser tests; no indexing or child process |
| Language extraction and graph semantics | In-memory tests and language samples/snapshots |
| Dependency reachability | Existing independent walk oracle, including the exact diamond/cycle/duplicate/self-edge topology |
| Candidate-only parity | One expanded CLI library contract: ordered multi-language candidates, ignores, raw-byte hash, full-analysis parity, saved parity, manifests, and stale-source rejection |
| Candidate-only database reads | Real database contract with unrelated report storage unavailable; graph existence, ordering, and JSON errors |
| Saved folder selection | Shared report/candidate scenario for URL exclusion, ambiguity, missing registration, and unindexed errors |
| Saved replacement | Existing reindex scenario; old candidates disappear after changed/deleted sources |
| Composition and workflow independence | One architecture test per boundary, with shared detector fixtures |
| Migration idempotence | Existing migration contract, not repeated roundtrip setup |
| Filesystem/process safety and Rust/Bun interoperability | Existing integration contracts; not replaced by in-memory tests |

The manually asserted dependency-metric test is removed only after its exact topology moves into the stronger oracle. The redundant second migration call is removed from the roundtrip test. New tests cover new execution and storage boundaries, not wrappers.

Mutation targets for entrypoints, candidate extraction, mutation workflows, mutation CLI, persistence, and repository services now include their relevant integration owners. Their timeout is 1200 seconds: verification found native grammar rebuilds taking over the old 300-second limit. Targeted campaigns remain optional and do not replace component validation.

## Deferred work

Do not move every file, introduce new crates, add generic pipeline stages, or add ports around pure functions. Do not combine this change with borrowed resolver indexes, shared syntax-tree caching, filesystem-independent resolution, or dashboard projection extraction. Those require separate evidence and compatibility review. In particular, lexical path normalization must not silently replace symlink-aware resolution.
