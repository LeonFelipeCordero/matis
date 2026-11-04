# Deterministic mutation manifests (phase two)

`matis mutations -f PROJECT` analyzes current sources without saving, then emits JSON
single-edit manifests. `matis mutations -f PROJECT --saved` derives the same output
from that folder's indexed candidates, verifying against current source; it never
refreshes the index. `matis mutations --rules` prints versioned language/token
coverage without analyzing files or opening SQLite.

Only phase-one candidates are persisted, in `graph_mutation_candidates`. Manifests
are derived data: no mutation tables, sessions, ledger or extra persistence lifecycle.
Disabling a rule does not remove candidates. The report API is
`matis.mutations.report.build_mutation_report(root, report, config)`, accepting either
`build_artifact_report(...)` or `RepositoryService.report(...)` output.

## Configuration

The analyzed folder's existing `matis.json` accepts an optional `mutations` section
alongside the flat quantitative check limits. No parent-folder search. Missing
configuration, omitted `mutations.rules`, and `[]` select no mutations.
`mutations.rules` is a list of rule IDs; duplicates are harmless. Selecting an ID
activates all its applicable language mappings. Unknown IDs and unknown
configuration keys fail. The old rule-to-replacement dictionary is rejected;
replacement tokens are defined by the catalog, not supplied in configuration.

```json
{
  "maxFunctionLines": 100,
  "mutations": {
    "rules": [
      "arithmetic.add-to-subtract",
      "comparison.equal-to-not-equal",
      "logical.and-to-or"
    ]
  }
}
```

Inspect coverage or generate manifests from the project root:

```sh
matis mutations --rules      # List versioned language/token mappings; no database needed
matis mutations -f .         # Analyze current sources without saving
matis index -f .             # Save candidates (run matis-migrate first)
matis mutations -f . --saved # Use saved candidates, verifying current source
```

Saved mode never refreshes the index. Reindex after source changes; stale or
missing source fails the entire request rather than producing partial output.

## Version-one catalog

In the table, **common** means JavaScript/JSX, TypeScript, TSX, Java, Kotlin,
Python, Rust, Go and Elixir. JSX uses the JavaScript adapter. **Assignment** means
the common languages except Elixir; **strict equality** means JavaScript/JSX,
TypeScript, TSX and Elixir.

| Rule ID | Token change | Languages |
| --- | --- | --- |
| `arithmetic.add-to-subtract` | `+` → `-` | Common |
| `arithmetic.subtract-to-add` | `-` → `+` | Common |
| `arithmetic.multiply-to-divide` | `*` → `/` | Common |
| `arithmetic.divide-to-multiply` | `/` → `*` | Common |
| `comparison.equal-to-not-equal` | `==` → `!=` | Common |
| `comparison.not-equal-to-equal` | `!=` → `==` | Common |
| `comparison.less-to-less-equal` | `<` → `<=` | Common |
| `comparison.less-equal-to-less` | `<=` → `<` | Common |
| `comparison.greater-to-greater-equal` | `>` → `>=` | Common |
| `comparison.greater-equal-to-greater` | `>=` → `>` | Common |
| `comparison.less-to-greater` | `<` → `>` | Common |
| `comparison.greater-to-less` | `>` → `<` | Common |
| `comparison.strict-equal-to-strict-not-equal` | `===` → `!==` | Strict equality |
| `comparison.strict-not-equal-to-strict-equal` | `!==` → `===` | Strict equality |
| `logical.and-to-or` | Python `and` → `or`; others `&&` → `\|\|` | Common except Elixir |
| `logical.or-to-and` | Python `or` → `and`; others `\|\|` → `&&` | Common except Elixir |
| `logical.strict-and-to-or` | `and` → `or` | Elixir |
| `logical.strict-or-to-and` | `or` → `and` | Elixir |
| `logical.truthy-and-to-or` | `&&` → `\|\|` | Elixir |
| `logical.truthy-or-to-and` | `\|\|` → `&&` | Elixir |
| `assignment.add-to-subtract` | `+=` → `-=` | Assignment |
| `assignment.subtract-to-add` | `-=` → `+=` | Assignment |
| `assignment.multiply-to-divide` | `*=` → `/=` | Assignment |
| `assignment.divide-to-multiply` | `/=` → `*=` | Assignment |

Arithmetic rules apply only to binary operators: unary signs and pointer
dereferences are excluded. Elixir assignment/match semantics are not compound
arithmetic assignment. Strict equality rules never convert to loose equality;
Kotlin referential equality is deliberately not selected. Elixir logical rules
keep strict and truthy operator families separate.

Python chained comparisons change exactly one operator per independent manifest.
Multiword comparisons (`is not`, `not in`), membership, identity,
null-coalescing and all other unlisted operators
are excluded, not silently normalized. Condition candidates remain inspectable but
have no replacement rule: no whole-expression true/false literals, no iterable-for
conversion, and no language-agnostic boolean substitution. Phase-one conservative
Rust macro/token-tree and pattern exclusions remain in force.

Syntax support is **not** proof of type validity, successful compilation, changed
behavior or safety to execute. For example `/` may change numeric types, `+` may
concatenate, overloaded operators may differ, and logical changes may alter
short-circuit evaluation. There is no type inference, macro expansion or execution.

## Manifest contract

Each manifest carries `candidateId`, `sourceHash` (SHA-256 of the entire original
file), `ruleId`, `ruleVersion`, language/path, exact half-open UTF-8 byte `span`,
`originalText`, `replacement`, plus standard-base64 `originalBytes` and
`replacementBytes`. `id` is SHA-256 of canonical UTF-8 JSON of every other manifest
field (sorted keys, compact separators, non-ASCII preserved). Thus identity includes
source, candidate, versioned rule and replacement; filesystem root is not included.
The report envelope has `schemaVersion: 1` and a `mutations` list.

For Python `x = a + b\n`, the add rule yields a manifest with
`span.startByte: 6`, `span.endByte: 7`, `originalText: "+"`,
`originalBytes: "Kw=="`, `replacement: "-"`, `replacementBytes: "LQ=="`,
plus the exact hashes/identity and one-based UTF-8 byte line/column coordinates.

Every input candidate is re-verified using the phase-one extractor, even when its
rule is disabled: source hashes, candidate identity, text/spans, callable context
and syntax provenance must match. Each proposed replacement is reparsed in memory;
its replacement must remain one exact operator token of the same family in valid
syntax, rejecting lexical collisions such as JavaScript `a+--b` → `a---b` or
`a*/regex/` → `a//regex/`. This never writes modified source to disk.
Missing files, unsafe paths, symlinks escaping
the project root and stale/forged candidates fail the entire request. This validates
a read-time snapshot, not a guarantee that a file cannot change afterward.

Output is sorted by path, span, rule, replacement and ID, independent of input or
configuration ordering. Repeated identical byte edits are deduplicated; no no-op
edits are emitted. Nested candidates stay independent. Manifests are alternatives
against the original file, never a composed patch or an instruction to apply them
sequentially.

Manifest inspection remains read-only. For opt-in execution with explicit test
targets, see [Autonomous mutation execution](MUTATION_RUNNER.md). The runner does
not create a workspace, run baselines, or require an AI agent or dashboard.
