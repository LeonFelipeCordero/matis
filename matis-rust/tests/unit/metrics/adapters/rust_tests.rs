use super::*;
use crate::{
    domain::models::{ParsedRepository, SourceFile},
    metrics::adapters::adapter_for_language,
    parsing::parser::parse_file,
    syntax::walk,
};
use std::{collections::BTreeMap, path::PathBuf};
fn parser() -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_language_pack::get_language("rust").unwrap()).unwrap();
    parser
}

fn file(path: &str, source: &str) -> ParsedFile {
    parse_file(SourceFile {
        path: path.into(),
        absolute_path: PathBuf::from("/matis-module-tests").join(path),
        language: "rust".into(),
        source: source.as_bytes().to_vec(),
    })
    .unwrap()
}
fn modules(files: Vec<ParsedFile>) -> RustModules {
    let syntax = files.iter().map(|f| (f.source.path.clone(), Syntax::new(f, adapter_for_language("rust")))).collect();
    RustModules::new(&ParsedRepository { files }, &syntax)
}
#[test]
fn inline_context_selects_relative_modules_without_leaking_declarations() {
    let root = file("lib.rs", "mod outer { pub fn local() {} mod inner { pub fn nested() {} } } pub fn top() {}");
    let index = modules(vec![root.clone()]);
    assert!(index.rust_import(&root, "crate::outer", None).is_empty());
    assert_eq!(
        index.rust_symbols(&root, "crate::outer", None).iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["local", "inner"]
    );
    let position = String::from_utf8_lossy(&root.source.source).find("nested").unwrap();
    assert_eq!(
        index.rust_symbols(&root, "super", Some(position)).iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["local", "inner"]
    );
    assert!(index.rust_key(&root, "super", None).is_none());
    assert!(index.rust_key(&root, "crate::outer", None).unwrap().contains("outer"));
}
#[test]
fn path_attributes_support_external_modules_inline_directories_and_cycles() {
    let root = file(
        "lib.rs",
        "#[path = r#\"alternate/feature.rs\"#] mod feature; #[path = \"custom\"] mod inline { mod child; }",
    );
    let feature = file("alternate/feature.rs", "#[path = \"../lib.rs\"] mod cycle; pub fn run() {}");
    let child = file("custom/child.rs", "pub fn work() {}");
    let index = modules(vec![root.clone(), feature.clone(), child.clone()]);
    assert_eq!(index.rust_import(&root, "feature", None)[0].source.path, feature.source.path);
    assert_eq!(index.rust_files("crate::inline::child")[0].source.path, child.source.path);
    assert!(index.rust_files("crate::feature::cycle").is_empty());
}
#[test]
fn duplicate_mounts_and_duplicate_entry_roots_remain_ambiguous() {
    let root = file("lib.rs", "#[path = \"shared.rs\"] mod a; #[path = \"shared.rs\"] mod b;");
    let shared = file("shared.rs", "mod child;");
    let child = file("shared/child.rs", "pub fn work() {}");
    let index = modules(vec![root.clone(), shared.clone(), child]);
    assert!(index.rust_key(&shared, "child", None).is_none());
    assert!(index.rust_key(&root, "crate::a::child", None).is_some());
    let index = modules(vec![root.clone(), file("main.rs", "fn main() {}")]);
    assert!(index.rust_key(&root, "crate", None).is_none());
}
#[test]
fn infers_unreferenced_owners_when_standard_entry_files_are_absent() {
    let root = file("owner.rs", "mod child;");
    let child = file("child.rs", "mod leaf;");
    let leaf = file("child/leaf.rs", "fn work() {}");
    let index = modules(vec![root.clone(), child, leaf]);
    assert_eq!(index.rust_files("crate")[0].source.path, root.source.path);
    assert_eq!(index.rust_files("crate::child::leaf").len(), 1);
}
#[test]
fn path_attributes_ignore_cfg_attr_and_unrelated_items_and_decode_escapes() {
    let source =
        b"#[path = \"dir\\x2ffile.rs\"] // allowed\n#[cfg_attr(test, path = \"wrong.rs\")]\n#[inline]\nmod value;";
    let tree = parser().parse(source, None).unwrap();
    let node = walk(tree.root_node()).find(|n| n.kind() == "mod_item").unwrap();
    assert_eq!(path_attributes(node, source), ["dir/file.rs"]);
}
#[test]
fn named_path_escapes_use_unicode16_aliases_and_strict_names() {
    let source = br##"#[path = "\N{SNOWMAN}\N{BYTE ORDER MARK}.rs"] mod valid;
#[path = "\N{Black_Star}.rs"] mod loose;
#[path = "\N{KAWI SIGN REPHA}.rs"] mod newer;"##;
    let tree = parser().parse(source, None).unwrap();
    let modules: Vec<_> = walk(tree.root_node()).filter(|n| n.kind() == "mod_item").collect();
    assert_eq!(path_attributes(modules[0], source), ["☃\u{feff}.rs"]);
    assert!(path_attributes(modules[1], source).is_empty());
    assert_eq!(path_attributes(modules[2], source), ["\u{11f02}.rs"]);
}
#[test]
fn parameter_counts_skip_self_comments_attributes_and_error_headers() {
    let source = b"impl A { fn f(&self, /* c */ #[cfg(test)] x: i32, y: i32) {} fn g(self) {} }";
    let tree = parser().parse(source, None).unwrap();
    let functions: Vec<_> = walk(tree.root_node()).filter(|n| n.kind() == "function_item").collect();
    assert_eq!(parameter_count(functions[0], source), Some(2));
    assert_eq!(parameter_count(functions[1], source), Some(0));
    assert!(has_self_node(functions[0]));
}
#[test]
fn match_wildcards_do_not_count_as_decisions_but_guarded_wildcards_do() {
    let source = b"fn f(x: i32) { match x { _ if true => 1, _ => 2 } }";
    let tree = parser().parse(source, None).unwrap();
    let decisions: Vec<_> =
        walk(tree.root_node()).filter(|n| n.kind() == "match_arm").map(|n| match_decision(n, source)).collect();
    assert_eq!(decisions, [true, false]);
}
#[test]
fn inherent_impl_methods_are_found_without_import_impl_shadowing() {
    let root =
        file("lib.rs", "mod types; use crate::types::A; impl A { pub fn run(&self) {} } fn f(a: A) { a.run(); }");
    let types = file("types.rs", "pub struct A;");
    let resolver = CallResolver::new(&ParsedRepository { files: vec![root, types] });
    let parent = resolver.symbols.values().find(|s| s.kind == "struct" && s.name == "A").unwrap();
    let matches = instance_members(&resolver, parent, "run");
    assert_eq!(matches.len(), 1);
    let syntax = &resolver.syntax[&matches[0].file_path];
    assert_eq!(static_symbol(syntax, &matches[0]), Some(false));
}
#[test]
fn import_paths_and_receivers_use_python_unicode_words() {
    assert_eq!(import_path("\u{301}use αⅧ::x\u{301}tail"), "αⅧ::x");
    let tree = parser().parse(b"fn f() {}", None).unwrap();
    assert!(!receiver_rejected(Some(tree.root_node()), Some("x\u{301}")));
    assert!(receiver_rejected(Some(tree.root_node()), Some("αⅧ::x")));
}
#[test]
fn empty_module_index_has_no_scopes() {
    let index = RustModules::new(&ParsedRepository::default(), &BTreeMap::new());
    assert!(index.rust_files("crate").is_empty());
}

