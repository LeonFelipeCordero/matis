//! Independent language registries for metrics and name resolution.
pub mod base;
pub mod callable_selection;
pub mod elixir;
pub mod generic;
pub mod go;
pub mod java;
pub mod javascript;
pub mod kotlin;
pub mod module_index;
pub mod parameters;
pub mod python;
pub mod resolution;
pub mod resolution_metadata;
pub mod resolution_paths;
pub mod resolution_policy;
pub mod rust;
pub mod script_python_selection;
pub mod script_resolution;
pub mod typescript;
pub use base::{MetricAdapter, MetricPolicy};
pub use resolution_policy::ResolutionAdapter;

pub const DEDICATED_LANGUAGES: &[&str] = crate::parsing::discovery::SUPPORTED_LANGUAGES;
/// Create a resolution policy for `language`; unknown languages use conservative generic rules.
pub fn adapter_for_language(language: &str) -> ResolutionAdapter {
    ResolutionAdapter { language: language.into() }
}
/// Return the resolution policy for `language`, including generic fallback for unknown names.
pub fn get_resolution_adapter(language: &str) -> ResolutionAdapter {
    adapter_for_language(language)
}
/// Create complexity and parameter-count policies for a supported importer language.
///
/// # Panics
/// Panic if `language` is not supported by the Rust importer.
pub fn metric_adapter_for_language(language: &str) -> MetricAdapter {
    assert!(
        crate::parsing::discovery::is_supported_language(language),
        "Unsupported Rust importer language: {language}"
    );
    MetricAdapter::new(language)
}
/// Return the metric adapter for a supported importer language.
///
/// # Panics
/// Panic if `language` is not supported by the Rust importer.
pub fn get_metric_adapter(language: &str) -> MetricAdapter {
    metric_adapter_for_language(language)
}
/// Count explicit inputs for a callable `node` using its language and matching `source` bytes.
/// Return `None` when the header cannot be measured.
///
/// # Panics
/// Panic if `language` is not supported by the Rust importer.
pub fn parameter_count(language: &str, node: tree_sitter::Node<'_>, source: &[u8]) -> Option<usize> {
    metric_adapter_for_language(language).parameter_count(node, source)
}
/// Return receiver-policy registration status for supported languages, not a verification guarantee.
pub fn receiver_coverage() -> serde_json::Value {
    let languages: Vec<_> = crate::parsing::discovery::supported_languages().iter().map(String::as_str).collect();
    generic::receiver_coverage(&languages, DEDICATED_LANGUAGES)
}
