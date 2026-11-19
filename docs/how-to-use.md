# How to use Matis to improve code quality

Use `matis report` to see what Matis measures. Use `matis check` to enforce the limits and rules your project agrees on.

## 1. Measure the whole project

[Install Matis](../README.md#install), then run these commands from your project root:

```sh
matis report --folder .
matis check --folder .
```

Neither command needs a database unless you use `--save`. Analyze the whole project, not just changed files: a change in one file can add a dependency or close an import cycle elsewhere.

Read the findings first. Each finding names the rule, identifies the affected code, and links to [fix guidance](check-fixes.md). Resolve `analysis` findings first: fix the reported syntax or analysis problem before trusting the measurements. An unavailable metric is not zero.

## 2. Choose limits and fix what they reveal

Put overrides in `matis.json` at the root of the folder you analyze. Omitted limits keep their defaults. Set a limit to `null` to disable it. A measured value equal to its limit passes.

| Check | Default maximum | What to do when it fails |
| --- | --- | --- |
| `maxFileLines` | 700 | Split a file by responsibility, not by line count alone. |
| `maxFunctionLines` | 150 | Extract a named operation that you can test on its own. |
| `maxParameters` | 3 | Group values that describe one concept or split the operation. Do not group unrelated arguments just to pass. |
| `maxWeightedComplexity` | 8 | Simplify branching or separate distinct cases. |
| `maxDecisionDepth` | 3 | Use early returns or extract the decision from the surrounding logic. |
| `maxNestedLoops` | 0 | Check whether a lookup, grouping step, or separate pass removes the need for the inner loop. |
| `maxCircularImportGroups` | 0 | Follow the example cycle. Remove one edge by moving a shared contract or passing in the dependency. Type-only imports do not establish runtime cycles. |
| `maxFanIn` | 30 | If many files depend on this file, keep its interface stable. Split it if unrelated callers depend on unrelated parts. |
| `maxFanOut` | 20 | If this file depends on many others, remove unused dependencies or separate unrelated responsibilities. |
| `maxChangeImpact` | Disabled | Counts direct and indirect dependents. Use a high count to identify code that needs a stable interface and focused regression tests. |
| `maxInstability` | Disabled | Measures outgoing dependencies as a share of incoming plus outgoing dependencies. Values range from 0 to 1. Review high values in code that should be stable; high values can be normal at entry points. |

Size and complexity limits apply to files or functions and methods as shown by the check name or finding. Fan limits apply only to files. Change impact and instability checks apply to files and modules. Dependency metrics include tests and type-only dependencies.

For an existing project, fix a few important findings first. If a strict limit would block adoption, set a limit the team can meet now and lower it as you improve the code. Use `fanExclude` for an intentional fan-in or fan-out exception, not to hide code from analysis:

```json
{
  "maxFileLines": 800,
  "fanExclude": ["/src/shared/contracts.ts"]
}
```

`fanExclude` uses Git-style patterns and does not change dependency counts or other checks. Use `.matisignore` for files you intentionally want to remove from analysis, such as generated code.

## 3. Enforce architecture and coding rules

Add only the rules your project needs. These three lists are disabled when omitted or empty. This example assumes a project with TypeScript UI files and Java tests. Replace the paths and names with your project's own.

```json
{
  "forbiddenDependencies": [
    {
      "from": ["src/ui/**"],
      "to": ["src/storage/**"],
      "message": "Call the service instead of accessing storage directly."
    }
  ],
  "forbiddenCalls": [
    {
      "files": ["tests/**/*.ts"],
      "calls": ["setTimeout", "window.setTimeout"],
      "message": "Use fake timers or wait for the expected condition."
    }
  ],
  "forbiddenConstructors": [
    {
      "files": ["src/test/**/*.java"],
      "types": ["java.util.Random"],
      "message": "Use a deterministic test fixture."
    }
  ]
}
```

- **`forbiddenDependencies`** checks direct, resolved dependencies between files, including type-only imports. This example rejects `UI → storage` but permits `UI → service → storage`. Use a directory glob for a whole layer or an exact file path for a narrower rule.
- **`forbiddenCalls`** checks call names in selected files. List bare and qualified spellings separately when you want both. It does not infer receiver types or resolve dynamic calls.
- **`forbiddenConstructors`** checks explicit `new` expressions in Java, JavaScript, JSX, TypeScript, and TSX. In Java, an explicit import can identify the fully qualified type. An import alone is not a constructor violation.

The path globs are case-sensitive and relative to the project root. `**` matches zero or more path components. Ignored files stay ignored. Messages are optional. See the [check guide](check-fixes.md) for language details and how to use `helpLinks` to point findings to your team's standards.

## 4. Make it part of code review

Fix one issue at a time. For example, if a UI component accesses storage, move the operation behind the service interface, then run the UI and service tests. Rerun Matis to confirm the forbidden edge is gone and the change did not introduce other findings. If you need to trace callers before a refactor, use the [web UI](../README.md#web-ui) to explore incoming and outgoing dependencies.

Once the findings are under control, commit `matis.json` and run this command in CI after installing Matis:

```sh
matis check --folder . --format json
```

Exit status `0` means the checks passed. Findings produce exit status `1`. Keep the command's exit status when saving or processing JSON in CI so a failed check still fails the job.

Keep your normal tests and linters. Matis cannot enforce dependencies it does not resolve or prove that code behaves correctly. To see whether tests catch deliberate changes to behavior, use [targeted mutation testing](../matis-rust/MUTATION_RUNNER.md) after your regular tests pass.
