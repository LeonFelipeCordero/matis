//! Declaration queries for supported languages.
//! Includes tags from the pinned Python 1.20.0 wheel; see queries/LICENSE.
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
