# Full mutation validation on PR #47

Historical report for the removed Python importer. Paths, commands, and timings
below describe that revision, not the current Rust importer. Use `AGENTS.md` for
current checks and mutation commands.

Base: `optimize-slow-tests`, commit `436d146f70910a48a6672ca45322c37ebbe8f217`.
This change targets that branch, not `autonomous-mutation-testing`.

## Completed scope

The existing full campaign was monitored to completion. It was not duplicated or
interrupted. No source edits were made while it ran. The runner restored sources
1,584 times, and the working tree was clean afterward.

```sh
uv run --project matis matis mutations run --folder . --target all \
  --output /tmp/matis-mutation-validation/results.jsonl
```

All 23 configured targets were selected. They contained 3,514 candidate sites and
produced 1,584 enabled replacements. Twenty targets had executable replacements;
`domain-models`, `domain-navigation`, and `domain-repositories` had none.
Execution took 15,588.07 seconds (4 hours, 19 minutes, 48 seconds).

| Campaign | Executions | Killed | Survived | Timeout | Error / unmapped |
| --- | ---: | ---: | ---: | ---: | ---: |
| Full baseline | 1,584 | 1,373 | 203 | 8 | 0 |
| First affected rerun | 366 | 323 | 34 | 9 | 0 |
| Follow-up, overlapping the rerun | 13 | 13 | 0 | 0 | 0 |
| Latest outcome for each affected mutation | 366 | 336 | 21 | 9 | 0 |

The follow-up is not 13 additional distinct mutations. Do not add these rows or
present the affected rerun as a second full campaign.

## Confirmed fixes

`script_python_selection.py` now uses only prior, visible builtins descriptor
imports. An import inside an unrelated function or after a decorated declaration
must not turn an ordinary decorator into a static/class descriptor.

Python local assignments inside a nested function no longer invalidate the outer
method's `self`. The selection and receiver-write checks both enforce this rule.
Nonlocal writes, global writes, member writes, and writes through aliases remain
conservative invalidation cases.

`receiver_resolution.py` now requires fields to belong directly to the selected
class. A nested class cannot make an outer field ambiguous or supply a missing
outer field. Actual nested-method writes to an outer field still invalidate it.

The regression proof copy used the new tests with both original production files
from the base commit: **14 failed, 61 passed**. Those failures cover all three
fixes, including field ownership in Python, Java, and Kotlin. The final suite has
**2,224 passing cases**, compared with the original 1,854: 370 additional cases.

`matis.json` now includes the new tests in affected target commands. In particular,
mutation-candidate tests must include their parsing consumers. Existing directory
selections already cover new runner, parsing, and storage tests.

## Rerun method and results

The fresh manifest inventory was generated after the fixes:

```sh
uv run --project matis matis mutations --folder .
```

The rerun selected every original survivor and timeout that still mapped to the
current source, plus **all** current mutations in both changed production files.
Unchanged source positions matched directly. Positions in changed files matched
unchanged byte blocks, the rule ID, and fresh verified manifests. Original indices
875 and 877 were retired because their original source expressions were replaced.
They are not kills. The selection contains 209 mapped original nonkills and 157
additional mutations in the changed files.

Four independent temporary copies ran 92, 92, 91, and 91 mutations through
`MutationRunner.run`. Each command used the same pytest paths and timeout as its
configured target, with the existing environment's absolute `python -m pytest`
in place of `uv run pytest`. `PYTHONPATH` selected each copy's `matis/src`.
A complete isolated-copy baseline passed all 2,213 then-current tests before
mutation execution. No two mutators shared source paths. All 366 restorations
completed. A fifth copy ran the 13 follow-up mutations and restored all 13.

The follow-up corrected a target omission and strengthened tests that had not
actually distinguished their intended mutation. It covered original indices
26, 27, 87, 110, 277, 842, 844, 916, 1282, 1318, 1402, and 1403, plus the new
Python-only write-filter guard. Every follow-up command reported failed tests.

Of the original **203 survivors**, **179 now report killed**, **21 survive**,
**one times out**, and **two are retired**. The original eight timeouts remain
timeouts. All 157 additional changed-file mutations report killed. All 336 latest
killed outcomes have test-failure summaries, rather than collection-only exits.

## Remaining limits

The 21 remaining survivors are original indices:

```text
100, 248, 376, 422, 424, 574, 601, 603, 684, 686, 1158,
1160, 1164, 1233, 1235, 1250, 1293, 1302, 1357, 1386, 1396
```

The review reports explain their supported-input equivalence or unreachable
syntax/span boundaries. Index 1357 changes unused intermediate carrier metadata;
it does not change the canonical structure. These are review classifications,
not formal proofs of equivalence for arbitrary malformed input.

The nine timeouts are original indices **265, 699, 706, 748, 888, 979, 1039, 1114,
and 1508**. Index 265 changed from surviving to timeout after its newly selected
consumer test reached the broken scope loop. Several timeout logs contain failed
tests, but they retain the runner's timeout outcome. Ancestor/alias/uniqueness
loops and process-session cleanup account for most stalls. Index 979 has report
failures but no established infinite loop in the changed expression itself.
Timeouts are not counted as assertion kills or silently excluded.

The full baseline's six invalid/harness-only kills are **1, 2, 4, 498, 1253,
and 1255**. They involve import/collection TypeErrors, Click import-time exit, or
fixture setup TypeErrors. Excluding them leaves 1,367 other baseline kills; this
is not a type-validity audit of every arithmetic mutant. A nonzero command exit
alone does not establish a failed assertion.

This work does not claim exhaustive mutation success, 100% mutation coverage, or
a full rerun after fixes. Execution covers configured Python production sources,
not dashboard or test mutants. No frontend behavior changed; no browser check was
needed. The earlier dashboard failures while CLI sources were mutated were
invalid baseline evidence, not production defects.

## Durable evidence

[`mutation-validation/`](mutation-validation/) contains:

- `baseline-0.csv` through `baseline-3.csv`: all 1,584 original outcomes, in order.
- `rerun.csv`: all 366 first-rerun outcomes; `followup.csv` supersedes 13 of them.
- `mapping.csv`: all 211 original survivors/timeouts and their fresh IDs or retirement.
- Adapter, receiver, other-module, and follow-up review reports with exact indices,
  test mappings, equivalent-case reasons, timeout analysis, and invalid kills.

CSV `index` is the zero-based original record index, or `new` for an additional
changed-file mutation. Manifest IDs identify distinct edits. CSV rows retain the
source line, operator, replacement, target, exit code, and command duration.
Historical review reports describe the state at their review round; this document
and the follow-up CSV define the final outcome. In particular, the original
index-1318 stab-pattern attribution was wrong; the final test checks exclusion of
Elixir module attributes instead.

Full command stdout/stderr and original JSONL remain local under
`/tmp/matis-mutation-validation/`; they are not committed. The CSV evidence is
durable, but those local full logs require separate retention if needed later.

## Final validation

All commands ran from the repository root after source restoration:

```sh
(cd matis && uv run pytest -n auto)  # 2,224 passed
(cd matis && uv run pyright)        # 0 errors
(cd matis && uv run ruff check .)    # passed
(cd dashboard && bun run build)     # passed
(cd dashboard && bun test)          # 200 passed
(cd dashboard && bun run typecheck) # passed
(cd dashboard && bun lint --fix)    # passed; no dashboard diff
uv run --project matis matis check --f . # entire project passed
```

No README change, new dependency, or mutation-runner outcome-policy change.
