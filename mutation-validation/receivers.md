# Receiver/resolver/shared-syntax mutation review

The indices below are exact, zero-based indices into `results.jsonl` and `manifests.json.mutations`. These indices agree in the restored campaign artifacts. Lines refer to the original manifest source, not the edited file. Only `survived` results in the five assigned files are listed. No mutation was executed during this review. A new regression is not a verified mutation kill.

## Confirmed production bugs

`ReceiverTypes._field_bindings` used byte containment, not direct class ownership. An outer and nested class with the same field made `outer.client.run()` unresolved. Real-parser regressions reproduced the failure in Python, Java, and Kotlin. The fix requires the binding's type-scope span to equal the selected type's span. Python also required `_reassigned` to exclude another type's field declaration; otherwise that declaration still looked like a write to the outer field. Ordinary nested Java method writes remain conservative and still invalidate the outer field.

Nested Python function-local `self` assignment also invalidated an outer `self.run()`. There are two independent scans. This work fixes `ReceiverTypes._path_written` by excluding nested function-local identifier assignments from write targets, but not from alias discovery. It retains nonlocal/global assignments and member writes, including writes through aliases created inside nested functions. The other agent's concurrent `script_python_selection.py` change fixes its independent lexical-owner check. This work did not edit that file. The full real-parser regressions depend on both fixes.

Only `matis/src/matis/metrics/receiver_resolution.py` was changed by this work. New test files are `matis/tests/metrics/test_receiver_ownership_regressions.py`, `test_receiver_mutation_regressions.py`, and `test_shared_syntax_mutation_regressions.py`. No existing test or configuration file was edited.

## Exact survivor classification

“Regression” means a new assertion covers the missing contract. “Boundary contract” means a synthetic test covers exact half-open bounds; the boundary declaration/write is not generally reachable in valid parser output. These are not claims that the original mutant was executed or killed.

