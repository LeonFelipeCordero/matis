# Parsing follow-up review

Baseline indices below come from `manifests.json`, joined by originalIndex/freshId through `rerun-mapping.json` and `rerun-selected.json`. The supplied rerun logs record survival; no new mutation execution was performed.

## Changes

Only `matis/tests/parsing/test_campaign_core_regressions.py` changed. The adapter regression file needed no change. No production, configuration, README, or other test edits. No commit.

| Exact baseline indices | Classification | Evidence and test |
| --- | --- | --- |
| 26, 27 | Distinguishable declaration-head test gap | Extended `test_elixir_declaration_head_keeps_unguarded_calls` with valid `def a + b, do: {a, b}` and expected head `a + b`. Unlike an ordinary call, this head is a binary_operator without a `when` child. Either mutant incorrectly unwraps it to `a`. |
| 1282 | Target-selection gap | Rerun `mutation-candidates` runs only `tests/mutations` and `tests/test_coffee_mutation_candidate_snapshots.py`; it omits `tests/parsing/test_campaign_core_regressions.py`. Existing `test_named_operand_is_not_an_operator` covers the named `and` operand. No duplicate test or configuration change. |
| 1318 | Target-selection gap, with an additional attribution caveat | The same rerun command omits the parsing regression file. However, the exact manifest mutates `node.type == "unary_operator"` in `_elixir_excluded`, not the stab-pattern guard. The earlier attribution to `test_elixir_stab_body_is_not_a_pattern` is not sufficient evidence of a kill: that input contains no `@` expression. No duplicate test was added, as requested. Parent should verify this exact index after target correction; survival must not be declared resolved solely from the previous test mapping. |
| 1357 | Equivalent dead intermediate value; NOT a span-check mutation | Exact manifest ID `1b2bf6e58ad415ea0da8dcfc1b02fcd10cb8b8db1160a62aa79c7ac5c8bd551e`, byte 4114, line 129, changes `carrier.kind or str(item.kind)` in `_carrier_metadata`. `_structure_item` stores this in `metadata.kind`; its only consumer, `_canonical_item`, instead uses `carrier.kind or item.kind` directly. It never reads `metadata.kind`. Other metadata fields do not depend on this value. Existing independent endpoint/missing-span checks cannot distinguish an unrelated dead kind field. No artificial test added. |
| 1402 | Distinguishable helper fallback gap, masked in simple top-level traversal | Added `test_inheritance_constructor_without_fields_keeps_qualified_base`. A real, error-free Kotlin `constructor_invocation` from `class C : foo.Base<Int>() {}` has no value/function field. `_base_name(constructor, DEFAULT_POLICY)` must use the first named child and return `foo.Base`; the mutant yields None. Ordinary inheritance enumeration can recover by descending into user_type, which explains survival of the earlier top-level test. This is a direct real-node helper contract test, not a claim that the simple class loses its final inheritance edge. |
| 1403 | Distinguishable public inheritance gap | Extended `test_parameterized_inheritance_names` with Java `class C extends Outer<String>.Inner<Integer> {}` expecting `Outer.Inner`. Java generic_type has no name/type field. The mutant loses the nested `Outer` component during scoped-name composition, producing `.Inner` instead. A single generic base can be recovered by clause recursion, explaining why `Base<String>` did not distinguish it. |

The new expectations above follow the actual grammar nodes and source branches. They are not newly observed mutant kills; the supplied rerun predates these additions.

## Previously documented equivalents retained

No new artificial tests for these exact indices. Reasons remain scoped to valid supported syntax and actual callers, as documented in `review-other.md` / `mutation-validation/other-modules.md`.

| Exact baseline indices | Reason |
| --- | --- |
| 1158, 1160 | Go non-assignment literal parents still fail the subsequent owner/value-field checks. |
| 1164 | The extra index boundary requires invalid Go assignment arity. |
| 1233 | Valid Rust macro invocations supply a token_tree. |
| 1235 | Valid token trees have both delimiters; an empty payload produces no extracted calls either way. |
| 1250 | Valid Rust generic_function function field is its first named child. |
| 1293 | Qualified Elixir targets still fail the exact if/unless spelling allowlist. |
| 1302 | Only Kotlin callable nodes have the relevant direct simple_identifier name children. |
| 1386 | Supported generic/template callee fields already select the first named child. |
| 1396 | Actual callback carriers occupy the supported value/right position, not a name/key position. |

## Validation

All commands ran from the repository root against the shared checkout, without a mutator:

- `(cd matis && uv run pytest tests/parsing/test_campaign_core_regressions.py tests/parsing/test_campaign_adapter_regressions.py)` — 51 passed.
- `(cd matis && uv run pytest -n auto)` — 2223 passed. Log: `followup-parsing-pytest.log`.
- `(cd matis && uv run pyright)` — 0 errors, warnings, or informations. Log: `followup-parsing-pyright.log`.
- `(cd matis && uv run ruff check .)` — passed. Log: `followup-parsing-ruff.log`.
- `uv run --project matis matis check --f .` — whole-project analysis passed. Log: `followup-parsing-matis-check.log`.
- Dashboard production build, `bun test` (200 passed), `bun run typecheck`, and `bun lint --fix` — passed. Log: `followup-parsing-dashboard.log`.

Logs are under `/tmp/matis-mutation-validation/`. The changed test file remains below 600 lines; all its functions remain below 100 lines, at most three parameters, and complexity below eight.

No frontend behavior changed, so no browser smoke check was needed. Existing shared-checkout changes were left untouched. Remaining risk: target correction and an actual later mutation rerun must confirm outcomes, especially the incorrect previous attribution for 1318.
