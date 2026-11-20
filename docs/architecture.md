# Architecture: services own table repositories

## Dependency direction

Keep one Rust crate and one dashboard package. Commands, routes, and business workflows use capability services. Each table has one repository entry point. Only its owning capability service can call that repository through its domain port.

```text
CLI / API route
  → operation workflow
    → capability services
      → owned repository ports
        → table repositories
          → SQLite
```

Services can call other services, but must not form a dependency cycle. Repositories cannot call another repository or a service. Services and workflows cannot use ORM entities, query builders, or database handles. Bootstrap code constructs repositories and injects them into their owning services; it does not perform table operations.

Rust owns schema migrations. Migration code and migration metadata are exceptions to table repository ownership. The dashboard reads the shared SQLite graph through Bun and registers repositories. It does not invoke the Rust CLI in production. Dashboard tests build the CLI and a test-only artifact helper.

## Ownership

Each table below has its own repository. NodeService owns four repositories because files, symbol details, and decorators describe a node. It does not own relationships, metrics, diagnostics, or mutation candidates.

| Table | Owning service | Rust service / repository module | Dashboard service / repository module |
| --- | --- | --- | --- |
| `repositories` | RepositoryService | `repositories` / `repositories` | `repository-service` / `repository-store` |
| `repository_graphs` | GraphRevisionService | `graph_revisions` / `revisions` | `graph-revision-service` / `graph-revision-repository` |
| `graph_nodes` | NodeService | `nodes` / `nodes` | `node-service` / `node-repository` |
| `graph_files` | NodeService | `nodes` / `files` | `node-service` / `file-repository` |
| `graph_symbols` | NodeService | `nodes` / `symbols` | `node-service` / `symbol-repository` |
| `graph_node_decorators` | NodeService | `nodes` / `decorators` | `node-service` / `decorator-repository` |
| `graph_edges` | RelationshipService | `relationships` / `relationships` | `relationship-service` / `relationship-repository` |
| `graph_node_metrics` | NodeMetricsService | `node_metrics` / `metrics` | `node-metrics-service` / `node-metrics-repository` |
| `graph_diagnostics` | DiagnosticService | `diagnostics` / `diagnostics` | `diagnostic-service` / `diagnostic-repository` |
| `graph_mutation_candidates` | MutationCandidateService | `mutation_candidates` / `candidates` | Not accessed |

Rust paths are under `matis-rust/src/matis/services/` and `persistence/`. Dashboard paths are under `dashboard/src/lib/services/` and `persistence/repositories/`.

Repository operations are specific to their table. A repository cannot query another owner's table, including through a join. Services combine batched results inside a shared transaction. Keep parameterized queries, bounded navigation reads, stable ordering, and SQLite bind limits.

## Workflows, not a GraphService

There is no service that owns all graph repositories. Small workflows compose capability services for one operation.

```text
Rust bootstrap.rs
  ├─ RepositoryWorkflow
  │    ├─ RepositoryService
  │    └─ SavedGraphWorkflow
  │         → revision / node / relationship / metric / diagnostic / candidate services
  └─ NavigationService
       → CapabilityReader
         → capability services on one read-only snapshot

Dashboard repositories.servers.ts
  → persistence/repositories/factory.ts (composition and connection lifetime)
    ├─ registration / list → RepositoryService
    └─ architecture → graphView → capability services
                         └─ graphDetails → node / relationship / metric services
```

`RepositoryWorkflow` retains the command-facing operations for registration, indexing, saved reports, and saved mutation candidates. `SavedGraphWorkflow` coordinates replacement order, graph summaries, and report facts. These workflows do not access table ports or ORM types.

`NavigationService` retains request validation, graph traversal, and source containment checks. `CapabilityReader` implements the navigation read contract using services, not database queries. The dashboard graph-view and graph-details workflows own projection and metric assembly; persistence no longer owns those business rules.

Analysis stays in `indexer::analyze_repository`: discovery → parsing → callable metrics → relationship resolution → artifact. Pure calculations do not need repository ports. Fresh mutation discovery stays independent of graph analysis. Saved mutation discovery reads revision existence and candidate rows without loading unrelated graph facts.

## Transactions and snapshots

Rust's private persistence `Session` is shared by the repositories built in `bootstrap.rs`. The domain `UnitOfWork` opens a scope. A successful workflow commits it; dropping an unfinished scope rolls it back. Services do not commit independently and business callers never receive a connection.

