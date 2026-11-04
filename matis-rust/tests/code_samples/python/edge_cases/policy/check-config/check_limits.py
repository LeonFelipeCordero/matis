"""Enumerate public coffee project limits without exposing private dependencies."""

from dataclasses import dataclass

from matis.metrics.config import CheckConfig

RULES = (
    ("maxFileLines", "lineCount", {"file"}),
    ("maxFunctionLines", "lineCount", {"function", "method"}),
    ("maxParameters", "parameterCount", {"function", "method"}),
    ("maxWeightedComplexity", "weightedComplexity", {"function", "method"}),
    ("maxDecisionDepth", "maxDecisionDepth", {"function", "method"}),
    ("maxNestedLoops", "nestedLoops", {"function", "method"}),
)
DEPENDENCY_RULES = (
    ("maxChangeImpact", "transitiveChangeImpact"),
    ("maxInstability", "instability"),
)


@dataclass(frozen=True)
class _MetricLimit:
    rule: str
    metric: str
    maximum: int | float


def _enabled_limits(node, config: CheckConfig):
    rules = [(rule, metric) for rule, metric, kinds in RULES if node["kind"] in kinds]
    if node["level"] in {"module", "file"}:
        rules.extend(DEPENDENCY_RULES)
    for rule, metric in rules:
        maximum = getattr(config, rule)
        if maximum is not None:
            yield _MetricLimit(rule, metric, maximum)


def coffee_limits(nodes, config: CheckConfig):
    return [(node["subject"], list(_enabled_limits(node, config))) for node in nodes]
