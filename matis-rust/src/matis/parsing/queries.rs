//! Purpose-specific syntax queries for supported languages.
//! Includes tags from the pinned Python 1.20.0 wheel; see queries/LICENSE.
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, OnceLock},
};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Node, Query, QueryCapture, QueryCursor};
const QUERIES: &[(&str, &str)] = &[
    ("python", include_str!("queries/python.scm")),
    ("javascript", include_str!("queries/javascript.scm")),
    ("jsx", include_str!("queries/jsx.scm")),
    ("typescript", include_str!("queries/typescript.scm")),
    ("tsx", include_str!("queries/tsx.scm")),
    ("java", include_str!("queries/java.scm")),
    ("kotlin", include_str!("queries/kotlin.scm")),
    ("go", include_str!("queries/go.scm")),
    ("rust", include_str!("queries/rust.scm")),
    ("elixir", include_str!("queries/elixir.scm")),
];

/// Return declaration query text for the exact language ID, or `None` if no asset exists.
/// This lookup does not validate importer support or compile the query.
pub fn declarations(language: &str) -> Option<&'static str> {
    QUERIES.iter().find(|(name, _)| *name == language).map(|(_, query)| *query)
}

/// Return call-site selection rules, separate from the bundled declaration/reference tags.
pub fn calls(language: &str) -> &'static str {
    const CALLS: &[(&str, &str)] = &[
        ("python", include_str!("queries/calls/python.scm")),
        ("javascript", include_str!("queries/calls/javascript.scm")),
        ("jsx", include_str!("queries/calls/javascript.scm")),
        ("tsx", include_str!("queries/calls/javascript.scm")),
        ("typescript", include_str!("queries/calls/typescript.scm")),
        ("java", include_str!("queries/calls/java.scm")),
        ("kotlin", include_str!("queries/calls/kotlin.scm")),
        ("go", include_str!("queries/calls/go.scm")),
        ("rust", include_str!("queries/calls/rust.scm")),
        ("elixir", include_str!("queries/calls/elixir.scm")),
    ];
    CALLS.iter().find(|(name, _)| *name == language).expect("call queries require a supported language").1
}

/// Return supplemental import selection rules; native imports remain authoritative.
pub fn imports(language: &str) -> Option<&'static str> {
    match language {
        "kotlin" => Some(include_str!("queries/imports/kotlin.scm")),
        "rust" => Some(include_str!("queries/imports/rust.scm")),
        _ => None,
    }
}

/// Return language-specific metadata selectors, where the grammar has matching modifier groups.
pub fn metadata(language: &str) -> Option<&'static str> {
    match language {
        "python" => Some(include_str!("queries/metadata/python.scm")),
        "java" => Some(include_str!("queries/metadata/java.scm")),
        "kotlin" => Some(include_str!("queries/metadata/kotlin.scm")),
        _ => None,
    }
}

/// Return header syntax selectors; semantic base-name normalization stays in Rust.
pub fn inheritance(language: &str) -> Option<&'static str> {
    match language {
        "python" => Some(include_str!("queries/inheritance/python.scm")),
        "javascript" | "jsx" => Some(include_str!("queries/inheritance/javascript.scm")),
        "typescript" | "tsx" => Some(include_str!("queries/inheritance/typescript.scm")),
        "java" => Some(include_str!("queries/inheritance/java.scm")),
        "kotlin" => Some(include_str!("queries/inheritance/kotlin.scm")),
        _ => None,
    }
}

/// Select unique captured nodes in depth-first source order, omitting `@exclude` captures.
/// The embedded query must compile for the root's grammar; source bytes must match the tree.
pub fn selected_nodes<'tree>(
    (root, source): (Node<'tree>, &[u8]),
    query_source: &'static str,
    capture: &str,
) -> Vec<Node<'tree>> {
    type Cache = HashMap<(Language, &'static str), Arc<Query>>;
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let key = ((*root.language()).clone(), query_source);
    let query = CACHE
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .entry(key)
        .or_insert_with(|| Arc::new(Query::new(&root.language(), query_source).expect("valid embedded query")))
        .clone();
    let mut nodes = vec![];
    let mut excluded = HashSet::new();
    let mut seen = HashSet::new();
    for item in captures((root, source), &query) {
        let name = query.capture_names()[item.index as usize];
        if name == "exclude" {
            excluded.insert(item.node.id());
        } else if name == capture && seen.insert(item.node.id()) {
            nodes.push(item.node);
        }
    }
    nodes.retain(|node| !excluded.contains(&node.id()));
    // Parents precede descendants even when their byte ranges are identical (error recovery).
    nodes.sort_by_key(|node| {
        (node.start_byte(), Reverse(node.end_byte()), std::iter::successors(node.parent(), |n| n.parent()).count())
    });
    nodes
}

fn captures<'tree>((root, source): (Node<'tree>, &[u8]), query: &Query) -> Vec<QueryCapture<'tree>> {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, source);
    let mut captures = vec![];
    while let Some(found) = matches.next() {
        captures.extend_from_slice(found.captures());
    }
    captures
}
