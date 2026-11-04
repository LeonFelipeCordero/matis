# Other-module regression review

## Evidence and scope

Indices below are **zero-based JSONL record indices**, not the one-based progress labels. Reviewed `results.jsonl`, `manifests.json`, source at each relevant manifest span, all eight timeout logs, collection/internal/setup-only kill logs, and the campaign tail from index 1169 through 1583.

The campaign finished: 1584 records, 1373 killed, 203 survived, 8 timeout. `progress.log` ends with `Completed 1584 mutations in 15588.07s`; it contains exactly 1584 `Source restored:` messages. This review did not execute any mutator. Existing campaign outcomes remain unchanged; added tests are not a rerun score.

There are 58 survivors outside metrics/adapters and receiver/resolution files. New tests address 48 distinguishable behavior gaps. Ten survivors have valid-grammar equivalence/reachability limits documented below. All 37 tail survivors are accounted for. No production bug was confirmed. No production source, existing test, or matis.json was changed by this work. Other workers have concurrent changes in metrics adapters and receiver files; those are not part of this work.

## New test files and survivor mapping

Paths below are under `matis/tests/`. Test names are exact; parameterized cases cover independent conditions.

| Indices | New file and test | Contract |
|---|---|---|
| 26, 27 | parsing/test_campaign_core_regressions.py: test_elixir_declaration_head_keeps_unguarded_calls | Preserve ordinary declaration calls; unwrap only guarded heads. |
| 745 | metrics/test_campaign_other_regressions.py: test_either_external_marker_excludes_file_owner | Either external marker excludes ownership, including a file marked level=external. |
| 751 | same: test_one_missing_import_endpoint_is_ignored | Either missing endpoint excludes the import. |
| 760 | same: test_cycle_search_does_not_reparent_seen_or_queue_outside_neighbors | Check parents/queue directly, avoiding a mutation-induced path reconstruction hang. |
| 763 | same: test_representative_cycle_skips_self_outside_and_seen_neighbors | A self edge must not replace the representative multi-file cycle. |
| 978 | same: test_one_sided_backtick_padding | Both one-sided backtick cases require padding. |
| 1028 | same: test_language_family_uses_adapter_or_language_fallback | Preserve TypeScript's JavaScript family and generic C fallback. |
| 1108 | mutations/test_runner_campaign_regressions.py: test_recursive_glob_requires_final_filename | Recursive glob still requires the final filename. |
| 1116 | same: test_command_duration_and_process_group_contract | Mock monotonic timestamps; assert exact subtraction. |
| 1118, 1119 | same: test_source_rejects_each_unsafe_path_independently | Absolute inside-root path, inside-root `..` path, and escaping symlink each fail independently. |
| 1121, 1126 | same: test_hash_and_expected_token_are_independent_guards | Wrong hash with correct token and wrong token with correct hash both fail. |
| 1122, 1124, 1125 | same: test_exact_span_boundaries | Start=0 and end=len accepted; empty, negative, and oversized spans rejected with matching expected slices. |
| 1140 | same: test_total_duration_is_elapsed_time | Empty run reports exact elapsed duration. |
| 1173 | parsing/test_campaign_adapter_regressions.py: test_native_jsx_tags_are_not_components | Include lowercase boundary tags a and z. |
| 1181, 1182 | same: test_callback_parentheses_ignore_comments | Single callback in parentheses remains transparent; ignore comments. |
| 1184 | same: test_uninitialized_binding_is_not_a_callback | A real `let value;` declaration has no callback value. |
| 1185, 1187 | same: test_kotlin_base_name_only_accepts_user_type | Accept qualified user type; reject a non-user-type node. |
| 1198, 1199 | same: test_python_decorated_definition_keeps_its_carrier_and_decorators | Unwrap decorated definitions and preserve name/decorator. |
| 1211, 1212, 1213, 1214, 1215, 1224, 1228, 1230 | same: test_rust_macro_evaluation_respects_namespace_and_shadowing | Cover matching/nonmatching modules, imports/extern aliases, std/core exemptions, named/glob macro imports, unrelated imports, macro definitions, nonstandard namespaces, and default origin=None. |
| 1241 | same: test_rust_parameter_type_path_does_not_bind_module_name | A module name in a parameter type path is not a local binding. |
| 1246 | same: test_rust_payload_binding_rejects_only_matching_names | Named and glob imports independently shadow payload targets. |
| 1282 | parsing/test_campaign_core_regressions.py: test_named_operand_is_not_an_operator | Legal JavaScript identifier `and` is an operand, not a logical operator. |
| 1318 | same: test_elixir_stab_body_is_not_a_pattern | Exclude stab pattern arithmetic; retain body arithmetic. |
| 1357 | same: test_span_change_checks_each_endpoint_and_missing_span | Missing span and either changed endpoint trigger correction. |
| 1402, 1403 | same: test_parameterized_inheritance_names | Preserve Kotlin constructor, Python subscript, TS generic, and Java unfielded generic base targets. |
| 1406 | same: test_ineligible_header_never_parses | Unsupported kind and empty signature independently prevent parsing. |
| 1407 | same: test_header_retry_requires_both_error_and_policy | Valid complete header and invalid non-completing policy each parse only once. |
| 1415, 1416 | services/test_campaign_storage_walk_regressions.py: test_migration_cli_rejects_invalid_arguments | Invalid count and wrong verb independently fail before migration. |
| 1417 | same: test_migration_cli_optional_path | Dispatch both default and explicit database paths. |
| 1493 | same: test_sql_hint_validates_table_and_index_independently | Wrong table with allowed hint and right table with invalid hint both fail. |
| 1568 | same: test_traversal_fetch_budget_subtracts_already_visited_edges | Existing edges reduce adjacency fetch budget; retain one sentinel row. |