| Index | File | Original line | Mutation | Classification and regression |
|---:|---|---:|---|---|
| 26 | language_syntax.py | 27 | and → or | Regression: ordinary Elixir heads must not be unwrapped as guard expressions. |
| 27 | language_syntax.py | 27 | == → != | Regression: binary non-`when` heads remain intact; guarded heads unwrap. The parser accepts binary heads even where semantic validation may reject a definition. |
| 784 | receiver_resolution.py | 58 | or → and | Regression: empty and dotted receiver spellings cannot enter lexical candidate lookup. |
| 785 | receiver_resolution.py | 58 | or → and | Regression: dotted and `::` receiver spellings cannot enter lexical candidate lookup. |
| 794 | receiver_resolution.py | 83 | and → or | Regression: both a dotted path and an implicit receiver root are required. |
| 797 | receiver_resolution.py | 92 | and → or | Regression: explicit member access cannot see nonmember bindings or out-of-scope members. |
| 798 | receiver_resolution.py | 92 | <= → < | Boundary contract: explicit member scope start is visible. |
| 800 | receiver_resolution.py | 92 | < → <= | Regression: explicit member scope end is not visible. |
| 832 | receiver_resolution.py | 189 | <= → < | Boundary contract: field index includes owner start; direct owner scope is also required. |
| 834 | receiver_resolution.py | 189 | < → <= | Boundary contract: field index excludes owner end. |
| 835 | receiver_resolution.py | 192 | and → or | Regression: static class ancestor and nonstatic method ancestor do not establish static callable context. |
| 838 | receiver_resolution.py | 196 | <= → < | Boundary contract: shadow scope start is visible. |
| 840 | receiver_resolution.py | 196 | < → <= | Regression: shadow scope end is not visible. |
| 842 | receiver_resolution.py | 197 | - → + | Regression: shadowing compares widths, not endpoint sums; wide late scope cannot shadow narrower binding scope. |
| 844 | receiver_resolution.py | 197 | - → + | Regression: same width comparison, with nonzero inner scope start. |
| 848 | receiver_resolution.py | 198 | < → <= | Boundary contract: declaration ending at receiver declaration start does not contain it. |
| 857 | receiver_resolution.py | 209 | <= → < | Boundary contract: reassignment scan includes binding scope start. |
| 860 | receiver_resolution.py | 213 | or → and | Regression: inferred, nonmember, and untrusted declarations cannot use constructor initialization exemption. |
| 861 | receiver_resolution.py | 213 | or → and | Regression: each of those exemption prerequisites is independent. |
| 870 | receiver_resolution.py | 227 | <= → < | Boundary contract: constructor parameter selection includes constructor start. |
| 872 | receiver_resolution.py | 227 | < → <= | Boundary contract: constructor parameter selection excludes constructor end. |
| 875 | receiver_resolution.py | 236 | <= → < | Boundary contract: path write scan includes function start. |
| 877 | receiver_resolution.py | 236 | < → <= | Regression: path write scan excludes function end. |
| 886 | receiver_resolution.py | 270 | or → and | Regression: missing alias target and member alias target are rejected even with a nonempty right side. |
| 894 | receiver_resolution.py | 301 | == → != | Regression: only expression lists unwrap; a plain identifier remains the same node. |
| 896 | receiver_resolution.py | 301 | == → != | Regression: only one-element expression lists unwrap; zero/two-element lists remain unchanged. |
| 897 | receiver_syntax.py | 30 | or → and | Regression: unnamed generic node falls back to its direct type identifier. |
| 916 | receiver_syntax.py | 109 | and → or | Regression: real Java and TypeScript instance fields are not static; explicit static fields are static. |
| 919 | receiver_syntax.py | 135 | and → or | Regression: injected parameter write scan excludes unrelated names and out-of-method writes. |
| 920 | receiver_syntax.py | 135 | <= → < | Boundary contract: injected parameter write scan includes method start. |
| 922 | receiver_syntax.py | 135 | < → <= | Boundary contract: injected parameter write scan excludes method end. |
| 923 | receiver_syntax.py | 135 | and → or | Regression: an in-method write to another name does not invalidate the injected parameter. |
| 928 | receiver_syntax.py | 148 | or → and | Regression: parented non-variable Java nodes are not receiver declarations. |
| 933 | receiver_syntax.py | 171 | and → or | Regression: plain Kotlin constructor parameter inside a real class is not a property. |
| 936 | receiver_syntax.py | 177 | == → != | Regression: Kotlin inferred class and local property types come from call initializers. |
| 937 | receiver_syntax.py | 178 | and → or | Regression: Kotlin local property is not a member merely because its property declaration has a parent. |
| 946 | receiver_syntax.py | 212 | and → or | Regression: `other.service` writes inside a real TypeScript class do not establish `this.service`. |
| 948 | receiver_syntax.py | 212 | and → or | Regression: same real-class negative prevents a class-scope false positive. |
| 995 | resolution_syntax.py | 51 | and → or | Regression: ordinary named declarator uses its binding name, not Go multi-name parameter handling. |
| 1008 | resolution_syntax.py | 185 | and → or | Regression: allowed symbol must satisfy both declaration containment bounds. |
| 1028 | resolver.py | 243 | or → and | Regression: adapters without family retain the original language; adapters with family use that family. |

Other new checks cover implicit-owner fallback to a typed lexical `self`, nested-field-only negative lookup, nested-local assignment versus nonlocal/global/member writes, and preservation of nested alias-write detection. No additional production change was justified in receiver_syntax.py, resolution_syntax.py, resolver.py, or language_syntax.py.

## Validation

Final focused validation: **360 passed in 1.67s**, recorded in `/tmp/receiver-final-tests.log`. The selection includes the three new regression files, existing receiver_resolution/receiver_bindings/receiver_syntax/resolution_syntax/resolver/resolution_members tests, `tests/test_coffee_go_rust_receiver_snapshots.py`, and the other agent's new `tests/metrics/test_adapter_python_mutation_regressions.py`. Focused Ruff checks passed for the changed production file and the three new test files (`/tmp/receiver-lint.log`).

A script checked this table against the original results: **41 survivors, 41 classified, no missing or extra indices**. No mutation was executed. No existing test/config file was edited by this work, and no commit was made. Broad repository checks, type checks, dashboard checks, and mutation reruns are intentionally excluded by the task scope. Concurrent agents own other worktree changes; do not attribute those files to this work.