#[test]
fn five_hundred_inline_modules_share_files_scopes_and_borrow_queries() {
    use std::sync::Arc;
    let source = (0..500).map(|index| format!("mod m{index} {{ pub fn f() {{}} }}")).collect::<Vec<_>>().join("\n");
    let root = file("lib.rs", &source);
    let resolver = CallResolver::new(&ParsedRepository { files: vec![root.clone()] });
    let modules = resolver.modules.rust.as_ref().unwrap();
    let scopes = &modules.index.file_scopes["lib.rs"];
    assert_eq!(scopes.len(), 501);
    assert_eq!(scopes[0].identity, "crate");
    assert_eq!(scopes[1].identity, "crate::m499");
    assert_eq!(scopes[500].identity, "crate::m0");
    assert!(scopes.iter().all(|scope| Arc::ptr_eq(&scope.file, &scopes[0].file)));
    assert!(scopes.iter().all(|scope| Arc::ptr_eq(scope, &modules.index.scopes[&scope.identity][0])));
    let imported = resolver.imports.modules.rust.as_ref().unwrap();
    assert!(Arc::ptr_eq(&scopes[1], &imported.index.file_scopes["lib.rs"][1]));
    let count = Arc::strong_count(&scopes[1]);
    let nearest = modules.index.scopes_at(&root, Some(source.rfind("pub fn").unwrap()), &modules.syntax);
    assert_eq!(nearest.len(), 1);
    assert!(std::ptr::eq(nearest[0], scopes[1].as_ref()));
    assert_eq!(Arc::strong_count(&scopes[1]), count);
    assert_eq!(modules.rust_symbols(&root, "crate::m499", None).len(), 1);
    assert!(modules.rust_key(&root, "crate::m499", None).is_some());
}

#[test]
fn equal_span_scopes_preserve_insertion_order_and_duplicate_ambiguity() {
    use super::super::module_index::ModuleIndex;
    let root = file("lib.rs", "mod original { pub fn f() {} }");
    let modules = modules(vec![root.clone()]);
    let original = &modules.index.scopes["crate::original"][0];
    let mut first = original.as_ref().clone();
    first.identity = "first".into();
    let mut second = original.as_ref().clone();
    second.identity = "second".into();
    let mut index = ModuleIndex::new();
    index.add_scopes([first.clone(), second, first]);
    let position = String::from_utf8_lossy(&root.source.source).find("pub fn").unwrap();
    let scopes = index.scopes_at(&root, Some(position), &modules.syntax);
    assert_eq!(scopes.iter().map(|scope| scope.identity.as_str()).collect::<Vec<_>>(), ["first", "second", "first"]);
    assert!(index.unique_scope(&["first".into()]).is_none());
    assert!(index.unique_scope(&["second".into()]).is_some());
}