Additional ordinary regression cases cover Go positional/unbound literals, qualified Elixir calls, Kotlin callable names, generic Rust callees, nested Rust macro shadowing, Markdown location fallbacks, and process session setup. These do not imply that every corresponding mutant is distinguishable.

## Equivalent or unreachable on valid grammar inputs

These are not claimed as newly killed. No fabricated Node objects or impossible syntax were added to force a score.

| Indices | Review classification |
|---|---|
| 1158, 1160 | Go `_binding` parent guard changes do not change supported literal bindings. Assignment/spec values use expression_list. Other real parents (e.g. argument_list) still fail the subsequent owner/value-field checks. New positional/unbound literal cases preserve this contract, but do not distinguish these mutants. |
| 1164 | `_assigned_name` index==len(names) requires more explicit RHS literals than LHS bindings. That is invalid Go arity. No artificial out-of-range helper call was added. Valid positional literals are covered. |
| 1233 | Every valid Rust macro invocation supplies a token_tree. Changing its presence conjunction to disjunction only matters when tokens are absent, not for a valid invocation. |
| 1235 | A valid Rust token_tree contains both delimiters, so its text length is at least two. The removed length==2 case is an empty payload; both synthetic wrappers contain no call sites. No observable call extraction gap on valid input. |
| 1250 | Rust generic_function exposes its function field as its first named child. The fallback returns that same child; `or` to `and` does not change a valid generic target. |
| 1293 | A valid Elixir call has a target. Nonidentifier target spellings (e.g. Kernel.if) cannot equal the exact `if`/`unless` text allowlist, so the later text check still rejects them. Qualified-call regression added without claiming a new kill. |
| 1302 | Only Kotlin callable nodes use direct simple_identifier name children here. On other languages, the additional name scan finds nothing and assigned-name/Elixir naming still applies. Kotlin named callable regression added without claiming a new kill. |
| 1386 | For valid generic_function/template_function wrappers, the recognized callee field is the first named child. The current wrapper fallback returns the same node. Generic callee regression added; no synthetic missing-field wrapper. |
| 1396 | `_callback_parent` is called on an actual callback value (after transparent parentheses). In supported variable/assignment/pair parents, it is the value/right child. Name/key children cannot be direct callback carriers. No fabricated wrong-child callback was added. |

## All eight timeouts

Timeout is an operational outcome, not a survivor or an assertion-confirmed kill. No timeout was rerun.

