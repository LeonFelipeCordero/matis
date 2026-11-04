//! Finite equality reachability for prefix aliases, including expanding cycles.
use std::collections::{BTreeMap, BTreeSet};

/// Paths are ground terms: `a.b.c` applies `.c` to the term `a.b`.
/// Alias equations permit replacement of an equal prefix, not cancellation of a suffix.
/// Congruence closure needs only observed terms and their prefixes; it creates no paths.
pub(super) fn reaches(aliases: &[(String, String)], source: &str, targets: &BTreeSet<String>) -> bool {
    if targets.is_empty() {
        return false;
    }
    let mut graph = AliasGraph::default();
    graph.add_node();
    let source = graph.path(source);
    let targets: Vec<_> = targets.iter().map(|path| graph.path(path)).collect();
    let equations: Vec<_> = aliases.iter().map(|(left, right)| (graph.path(left), graph.path(right))).collect();
    for (left, right) in equations {
        graph.merge(left, right);
    }
    targets.into_iter().any(|target| graph.find(source) == graph.find(target))
}

#[derive(Default)]
struct AliasGraph {
    parents: Vec<usize>,
    sizes: Vec<usize>,
    fields: Vec<BTreeMap<String, usize>>,
}

impl AliasGraph {
    fn add_node(&mut self) -> usize {
        let id = self.parents.len();
        self.parents.push(id);
        self.sizes.push(1);
        self.fields.push(BTreeMap::new());
        id
    }

    fn path(&mut self, path: &str) -> usize {
        let mut current = 0;
        for component in path.split('.') {
            current = match self.fields[current].get(component).copied() {
                Some(id) => id,
                None => {
                    let next = self.add_node();
                    self.fields[current].insert(component.to_owned(), next);
                    next
                }
            };
        }
        current
    }

    fn find(&mut self, mut node: usize) -> usize {
        while self.parents[node] != node {
            self.parents[node] = self.parents[self.parents[node]];
            node = self.parents[node];
        }
        node
    }

    fn merge(&mut self, left: usize, right: usize) {
        let mut pending = vec![(left, right)];
        while let Some((left, right)) = pending.pop() {
            let (mut left, mut right) = (self.find(left), self.find(right));
            if left == right {
                continue;
            }
            if self.sizes[left] < self.sizes[right] {
                std::mem::swap(&mut left, &mut right);
            }
            self.parents[right] = left;
            self.sizes[left] += self.sizes[right];
            self.merge_fields(left, right, &mut pending);
        }
    }

    fn merge_fields(&mut self, left: usize, right: usize, pending: &mut Vec<(usize, usize)>) {
        // Union by size bounds every field's moves; matching applications are queued once.
        for (field, target) in std::mem::take(&mut self.fields[right]) {
            if let Some(previous) = self.fields[left].get(&field) {
                pending.push((*previous, target));
            } else {
                self.fields[left].insert(field, target);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aliases(equations: &[(&str, &str)]) -> Vec<(String, String)> {
        equations.iter().map(|(left, right)| (left.to_string(), right.to_string())).collect()
    }

    fn reachable(equations: &[(&str, &str)], source: &str, target: &str) -> bool {
        reaches(&aliases(equations), source, &[target.to_owned()].into())
    }

    #[test]
    fn self_expansion_reaches_observed_terms_without_generating_paths() {
        let deep = format!("x{}", ".field".repeat(4096));
        assert!(reachable(&[("x", "x.field")], "x", &deep));
        assert!(reachable(&[("x", "x.field")], &deep, "x"));
        assert!(!reachable(&[("x", "x.field")], "x", "x.other"));
    }

    #[test]
    fn mutual_expansion_and_branching_cycles_terminate() {
        let equations = [("x", "y.a"), ("y", "x.b"), ("x", "x.c"), ("y", "y.d")];
        assert!(reachable(&equations, "x", "x.b.a.b.a"));
        assert!(reachable(&equations, "x", "y.a.c"));
        assert!(!reachable(&equations, "x", "unrelated.a"));
    }

    #[test]
    fn finite_chains_keep_reverse_prefix_substitution_and_boundaries() {
        let equations = [("alias", "root"), ("other", "alias.dep")];
        assert!(reachable(&equations, "root.dep.child", "other.child"));
        assert!(reachable(&equations, "other.child", "root.dep.child"));
        assert!(!reachable(&equations, "rooted.dep.child", "other.child"));
        assert!(!reachable(&equations, "root.dep.child", "other.children"));
    }

    #[test]
    fn transient_unobserved_paths_do_not_hide_reachable_writes() {
        // Derivation needs x.a.b -> y.b -> z.c.b, neither intermediate is a query.
        assert!(reachable(&[("y", "x.a"), ("y", "z.c")], "x.a.b", "z.c.b"));
        assert!(reachable(&[("x", "y.a"), ("x", "z.b")], "y.a.c", "z.b.c"));
    }

    #[test]
    fn equal_fields_do_not_imply_equal_receivers() {
        let equations = [("alias", "x.run"), ("alias", "y.run")];
        assert!(reachable(&equations, "x.run", "y.run"));
        assert!(!reachable(&equations, "x", "y"));
    }

    fn expansions(path: &str, equations: &[(String, String)]) -> Vec<String> {
        equations
            .iter()
            .flat_map(|(left, right)| [(left, right), (right, left)])
            .filter(|(from, _)| path == from.as_str() || path.starts_with(&format!("{from}.")))
            .map(|(from, to)| format!("{to}{}", &path[from.len()..]))
            .collect()
    }

    fn finite_expansion(source: &str, equations: &[(String, String)]) -> BTreeSet<String> {
        let mut seen: BTreeSet<_> = [source.to_owned()].into();
        let mut pending = vec![source.to_owned()];
        while let Some(path) = pending.pop() {
            pending.extend(expansions(&path, equations).into_iter().filter(|path| seen.insert(path.clone())));
        }
        seen
    }

    fn assert_finite_equivalence(equations: &[(&str, &str)], source: &str) {
        let equations = aliases(equations);
        let expanded = finite_expansion(source, &equations);
        let targets: BTreeSet<_> = expanded.iter().cloned().chain(["absent".into(), "rooted.child".into()]).collect();
        for target in targets {
            assert_eq!(
                reaches(&equations, source, &[target.clone()].into()),
                expanded.contains(&target),
                "{source} -> {target}: {equations:?}"
            );
        }
    }

    #[test]
    fn finite_graphs_match_explicit_prefix_expansion() {
        let cases: &[(&[(&str, &str)], &str)] = &[
            (&[], "a.b"),
            (&[("a", "b")], "a.c"),
            (&[("a", "b.c"), ("b", "d.e"), ("f", "d.e.c")], "a.g"),
            (&[("a", "b.c"), ("a", "b.d")], "a.e"),
            (&[("a", "b"), ("b", "c"), ("c", "a")], "a.c"),
            (&[("a", "b.c"), ("d", "b.c.e"), ("e", "d.f")], "a.e.f"),
        ];
        for (equations, source) in cases {
            assert_finite_equivalence(equations, source);
        }
    }

    #[test]
    fn literal_text_not_identifier_validation_defines_alias_components() {
        assert!(reachable(&[("alias", "factory(a.b)")], "alias.child", "factory(a.b).child"));
        assert!(reachable(&[("alias", "pkg::Type")], "alias.child", "pkg::Type.child"));
    }
}
