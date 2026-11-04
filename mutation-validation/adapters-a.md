# Adapter group A mutation regression review

Scope: NEW tests only for base, callable_selection, elixir, generic, go, javascript, kotlin. Production, existing tests, and config unchanged by this work. No commit.

Indices below are exact **zero-based full-run results.jsonl record indices**, joined to baseline manifests.json by manifestId; source lines are baseline one-based span.startLine. Reviewed all 44 assigned survivors.

New files:
- matis/tests/metrics/test_adapter_regression_syntax_a.py
- matis/tests/metrics/test_adapter_regression_contracts_a.py

Validation: `cd matis && uv run pytest tests/metrics/test_adapter_regression_syntax_a.py tests/metrics/test_adapter_regression_contracts_a.py -q` — 72 passed in 0.21s. Ruff format/check limited to these new files passed. No existing tests, full suite, typecheck, matis check, dashboard checks, or mutation execution were run, as requested.

“Coverage added” means a passing baseline assertion exercises the relevant contract; it is NOT a verified mutation kill. No survivor is claimed fixed without a rerun. Indices 87, 110, 248 are explicitly not claimed covered against their mutations.

| Full-run index | Source:line | Mutation | Classification / evidence |
|---:|---|---|---|
| 34 | base.py:99 | `and` → `or` | Coverage added (kill unverified). Syntax: nested Java consequent versus direct else-if. |
| 53 | callable_selection.py:131 | `or` → `and` | Coverage added (kill unverified). Syntax: Kotlin trailing lambdas both with and without parenthesized arguments. |
| 55 | callable_selection.py:146 | `or` → `and` | Coverage added (kill unverified). Contract: reject non-argument nodes and recovered empty value arguments. |
| 58 | callable_selection.py:155 | `or` → `and` | Coverage added (kill unverified). Contract: missing and parser-error parameter declarations remain unknown. |
| 60 | callable_selection.py:158 | `or` → `and` | Coverage added (kill unverified). Contract: a real non-callable root has no parameter container. |
| 61 | callable_selection.py:184 | `or` → `and` | Coverage added (kill unverified). Syntax: Java spread parameter obtains its name from variable_declarator. |
| 72 | callable_selection.py:231 | `==` → `!=` | Coverage added (kill unverified). Syntax: Java/Kotlin character literals retain language-specific type names. |
| 73 | callable_selection.py:233 | `==` → `!=` | Coverage added (kill unverified). Syntax: Java/Kotlin boolean literals retain language-specific type names. |
| 79 | callable_selection.py:254 | `-` → `+` | Coverage added (kill unverified). Syntax: Kotlin shadowed value uses inner Int, then outer String. |
| 85 | callable_selection.py:286 | `<=` → `<` | Coverage added (kill unverified). Syntax: nullable annotation at its own start remains unproven. |
| 87 | callable_selection.py:286 | `<` → `<=` | DEFERRED boundary: no supported declaration/binding whose annotation starts exactly at its exclusive end was identified. No artificial adjacent annotation endpoint test added. |
| 91 | callable_selection.py:311 | `or` → `and` | Coverage added (kill unverified). Contract: annotation-only, node-only and fully absent type proofs remain unknown. |
| 100 | callable_selection.py:338 | `or` → `and` | Coverage added (kill unverified). Contract: unknown actual/expected and equal types cannot prove a mismatch. |
| 110 | elixir.py:68 | `or` → `and` | NO demonstrated distinction on supported syntax: guarded clause left is a binary operator with two named children, so the later length check still returns True. Guarded/plain wildcard behavior is tested; missing-pattern recovery is not asserted. |
| 118 | elixir.py:87 | `or` → `and` | Coverage added (kill unverified). Contract: field-based argument lookup and documented direct-child fallback. |
| 121 | elixir.py:104 | `or` → `and` | Coverage added (kill unverified). Syntax: ordinary nested call run(inner(1)) is not a declaration header. |
| 132 | elixir.py:162 | `and` → `or` | Coverage added (kill unverified). Contract: clause indexing tolerates missing declarations and empty argument containers. |
| 139 | elixir.py:184 | `and` → `or` | Coverage added (kill unverified). Syntax: only the right pipeline operand gains one argument; left call stays arity zero. |
| 141 | elixir.py:185 | `and` → `or` | Coverage added (kill unverified). Syntax: non-pipeline binary operands do not gain pipeline input. |
| 152 | elixir.py:225 | `or` → `and` | Coverage added (kill unverified). Contract: quoted, declared module, and stripped raw import paths. |
| 157 | elixir.py:266 | `or` → `and` | Coverage added (kill unverified). Contract: qualified dispatch forwards absent/empty/nonempty qualifiers as strings. |
| 164 | generic.py:27 | `and` → `or` | Coverage added (kill unverified). Syntax/contract: unwrap nested parentheses, retain calls, accept None. |
| 168 | go.py:46 | `and` → `or` | Coverage added (kill unverified). Syntax: real Go Owner.Run competes with Owner.Stop, Other.Run and package Run. |
| 170 | go.py:56 | `and` → `or` | Coverage added (kill unverified). Contract: absent declaration/receiver and empty receiver list reject safely. |
| 171 | go.py:57 | `and` → `or` | Coverage added (kill unverified). Syntax/contract: competing receiver types and missing receiver parameters. |
| 218 | javascript.py:64 | `<` → `<=` | Coverage added (kill unverified). Syntax: minified adjacent JavaScript declarations share an exact end/start boundary; export only the first. |
| 225 | javascript.py:140 | `or` → `and` | Coverage added (kill unverified). Syntax: side-effect/default/mixed imports are not named-only type imports. |
| 237 | javascript.py:185 | `and` → `or` | Coverage added (kill unverified). Syntax/API: a real box.alias assignment cannot become an identifier receiver alias. |
| 238 | javascript.py:185 | `==` → `!=` | Coverage added (kill unverified). Syntax/API: reject member-expression alias targets, retain identifier alias chains. |
| 241 | javascript.py:200 | `and` → `or` | Coverage added (kill unverified). Contract: reject unrelated method groups and non-method symbol groups. |
| 248 | javascript.py:213 | `<=` → `<` | OUTPUT-EQUIVALENT by inspection: replacing <= with < only continues the bounded loop when aliases equal receivers. Union adds nothing; later identical scans cannot change receivers or the result. No timing test added. |
| 255 | javascript.py:277 | `and` → `or` | Coverage added (kill unverified). Contract: class-member and prototype-member stability are independently required. |
| 256 | javascript.py:277 | `+` → `-` | Coverage added (kill unverified). Contract: prototype lookup uses Owner.prototype (string concatenation). |
| 265 | javascript.py:301 | `and` → `or` | Coverage added (kill unverified). Syntax: block var climbs to function, let stays block-local, top-level var stays program-local. Mutant may loop indefinitely at the root; not executed. |
| 266 | javascript.py:309 | `and` → `or` | Coverage added (kill unverified). Contract: relative slash paths resolve; package/tools and dot-only do not, even with matching local files/index. |
| 273 | kotlin.py:27 | `==` → `!=` | Coverage added (kill unverified). Syntax: Kotlin if nodes and all real non-if nodes in nested/else-if source. |
| 274 | kotlin.py:28 | `and` → `or` | Coverage added (kill unverified). Syntax: ordinary and nested Kotlin if are not alternatives. |
| 275 | kotlin.py:29 | `and` → `or` | Coverage added (kill unverified). Syntax: Kotlin alternative requires the control_structure_body wrapper. |
| 276 | kotlin.py:29 | `==` → `!=` | Coverage added (kill unverified). Syntax: ordinary function/block if versus wrapped else-if. |
| 277 | kotlin.py:30 | `and` → `or` | Coverage added (kill unverified). Syntax: Kotlin root and non-if traversal must not count as alternatives. |
| 278 | kotlin.py:31 | `and` → `or` | Coverage added (kill unverified). Syntax: control bodies beneath non-if owners do not count as alternatives. |
| 279 | kotlin.py:31 | `==` → `!=` | Coverage added (kill unverified). Syntax: actual if owner required, not function/block owner. |
| 280 | kotlin.py:32 | `and` → `or` | Coverage added (kill unverified). Syntax: then-branch nested if stays false; else-if stays true. |
| 281 | kotlin.py:32 | `==` → `!=` | Coverage added (kill unverified). Syntax: alternative field identity distinguishes then from else. |

## Remaining limits

No production fix was attempted. An exploratory valid Kotlin `run(label = 'x', *items)` call yielded `Argument(value='*items', spread=False)` for the spread expression. The new tests do not freeze this behavior or claim spread inference works. This is a potential existing limitation outside this tests-only task.

Defensive cases use missing syntax caches, recovered empty argument/receiver containers, and the documented argument-child fallback. No fake annotation-at-exclusive-end endpoint was introduced. Kotlin boolean cases and Java/Kotlin char cases use actual parsed nodes. Go uses parsed repository symbols and real receiver declarations. JavaScript export adjacency uses valid minified source, not impossible symbol offsets.