| Index | Evidence and classification |
|---|---|
| 699 | artifact.py:124 `!=` to `==`: `_parent_paths` enters at `.` and repeatedly assigns `.` to its parent. Definite mutation-induced nontermination. Log already shows six failed artifact cases before the stall. |
| 706 | artifact.py:301 `==` to `!=`: root module gets itself as parent, allowing ancestor/projection walks to loop. Mutation-induced graph-parent nontermination; log stalls at artifact tests. |
| 748 | cycles.py:49 `and` to `or`: a repeated in-graph parent no longer terminates owner traversal. Mutation-induced cycle nontermination; log shows ten passes and a failure before stopping. |
| 888 | receiver_resolution.py:276 equality inversion broadens alias propagation to unequal paths. Log already shows four receiver failures, then stalls in the receiver snapshot suite. Behavioral detection exists; alias expansion/snapshot completion exceeded the budget. No claim of an independently measured infinite loop. Out of production-fix ownership. |
| 979 | report_markdown.py:32 path fallback `or` to `and`: truthy paths become Unknown file and missing/empty paths can become nonstrings. Log already shows six report-unit failures and five snapshot failures. Budget exhaustion during failing report/snapshot processing; the changed expression itself has no loop. New location fallback tests cover the local contract directly. |
| 1039 | resolver.py:306 `and` to `or`: repeated truthy owners no longer stop ancestor traversal (and None may enter the loop). Mutation-induced ancestor-cycle nontermination. Log stalls after the first resolver test. Out of production-fix ownership. |
| 1114 | runner.py:68 equality inversion disables POSIX process-session isolation. Group cleanup can miss the child and then wait unboundedly after command timeout. Infrastructure/liveness mutation, not an assertion kill. New `test_command_duration_and_process_group_contract` mocks Popen/_stop and asserts start_new_session exactly, without spawning or waiting on a mutated process. |
| 1508 | persistence/repositories.py:116 equality inversion makes name resolution return unrelated rows. Repository unique-name selection can keep finding a row for every generated name. Mutation-induced uniqueness-loop nontermination; navigation log stops after 28 passing tests. No production change needed. |

## Invalid / harness-only kills

Scanned all target logs for collection errors, INTERNALERROR, and setup errors. The following six records have no ordinary failed test assertion and require separate reporting, not behavioral-kill credit. Other setup errors accompanied ordinary failed tests and are not additional setup-only records. No additional tail collection/internal/setup-only kills were found beyond 1253 and 1255.

| Indices | Classification |
|---|---|
| 1, 2 | commands.py string concatenation `+` to `-` raises TypeError at import. Three collection errors each. Invalid operator-domain/import-only kills. |
| 4 | commands.py `__name__ == '__main__'` inversion invokes Click while pytest imports it. Click consumes pytest filenames and exits 2; pytest reports INTERNALERROR/SystemExit with no tests. Harness-only/import-entrypoint kill. |
| 498 | metrics/adapters/rust.py Path `/` to `*` raises TypeError in fixture setup. 482 passes, 11 errors, no assertion failures. Invalid operator-domain/setup-only kill, outside ownership. |
| 1253 | parsing/adapters/typescript.py query string `+` to `-` raises TypeError at import. Four collection errors. Tail invalid operator-domain/import-only kill. |
| 1255 | parsing/discovery.py frozenset difference `-` to `+` raises TypeError at import. Two collection errors. Tail invalid operator-domain/import-only kill. |

Raw campaign killed count is 1373. Excluding these six operationally invalid/harness-only records leaves 1367 other killed records; this is not a claim that every remaining operator-domain mutation was audited for semantic validity. All 1584 records remain preserved.

## Validation

85 new test cases pass (`other-final-checks.log`); the full importer suite also passes after the final parameter-count-only cleanup: 2213 passed in 44.75s (`other-pytest-final.log`). Ruff passes (`other-ruff-final.log`). Pyright has no errors in these five new files; its three remaining errors are in concurrent adapter regression files (`other-pyright-final.log`).

Dashboard production build, tests, typecheck, and lint --fix pass (`other-dashboard-*.log`). No frontend source was changed, so no browser check was needed.

Entire-project `matis check --f .` was run twice. The final findings are recorded in `other-matis-check-final.log`; none refer to these five new files. Existing/concurrent metrics adapter/receiver complexity and other workers' test parameter counts remain outside this work's ownership.

No matis.json edits, no existing-test edits, no production fixes, no mutator execution, and no commit. Mutation kill effectiveness is based on source/contract review only; it was not remeasured.
