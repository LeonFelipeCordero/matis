# Metrics follow-up review

Only the five owned regression test files changed in this follow-up. Production code, configuration, and other tests were left untouched. The shared checkout already contained production/configuration changes. No mutators or commits were run.

## Evidence and classification

Indices below are the original indices from `rerun-mapping.json`, not positions in the selected rerun list. I checked the selected manifests and recorded outcomes against `rerun-{0..3}.jsonl`. “Distinguishing” below means the new assertion logically separates the recorded replacement from current code. It does not mean a new mutation execution confirmed a kill.

| Original index | Recorded rerun | Classification and new evidence |
| --- | --- | --- |
| 87 | survived, rerun-0 | Defensive half-open range contract; not claimed to be a demonstrated valid-source resolution bug. `callable_selection.py:286` changes candidate start `< node.end_byte` to `<=`. Added `test_annotation_range_excludes_type_starting_at_its_end` in `test_adapter_regression_contracts_a.py`. A generic type at byte 30 must not belong to a declaration ending at byte 30. The direct node-range assertion distinguishes the replacement. The prior test had whitespace/declaration syntax between declarations and did not hit exact equality. No supported-parser source example that changes final type proof was established; do not count this as a proven end-to-end gap or declare global equivalence. |
| 110 | survived, rerun-3 | Defensive recovered-syntax gap, not equivalent. The actual manifest changes `pattern is None or pattern.type != "arguments"` in `elixir.clause_decision` at line 68. It is a clause-pattern guard, not a declaration-head guard. Added `test_elixir_recovered_clause_without_pattern_is_conservatively_refutable` in `test_adapter_regression_syntax_a.py`. The real parser produces a `stab_clause` without `left` for `case x do\n -> :ok\nend`. Current code returns True; the replacement accesses `None.type` and raises. Valid guarded clauses never exercise the missing-pattern branch. |
| 277 | survived, rerun-1 | Valid-source behavior gap, not equivalent. Added `test_kotlin_if_inside_while_body_is_not_an_else_if` in `test_adapter_regression_syntax_a.py`. A real `if_expression` inside a while control body must return False. Changing the conjunction before `owner is not None` to `or` lets the preceding true conditions return True without requiring an if-expression grandparent or alternative field. Prior nested-if examples used block bodies and did not hit this shape. |
| 842 | survived, rerun-1 | Scope-width contract gap, not equivalent. Added the `(120, 180, True)` case in `test_shadowing_uses_width_not_endpoint_sum_for_nonzero_scope`, in `test_receiver_mutation_regressions.py`. Receiver scope is `[100,200)`, width 100, and the competing scope is `[120,180)`, width 60; position 150 is visible in both. Current result is True. Replacing the left subtraction with addition compares 300 <= 100 and returns False. |
| 844 | survived, rerun-3 | Scope-width contract gap, not equivalent. The same test adds `(0, 250, False)` against receiver scope `[100,200)`. Current comparison is 250 <= 100, so False. Replacing the right subtraction with addition compares 250 <= 300 and incorrectly returns True. Existing receiver scopes starting at zero made right subtraction and addition equal. |
| 916 | survived, rerun-1 | Valid-source static-field classification gap, not equivalent. Extended `test_receiver_bindings_on_real_class_grammars` in `test_shared_syntax_mutation_regressions.py` with `public final Service service` and `public static Service shared`. Java wraps these keywords in a `modifiers` child. The first field must remain non-static. Changing the conjunction at line 109 to `or` accepts any modifiers node, including public/final without static. Prior bare instance fields had no modifiers node. |
| NEW, no original index | survived, rerun-3 | Valid-source write-filter gap, not equivalent. Manifest ID `20eb0775f8e08f3b2ae2e4577e21189872a1a6b163ec8750ed6b3dc256577cfa`, rule `logical.or-to-and`, `receiver_resolution.py:252`. There is no matching entry in `rerun-mapping.json`. Added `test_javascript_nested_identifier_assignment_is_not_a_python_local_write` in `test_receiver_ownership_regressions.py`. A nested JavaScript function assigns the enclosing `service` variable. `_path_written` must remain True. Original code bypasses Python-only filtering for every JavaScript node. The replacement permits a JavaScript assignment into that filter and incorrectly drops this write as nested-local. Existing Python assignment cases make both guard terms false and therefore cannot distinguish or from and. |
| 265 | timeout, rerun-3 | Non-equivalent termination defect; not a survivor and not a confirmed assertion kill. The JavaScript var-scope loop changes `scope.parent and scope.type not in FUNCTIONS` to `or`. A function scope with a parent no longer stops there, and a root scope outside FUNCTIONS can keep requesting its own enclosing scope. Existing `test_javascript_var_scope_stops_at_function_not_its_parent` already covers inner var, block-local let, and root var on current code. I did not run the hanging replacement or add another test that would reach the same unbounded loop. |

## Checks

Focused command: `cd matis && uv run pytest tests/metrics/test_adapter_regression_contracts_a.py tests/metrics/test_adapter_regression_syntax_a.py tests/metrics/test_receiver_mutation_regressions.py tests/metrics/test_shared_syntax_mutation_regressions.py tests/metrics/test_receiver_ownership_regressions.py -q` — 157 passed.

Full importer suite: `(cd matis && uv run pytest -n 4)` — 2220 passed. Four isolated workers were used as permitted by the repository instructions.

`(cd matis && uv run pyright)` — 0 errors, 0 warnings.

`(cd matis && uv run ruff check .)` — passed.

`uv run --project matis matis check --f .` — passed against the entire shared checkout.

Logs: `followup-metrics-pytest.log`, `followup-metrics-pyright.log`, `followup-metrics-ruff.log`, and `followup-metrics-matis-check.log` in this directory.

All five owned files remain below 600 lines. Added functions stay below 100 lines, three parameters, and complexity eight; the whole-project Matis check passes.

No mutation rerun was performed. The recorded outcomes remain the historical survived/timeout outcomes above. Dashboard checks were not run for this Python-test-only follow-up.
