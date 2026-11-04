# Adapter mutation review B

Scope: `metrics/adapters/{python,resolution_policy,rust,script_python_selection,script_resolution,typescript}.py` only. The completed campaign was read, not rerun. No mutations were applied during this work.

## Result

Reviewed all 63 survivors in this scope: 50 behavior/test gaps, 5 defensive guard gaps, and 8 equivalent mutations under the current parser/extractor contract. Classification is by code inspection and regression assertions, not by rerunning mutations. Do not describe these as 55 confirmed mutation kills.

Added 3 new files, 40 test functions, 124 parametrized cases:

- `matis/tests/metrics/test_adapter_python_mutation_regressions.py`: Python patterns, parameter counting, class visibility, overload imports, bound self, and both production regressions.
- `matis/tests/metrics/test_adapter_script_mutation_regressions.py`: default policy paths/static members, receiver scope boundaries, method isolation, receiver parameters, bounded alias convergence, and TypeScript single-target selection.
- `matis/tests/metrics/test_adapter_rust_mutation_regressions.py`: external/inline module directories, path attributes, root inference, import qualification, receiver owners, static guards, struct/call targets, receiver rejection, and import-key fallback.

Production edits are limited to `matis/src/matis/metrics/adapters/script_python_selection.py`. Descriptor aliases now require a prior visible import. Detached syntax nodes retain conservative fallback behavior. Self assignments now use their lexical binding scope, rather than byte containment alone. Nested local writes no longer reject the outer method. Nested `nonlocal self` writes and comprehension walrus writes still reject it.

Before the fixes, the first new Python test run had 7 failures and 45 passes. Those failures confirmed unrelated/later descriptor-import pollution and nested-function local self writes. The final new-file run has 124 passes. Existing tests, config, fixtures, snapshots, and README were not edited.

## Index convention

Every index below is **zero-based** in both `results.jsonl` and `manifests.json.mutations`; the IDs/order were checked. Lines are original, one-based manifest source lines, before these edits. The progress log uses one-based execution counters, so index 309 corresponds to counter 310. `G` means behavior/test gap, `D` means defensive guard gap, and `E` means equivalent under the current parser/extractor contract.

## Exact survivor ledger

