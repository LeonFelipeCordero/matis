use super::config::{MutationConfig, MutationTarget};
use indexmap::IndexMap;

pub use crate::paths::globs::matches;

/// Borrow targets whose source globs match a project-relative POSIX path.
///
/// Preserve configuration order and include each target at most once. Match
/// case-sensitively from the root; only a full `**` component crosses separators.
/// This does not inspect files or validate the path or configuration.
pub fn matching_targets<'a>(path: &str, config: &'a MutationConfig) -> IndexMap<&'a str, &'a MutationTarget> {
    config
        .targets
        .iter()
        .filter(|(_, target)| target.sources.iter().any(|pattern| matches(path, pattern)))
        .map(|(name, target)| (name.as_str(), target))
        .collect()
}
