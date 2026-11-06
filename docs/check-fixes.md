# Fix check failures

`matis check` links each failure to a section below. Use these suggestions as starting points, not automatic fixes. Keep behavior and tests unchanged when you refactor. If a metric is unavailable, resolve analysis problems before changing the code.

## maxParameters

Group related inputs into a small parameter object, record, or struct. Pass that object instead of several separate values. Do not group unrelated inputs only to pass the check. See [Introduce Parameter Object](https://refactoring.com/catalog/introduceParameterObject.html).

## maxFileLines

Split the file by responsibility. Move related functions or types into focused files and update their callers and imports. For a large class, extract a cohesive responsibility into another class. See [Extract Class](https://refactoring.com/catalog/extractClass.html). This rule measures files, not class size.

## maxFunctionLines

Extract a coherent operation into a named function. Keep each function focused on one task. See [Extract Function](https://refactoring.com/catalog/extractFunction.html).

## maxWeightedComplexity

Separate independent decisions into focused functions. Replace repeated conditional logic with a clear data lookup when appropriate. See [Decompose Conditional](https://refactoring.com/catalog/decomposeConditional.html).

## maxDecisionDepth

Use early returns for invalid inputs and special cases. Extract deeply nested decisions into focused functions. See [Replace Nested Conditional with Guard Clauses](https://refactoring.com/catalog/replaceNestedConditionalWithGuardClauses.html).

## maxNestedLoops

Use an index, map, or set instead of repeatedly scanning the same collection. Extract independent iterations when this makes the algorithm clearer. Check runtime and memory use; moving a loop alone does not make it faster.

## maxCircularImportGroups

Remove a dependency from the reported cycle. Move shared definitions into a lower-level module, or pass a dependency through a function parameter. Keep dependencies flowing in one direction.

## maxChangeImpact

Review the files that depend on this file or module. Separate stable interfaces from frequently changing implementation. Remove unnecessary dependencies rather than moving them to hide the measurement.

## maxInstability

Review outgoing dependencies. Separate unrelated responsibilities and depend on a small stable interface where useful. High instability can be appropriate for application entry points; choose a limit that fits the module's role.

## analysis

Read the diagnostic and fix the syntax or unsupported construct at the reported path. Run the check again. Missing measurements do not mean that the code passed.

## Use your own guide

Add `helpLinks` to the project's `matis.json`. Keys are rule names from the headings above, including `analysis`. Values must be HTTP or HTTPS links without whitespace or angle brackets. Omitted rules keep the default guide. Links appear as `helpUrl` in JSON and as `How to fix` in Markdown. Matis prints links; it does not fetch them.

```json
{
  "helpLinks": {
    "maxParameters": "https://example.com/team/parameter-objects",
    "maxFileLines": "https://example.com/team/module-structure"
  }
}
```