| Index | Original file:line | Class | Reason / regression assertion |
|---:|---|:---:|---|
| 309 | python.py:55 | G | A dotted value pattern is not an irrefutable capture. |
| 311 | python.py:60 | G | `1 as name` remains refutable; `_ as name` is irrefutable. |
| 312 | python.py:64 | G | A literal-only union remains refutable. |
| 313 | python.py:64 | G | A union with a wildcard is irrefutable. |
| 314 | python.py:70 | G | Parenthesized capture is irrefutable. |
| 315 | python.py:70 | G | A one-element tuple capture is still refutable. |
| 319 | python.py:104 | G | Empty method parameters return zero without indexing an empty list. |
| 331 | python.py:138 | G | Typed self is excluded from method input count. |
| 337 | python.py:167 | G | Missing bodies and positions outside a nested body must not hide class scope. |
| 338 | python.py:167 | G | Nested-body start is included by the helper's half-open interval contract. |
| 340 | python.py:167 | G | Nested-body end is excluded by that contract. |
| 347 | python.py:196 | G | Package markers, not arbitrary ordinary files, determine source roots. |
| 376 | python.py:340 | E | For one candidate, overload_implementation applies exactly the same final typing_overload predicate as the fast path; zero candidates use the unchanged fast path. |
| 389 | python.py:436 | G | Class-body start is included; positions before and after the body are rejected. |
| 407 | python.py:466 | D | Missing/orphan syntax must retain the shared static fallback, not dereference a missing node/parent. |
| 408 | python.py:466 | D | Same missing-node/parent static fallback guard. |
| 422 | python.py:557 | E | A call cannot start at its enclosing binding scope's declaration start in a valid parsed Python tree. Only the unreachable equality changes. |
| 424 | python.py:557 | E | A call cannot start at the exclusive end of its enclosing binding scope. |
| 470 | resolution_policy.py:114 | G | Relative slash paths, dot-only package spelling, and non-relative slash packages use different lookup paths. |
| 473 | resolution_policy.py:196 | G | Only module parents get implicit static membership. |
| 501 | rust.py:139 | G | An ordinary nonstandard Rust file with no mod item must not become a competing crate root. |
| 504 | rust.py:152 | G | `parent.rs` and `parent/mod.rs` both mount children under the parent directory. |
| 507 | rust.py:160 | G | A source-file path attribute uses the file parent; an inline module uses its module directory. |
| 530 | rust.py:288 | G | An absent or namespace binding must not replace impl candidates. |
| 531 | rust.py:288 | G | A non-namespace imported type replaces impl-only candidates, but a namespace does not. |
| 532 | rust.py:288 | G | Both local and matched candidate lists participate; subtraction is invalid for these lists. |
| 533 | rust.py:297 | G | Missing either the function or impl ancestor rejects implicit self resolution. |
| 534 | rust.py:306 | G | Receiver-owner lookup must select ancestors of the requested kind. |
| 535 | rust.py:311 | G | Functions without self and missing syntax do not prove an instance receiver. |
| 536 | rust.py:316 | D | Missing syntax and non-function impl items must not enter function-specific static inference. |
| 538 | rust.py:316 | D | A non-function impl item is not a self-less function; this guards the adapter against non-callable inputs. |
| 539 | rust.py:316 | D | Same non-function/declaration-list shape guard. |
| 541 | rust.py:318 | G | A trait function must not enter the inherent-impl static policy. |
| 548 | rust.py:331 | G | Ordinary calls retain their receiver; struct literal targets have no call receiver. |
| 549 | rust.py:331 | G | Struct literals and ordinary calls use different target rules. |
| 550 | rust.py:364 | G | Only unresolved qualified paths use member fallback; simple paths and existing keys do not. |
| 561 | rust.py:441 | G | Receiver rejection requires an actual receiver and does not reject `this` or computed qualifiers. |
| 562 | rust.py:441 | G | `this` is exempt; self and crate paths are not. |
| 563 | rust.py:442 | G | Computed qualifiers are not path-like explicit receivers. |
| 564 | rust.py:442 | G | Nonempty qualifiers must reach the regex; empty/None qualifiers must not. |
| 571 | script_python_selection.py:24 | G | An import inside an unrelated function cannot provide an overload decorator. |
| 572 | script_python_selection.py:25 | G | An import after the function cannot provide its overload decorator. |
| 574 | script_python_selection.py:25 | E | An import statement and a separate function declaration cannot have the same start byte. |
| 576 | script_python_selection.py:29 | G | Importing an ordinary module does not prove typing.overload. |
| 594 | script_python_selection.py:82 | G | A descriptor spelling imported from vendor is not a builtins descriptor alias. |
| 597 | script_python_selection.py:97 | G | A class method with no parameters or another first parameter cannot bind self. |
| 598 | script_python_selection.py:98 | G | Empty parameters do not prove a first self parameter. |
| 600 | script_python_selection.py:103 | G | An unrelated local assignment does not invalidate self. |
| 601 | script_python_selection.py:103 | E | An assignment cannot begin at its enclosing function's declaration start. |
| 603 | script_python_selection.py:103 | E | An assignment cannot begin at the exclusive end of the function that contains it. |
| 604 | script_python_selection.py:104 | G | Only writes to self invalidate the receiver, not writes to unrelated names. |
| 608 | script_python_selection.py:113 | G | A top-level function with a self parameter is not an instance method. |
| 609 | script_python_selection.py:114 | G | A top-level function's module parent must be rejected without dereferencing its absent parent. |
| 613 | script_python_selection.py:124 | G | Class methods without a first self parameter, as well as missing functions, must be rejected. |
| 632 | script_resolution.py:99 | G | The class node at the parent start belongs in the cached receiver scope scan. |
| 634 | script_resolution.py:99 | G | A node at the parent's exclusive end must not enter that scan. |
| 636 | script_resolution.py:106 | G | Only Python methods contribute a named receiver; class-only scopes must not add None. |
| 639 | script_resolution.py:130 | G | Alias closure must terminate when the alias set equals the receiver set. The test stub asserts at most three calls, so it cannot hang. |
| 658 | script_resolution.py:166 | G | Orphan/foreign method owners do not establish a class receiver. |
| 659 | script_resolution.py:166 | G | Direct/decorated methods require the matching class parent. |
| 661 | script_resolution.py:173 | G | Missing/empty parameters return no first receiver. |
| 684 | script_resolution.py:244 | E | A signature and a different implementation cannot share a declaration start in a valid tree; only that equality changes. |
| 686 | typescript.py:26 | E | Current extraction produces callable nodes with a body, or property signatures explicitly rejected by the fast path. A singleton body succeeds through either path; a singleton signature/missing node fails through either path. An invented public_field_definition candidate would distinguish the mutation, but extraction captures its arrow-function value, not that field. No artificial field test was retained. |

## Coordination and unresolved limits

`receiver_resolution.py` has the same nested local self-write issue in shared receiver inference. It belongs to another agent. This work does not edit it. Adapter-level selection tests isolate this fix from that separate receiver stage; complete call-edge regression coverage needs the other owner's receiver fix. The shared checkout already showed that agent's file/tests changing while this work ran. Do not overwrite or include those files as this work's changes.

No test or production fix was added for alias shadowing/rebinding beyond the requested lexical visibility/order bug. Unknown decorators remain subject to the existing policy. Nonlocal writes are conservatively rejected even when a more deeply nested binding chain might prove they target another intermediate closure; this preserves rejection rather than introducing unsafe resolution.

No full repository suite, dashboard checks, matis check, or mutation campaign was run, as requested. The equivalent cases remain un-killed by design. Test assertions target the inspected gaps, but kill counts remain unverified without a later owned mutation run.

## Validation

- Initial Python regression run before production edits: 45 passed, 7 failed.
- Final new regression files: 124 passed.
- Focused related adapter/selection/member/receiver tests: 243 passed before the final four added guard cases.
- Final full adapter test-file group (`uv run pytest -q tests/metrics/test_adapter_*.py`): 683 passed in 0.67s, including the other agent's adapter files present at that time.
- Focused Ruff: passed.
- Focused Pyright: 0 errors, 0 warnings, 0 informations. The initial test-stub parameter-name error was corrected.
- `git diff --check`: passed.
