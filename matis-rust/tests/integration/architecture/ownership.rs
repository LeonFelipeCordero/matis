use super::{root, rust_tree, source_files};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
use tree_sitter::Node;

type Owner = (&'static str, &'static str, &'static str, &'static str);
const OWNERS: &[Owner] = &[
    ("repositories", "repositories", "RepositoryPort", "repositories"),
    ("repository_graphs", "revisions", "GraphRevisionPort", "graph_revisions"),
    ("graph_nodes", "nodes", "NodePort", "nodes"),
    ("graph_files", "files", "FilePort", "nodes"),
    ("graph_symbols", "symbols", "SymbolPort", "nodes"),
    ("graph_node_decorators", "decorators", "DecoratorPort", "nodes"),
    ("graph_edges", "relationships", "RelationshipPort", "relationships"),
    ("graph_node_metrics", "metrics", "NodeMetricsPort", "node_metrics"),
    ("graph_diagnostics", "diagnostics", "DiagnosticPort", "diagnostics"),
    ("graph_mutation_candidates", "candidates", "MutationCandidatePort", "mutation_candidates"),
];
fn production(source: &str) -> String {
    let tree = rust_tree(source);
    let mut source = source.as_bytes().to_vec();
    for node in matis::syntax::walk(tree.root_node()).filter(|n| n.kind() == "mod_item") {
        let Some(name) = node.child_by_field_name("name") else { continue };
        let name = name.utf8_text(&source).unwrap().to_owned();
        let test = name == "tests" || name.ends_with("_tests") || name.contains("integration_tests");
        if test && let Some(body) = node.child_by_field_name("body") {
            source[body.start_byte() + 1..body.end_byte() - 1].fill(b' ');
        }
    }
    String::from_utf8(source).unwrap()
}
fn identifiers(source: &str) -> BTreeSet<String> {
    let tree = rust_tree(source);
    matis::syntax::walk(tree.root_node())
        .filter(|n| matches!(n.kind(), "identifier" | "type_identifier" | "field_identifier"))
        .map(|n| source[n.byte_range()].to_owned())
        .collect()
}
fn strings(source: &str) -> Vec<String> {
    let tree = rust_tree(source);
    matis::syntax::walk(tree.root_node())
        .filter(|n| matches!(n.kind(), "string_literal" | "raw_string_literal"))
        .map(|n| source[n.byte_range()].to_owned())
        .collect()
}
fn path_names(node: Node<'_>, source: &str) -> Vec<String> {
    let mut names: Vec<_> = matis::syntax::walk(node)
        .filter(|n| matches!(n.kind(), "identifier" | "type_identifier" | "crate" | "super" | "self"))
        .map(|n| source[n.byte_range()].to_owned())
        .collect();
    if node.kind() == "use_wildcard" {
        names.push("*".into());
    }
    names
}
fn import_paths(node: Node<'_>, source: &str) -> Vec<Vec<String>> {
    match node.kind() {
        "use_declaration" => import_paths(node.child_by_field_name("argument").unwrap(), source),
        "use_as_clause" => import_paths(node.child_by_field_name("path").unwrap(), source),
        "scoped_use_list" => {
            let prefix = path_names(node.child_by_field_name("path").unwrap(), source);
            import_paths(node.child_by_field_name("list").unwrap(), source)
                .into_iter()
                .map(|path| prefix.iter().cloned().chain(path).collect())
                .collect()
        }
        "use_list" => matis::syntax::named_children(node).into_iter().flat_map(|n| import_paths(n, source)).collect(),
        _ => vec![path_names(node, source)],
    }
}
fn repository_path(path: &[String], module: &str) -> bool {
    let root = path.iter().find(|p| !["crate", "super", "self"].contains(&p.as_str()));
    if root.is_some_and(|p| ["domain", "services"].contains(&p.as_str())) {
        return false;
    }
    let namespace = root.is_some_and(|p| ["persistence", "*"].contains(&p.as_str()));
    path.iter().any(|p| p == module)
        || path.last().is_some_and(|p| p == "persistence")
        || (namespace && path.last().is_some_and(|p| p == "*"))
}
fn inside_import(node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(node) = parent {
        if node.kind() == "use_declaration" {
            return true;
        }
        parent = node.parent();
    }
    false
}
fn module_reference(source: &str, module: &str) -> bool {
    let tree = rust_tree(source);
    matis::syntax::walk(tree.root_node())
        .filter(|n| {
            n.kind() == "use_declaration"
                || (matches!(n.kind(), "scoped_identifier" | "scoped_type_identifier") && !inside_import(*n))
        })
        .flat_map(|node| import_paths(node, source))
        .any(|path| repository_path(&path, module))
}
fn table_reference(source: &str, table: &str) -> bool {
    let tree = rust_tree(source);
    matis::syntax::walk(tree.root_node())
        .filter(|n| matches!(n.kind(), "use_declaration" | "scoped_identifier" | "scoped_type_identifier"))
        .any(|n| {
            let names = path_names(n, source);
            names.iter().any(|n| n == table)
                && names.iter().any(|n| ["models", "Entity", "Column", "Model", "ActiveModel"].contains(&n.as_str()))
        })
}
fn queries_table(literal: &str, table: &str) -> bool {
    let words: BTreeSet<_> =
        literal.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').map(str::to_ascii_lowercase).collect();
    words.contains(table) && ["select", "insert", "update", "delete", "join", "from"].iter().any(|w| words.contains(*w))
}
fn assert_table_owner(file: &Path, source: &str, &(table, repository, _, _): &Owner) {
    let owner = format!("persistence/{repository}.rs");
    let access = table_reference(source, table) || strings(source).iter().any(|s| queries_table(s, table));
    assert!(!access || file == Path::new(&owner), "{} accesses {table}, owned by {owner}", file.display());
}
fn assert_repository_owner(file: &Path, source: &str, &(_, repository, _, _): &Owner) {
    let owner = format!("persistence/{repository}.rs");
    assert!(
        !module_reference(source, repository) || file == Path::new(&owner),
        "{} calls another table repository: {repository}",
        file.display()
    );
}
fn assert_source_ownership(file: &Path, source: &str) {
    OWNERS.iter().for_each(|owner| assert_table_owner(file, source, owner));
    if file.starts_with("persistence") && file != Path::new("persistence/mod.rs") {
        OWNERS.iter().for_each(|owner| assert_repository_owner(file, source, owner));
    }
}
#[test]
fn only_one_repository_can_reference_each_table() {
    let native = root();
    for file in source_files(&native) {
        let relative = file.strip_prefix(&native).unwrap();
        if relative.starts_with("persistence/migrations") || relative == Path::new("persistence/models.rs") {
            continue;
        }
        assert_source_ownership(relative, &production(&fs::read_to_string(&file).unwrap()));
    }
}
fn assert_port_owner(file: &Path, names: &BTreeSet<String>, &(_, repository, port, service): &Owner) {
    let implementation = format!("persistence/{repository}.rs");
    let owner = format!("services/{service}.rs");
    assert!(
        !names.contains(port) || file == Path::new(&implementation) || file == Path::new(&owner),
        "{} bypasses owner service {service} through {port}",
        file.display()
    );
}
fn assert_storage_types(file: &Path, names: &BTreeSet<String>) {
    for name in [
        "sea_orm",
        "sea_query",
        "sqlx",
        "ConnectionTrait",
        "EntityTrait",
        "Statement",
        "DatabaseConnection",
        "DatabaseTransaction",
        "execute_unprepared",
        "query_one",
        "query_all",
        "get_sqlite_connection_pool",
    ] {
        assert!(!names.contains(name), "{} leaks storage type {name}", file.display());
    }
}
#[test]
fn table_ports_belong_only_to_the_owner_service() {
    let native = root();
    for file in source_files(&native) {
        let relative = file.strip_prefix(&native).unwrap();
        if relative == Path::new("domain/storage.rs") {
            continue;
        }
        let names = identifiers(&production(&fs::read_to_string(&file).unwrap()));
        OWNERS.iter().for_each(|owner| assert_port_owner(relative, &names, owner));
        if !relative.starts_with("persistence") {
            assert_storage_types(relative, &names);
        }
    }
}
#[derive(Default)]
struct Visits {
    active: BTreeSet<String>,
    complete: BTreeSet<String>,
}
fn visit(name: &str, edges: &BTreeMap<String, BTreeSet<String>>, visits: &mut Visits) {
    if visits.complete.contains(name) {
        return;
    }
    assert!(visits.active.insert(name.into()), "Cyclic service dependency at {name}");
    for target in &edges[name] {
        visit(target, edges, visits);
    }
    visits.active.remove(name);
    visits.complete.insert(name.into());
}
fn declared_names(source: &str) -> BTreeSet<String> {
    let tree = rust_tree(source);
    matis::syntax::walk(tree.root_node())
        .filter(|n| matches!(n.kind(), "struct_item" | "enum_item" | "trait_item" | "type_item"))
        .map(|n| n.child_by_field_name("name").unwrap().utf8_text(source.as_bytes()).unwrap().to_owned())
        .collect()
}
#[test]
fn service_dependencies_are_acyclic() {
    let mut declarations = BTreeMap::new();
    let mut references = BTreeMap::new();
    for file in source_files(&root().join("services")) {
        let source = production(&fs::read_to_string(&file).unwrap());
        declarations.extend(declared_names(&source).into_iter().map(|name| (name, file.clone())));
        references.insert(file, identifiers(&source));
    }
    let edges: BTreeMap<_, _> = declarations
        .iter()
        .map(|(name, file)| {
            let targets = declarations
                .iter()
                .filter(|(target, owner)| *owner != file && references[file].contains(*target))
                .map(|(target, _)| target.clone())
                .collect();
            (name.clone(), targets)
        })
        .collect();
    let mut visits = Visits::default();
    edges.keys().for_each(|name| visit(name, &edges, &mut visits));
}
#[test]
fn repository_reference_checks_each_grouped_path_without_aliases() {
    for source in [
        "use crate::{persistence::{files as sibling}, domain};",
        "use crate::{domain::repositories, persistence::{files as sibling}};",
        "use crate::{services::nodes, persistence::{files as sibling}};",
        "use super::{files as sibling, database};",
        "use crate::persistence as sibling;",
        "use super::*;",
        "type Store = crate::persistence::files::Store;",
    ] {
        assert!(module_reference(source, "files"), "{source}");
    }
    for source in [
        "use crate::domain::{repositories, storage::FilePort};",
        "use crate::services::{repositories::RepositoryService, nodes::NodeService};",
        "use super::{database::run, models::graph_nodes};",
    ] {
        assert!(!module_reference(source, "repositories"), "{source}");
    }
}