Registration plus its initial graph save is atomic. Graph replacement clears old edges before nodes, relies on declared cascades for dependent rows, then writes all new facts before committing. Validation and foreign-key failures preserve the previous graph. Source analysis runs outside the write transaction.

Rust navigation opens one read-only revision snapshot and retains it until the reader is dropped. Project selection, revision checks, node details, relationships, and diagnostics use that same snapshot. It never creates or migrates storage.

Dashboard composition opens one explicit deferred transaction for the complete graph view. Table services receive repositories bound to that transaction. Repository listing also combines registration and revision data within one snapshot. Do not replace these scopes with a separate transaction for each service call.

## Where to change code

| Change | Start here |
| --- | --- |
| CLI syntax or output | `matis-rust/src/matis/cli/` |
| Analysis execution | `matis-rust/src/matis/indexer.rs` |
| Table reads or writes | Its sole repository in the ownership table |
| Validation or capability behavior | Its owning service |
| Saved graph replacement, summaries, reports | `services/saved_graph.rs` and `services/repository_workflow.rs` |
| Storage construction or database selection | Rust `bootstrap.rs`; dashboard repository `factory.ts` |
| Saved graph traversal | Rust `services/navigation*.rs` |
| Dashboard graph response | `services/graph-view.ts` and `services/graph-details.ts` |
| Graph projection calculations | Dashboard `project-graph*.ts`, `graph-analysis.ts`, and graph helpers |
| Dependency rules | [`matis.json`](../matis.json); evaluator in `matis-rust/src/matis/metrics/architecture.rs` |
| Ownership guardrails | Rust `tests/integration/architecture/ownership.rs`; dashboard `tests/database-*` |

## Compatibility

CLI arguments, normalized SQLite schema, graph revisions, report JSON, and dashboard API responses stay unchanged. Opening a service does not implicitly migrate storage.

The Rust library API changes intentionally: persistence is private, `RepositoryProtocol` is removed, and the registration-only `RepositoryService` no longer exposes graph workflows. Existing command-facing callers use the workflow returned by bootstrap:

```rust
let mut workflow = matis::bootstrap::repository_service(database_path)?;
let report = workflow.saved_folder_report(root)?;
```

Custom library integrations use the capability ports in `domain/storage.rs` and the corresponding service constructors. They must supply a shared transaction scope for multi-capability operations. There is no public raw-store escape hatch.

## Checks and test ownership

Run `matis check --folder .` from the repository root. CI runs the same check with `--format json`. The `forbiddenDependencies` rules in [`matis.json`](../matis.json) reject these direct Rust dependencies:

| From | Must not depend on |
| --- | --- |
| `cli/` or `services/` | `persistence/` |
| `persistence/` | `services/` |
| `domain/` | `cli/`, `services/`, or `persistence/` |

Paths are relative to `matis-rust/src/matis/` in the table. `bootstrap.rs` is outside those source scopes so it can wire persistence implementations into services. Both persistence and services can use domain ports.

Each rule matches repository-relative `from` and `to` globs against direct, resolved dependencies between files, including type-only imports. The check reports the source file and line when available, the target file, and the rule message. It does not follow indirect paths or detect unresolved dependencies. See the [rule format](../DESIGN.md#architecture-dependency-checks).

Keep the architecture tests. The dependency graph cannot enforce exclusive table access or owner-service port access. Rust architecture tests also enforce private persistence and acyclic service dependencies. The root Matis check excludes `dashboard/` through `.matisignore`; dashboard architecture tests enforce its boundaries separately, including relative and aliased imports and storage-handle forwarding.

Use existing behavioral contracts for registration conflicts, replacement rollback, revision consistency, bounded navigation, ordered reports and candidates, and Rust/Bun interoperability. Batch-read tests cover scopes above SQLite's conservative bind limit. Keep unit tests in memory; real database and process contracts stay in integration tests.

Run the component checks in `AGENTS.md`. Executable changes also require one whole-repository Matis check. Mutation targets cover the moved domain validation and capability-service modules; run only relevant targets when mutation testing is needed.

Do not introduce a generic CRUD layer, a pipeline framework, another crate, or a second application server for this boundary. Do not move pure analysis into services merely to match a folder pattern.
