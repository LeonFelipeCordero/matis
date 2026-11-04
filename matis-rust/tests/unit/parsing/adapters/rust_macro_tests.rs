//! Macro traversal semantics and deterministic shadow-scan work counts.
use super::*;

fn calls(root: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut calls = vec![];
    visit_calls(root, source, &mut |site, source| {
        if site.node.kind() == "call_expression" {
            calls.push(text(field(site.node, &["function"]), source));
        }
    });
    calls
}

fn source_calls(source: &str) -> Vec<String> {
    let tree = syntax::parse("rust", source.as_bytes()).unwrap();
    assert!(!tree.root_node().has_error(), "{source}");
    calls(tree.root_node(), source.as_bytes())
}

#[test]
fn repeated_macros_scan_the_original_tree_once() {
    let source = format!("fn main() {{ {} }}", "println!(\"{}\", work());".repeat(1000));
    let tree = syntax::parse("rust", source.as_bytes()).unwrap();
    let root = tree.root_node();
    let mut cache = ShadowCache::default();
    let mut count = 0;
    visit(
        Traversal {
            root,
            source: source.as_bytes(),
            origin: (root, source.as_bytes()),
            offset: 0,
            opaque: false,
            parents: vec![],
        },
        &mut |site, _| count += usize::from(site.node.kind() == "call_expression"),
        &mut cache,
    );
    assert_eq!(count, 1000);
    assert_eq!(cache.scans, 1);
    assert_eq!(cache.nodes, walk(root).count());
    assert!(cache.roots.is_empty());
}

#[test]
fn repeated_shadowed_macros_scan_once_too() {
    let source = format!("use custom::*; fn main() {{ {} }}", "println!(hidden());".repeat(1000));
    let tree = syntax::parse("rust", source.as_bytes()).unwrap();
    let root = tree.root_node();
    let mut cache = ShadowCache::default();
    let mut count = 0;
    visit(
        Traversal {
            root,
            source: source.as_bytes(),
            origin: (root, source.as_bytes()),
            offset: 0,
            opaque: false,
            parents: vec![],
        },
        &mut |site, _| count += usize::from(site.node.kind() == "call_expression"),
        &mut cache,
    );
    assert_eq!(count, 0);
    assert_eq!(cache.scans, 1);
    assert!(cache.nodes <= walk(root).count());
}

#[test]
fn payload_cache_entries_expire_before_sibling_payloads() {
    let source = b"fn main() { println!(format!(one())); println!(format!(two())); }";
    let tree = syntax::parse("rust", source).unwrap();
    let root = tree.root_node();
    let traversal = Traversal { root, source, origin: (root, source), offset: 0, opaque: false, parents: vec![] };
    let mut cache = ShadowCache::default();
    let mut calls = vec![];
    for node in walk(root).filter(|node| node.kind() == "macro_invocation") {
        macro_sites(
            (node, &traversal),
            &mut |site, source| {
                if site.node.kind() == "call_expression" {
                    calls.push(text(field(site.node, &["function"]), source));
                }
            },
            &mut cache,
        );
        assert_eq!(cache.roots.len(), 1);
        assert!(cache.roots.contains_key(&root.id()));
    }
    assert_eq!(calls, ["one", "two"]);
}

#[test]
fn unqualified_macros_keep_conservative_shadowing() {
    for declaration in [
        "macro_rules! println { ($($x:tt)*) => {} }",
        "use custom::println;",
        "use custom::*;",
        "mod unrelated { macro_rules! println { () => {} } }",
    ] {
        let source = format!("{declaration} fn main() {{ println!(hidden()); println!(also_hidden()); visible(); }}");
        assert_eq!(source_calls(&source), ["visible"], "{declaration}");
    }
    assert_eq!(source_calls("use std::println; fn main() { println!(visible()); }"), ["visible"]);
}

#[test]
fn macro_paths_have_independent_cache_entries() {
    let source = r#"
        macro_rules! println { () => {} }
        fn main() {
            println!(hidden());
            std::println!(visible_std());
            core::println!(visible_core());
            custom::println!(hidden_custom());
            unknown!(hidden_unknown());
        }
    "#;
    assert_eq!(source_calls(source), ["visible_std", "visible_core"]);
    for declaration in ["mod std {}", "use custom as std;", "extern crate custom as std;"] {
        let source = format!("{declaration} fn main() {{ std::println!(hidden()); core::println!(visible()); }}");
        assert_eq!(source_calls(&source), ["visible"], "{declaration}");
    }
    assert_eq!(
        source_calls("use std::fmt; extern crate core; fn main() { std::println!(one()); core::println!(two()); }"),
        ["one", "two"]
    );
}

#[test]
fn nested_macros_check_current_payload_parents_and_origin() {
    assert_eq!(source_calls("fn main() { println!(format!(work())); }"), ["work"]);
    assert!(source_calls("macro_rules! format { () => {} } fn main() { println!(format!(hidden())); }").is_empty());
    assert!(source_calls("fn main() { println!({ macro_rules! format { () => {} } format!(hidden()) }); }").is_empty());
    assert!(
        source_calls("fn main() { println!({ macro_rules! vec { () => {} } format!(vec![hidden()]) }); }").is_empty()
    );
    assert_eq!(
        source_calls(
            r#"
        fn main() {
            println!({ macro_rules! format { () => {} } format!(hidden()) });
            println!(format!(visible()));
        }
    "#
        ),
        ["visible"]
    );
}

#[test]
fn subtree_traversal_keeps_the_original_origin_policy() {
    let source = b"macro_rules! format { () => {} } fn main() { println!(format!(visible())); }";
    let tree = syntax::parse("rust", source).unwrap();
    let function = walk(tree.root_node()).find(|node| node.kind() == "function_item").unwrap();
    assert!(calls(tree.root_node(), source).is_empty());
    // Initial checks use the full tree; nested checks use the supplied origin subtree.
    assert_eq!(calls(function, source), ["visible"]);
}

#[test]
fn separate_and_reentrant_visits_do_not_share_cache_state() {
    let source = b"fn main() { println!(visible()); }";
    let tree = syntax::parse("rust", source).unwrap();
    let mut nested = false;
    visit_calls(tree.root_node(), source, &mut |site, _| {
        if site.node.kind() == "macro_invocation" {
            assert!(source_calls("use custom::*; fn main() { println!(hidden()); }").is_empty());
            nested = true;
        }
    });
    assert!(nested);
    assert_eq!(calls(tree.root_node(), source), ["visible"]);
}
