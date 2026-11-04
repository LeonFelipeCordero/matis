use super::adapters::base::{DEFAULT_METRICS, MetricPolicy};
use crate::{domain::models::Complexity, syntax::named_children};
use tree_sitter::Node;

/// Calculate entry, decision, deep-decision and nested-loop costs without recursion.
/// Use `policy` to classify nodes in `root` and `source` for source-dependent predicates.
/// Exclude nested callable/type bodies and keep else-if alternatives at the same depth.
/// Return base complexity, penalties below depth two, nested loops, their sum, and maximum depth.
pub fn calculate_complexity(root: Node<'_>, policy: &MetricPolicy, source: &[u8]) -> Complexity {
    let root = policy.body(root);
    let (mut base, mut nested, mut loops, mut maximum) = (1, 0, 0, 0);
    let mut stack = vec![(root, 0usize, 0usize)];
    while let Some((node, mut depth, loop_depth)) = stack.pop() {
        if node != root && policy.scope(node, source) {
            continue;
        }
        if policy.alternative(node) {
            depth = depth.saturating_sub(1);
        }
        let decision = policy.decision(node, source);
        depth += usize::from(decision);
        base += usize::from(decision) + usize::from(policy.logical(node));
        nested += usize::from(decision && depth > 2);
        loops += usize::from(policy.is_loop(node) && loop_depth > 0);
        maximum = maximum.max(depth);
        let child_loop_depth = loop_depth + usize::from(policy.is_loop(node));
        stack.extend(named_children(node).into_iter().rev().map(|child| (child, depth, child_loop_depth)));
    }
    Complexity {
        base_cyclomatic_complexity: base,
        nested_cyclomatic_complexity: nested,
        loop_nested_complexity: loops,
        cyclomatic_complexity: base + nested + loops,
        max_decision_depth: maximum,
    }
}

/// Measure `root` against `source` using the merged default grammar policies.
/// Return the same complexity components as [`calculate_complexity`].
pub fn default_complexity(root: Node<'_>, source: &[u8]) -> Complexity {
    calculate_complexity(root, &DEFAULT_METRICS, source)
}
