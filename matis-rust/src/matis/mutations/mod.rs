//! Verified mutation manifests and opt-in, restoring command execution.
pub mod candidates;
pub mod config;
mod formatting;
pub mod manifests;
pub mod matching;
mod process;
pub mod report;
pub mod rules;
pub mod runner;
mod workspace;

pub use crate::domain::mutation_candidates::{canonical_json, digest};

#[cfg(test)]
#[path = "../../../tests/integration/mutations/manifests.rs"]
mod manifests_integration_tests;
#[cfg(test)]
#[path = "../../../tests/integration/mutations/runner.rs"]
mod runner_integration_tests;
#[cfg(test)]
#[path = "../../../tests/integration/mutations/runner_process.rs"]
mod runner_process_integration_tests;
#[cfg(test)]
#[path = "../../../tests/unit/mutations/runner.rs"]
mod runner_tests;
#[cfg(test)]
#[path = "../../../tests/unit/mutations/tests.rs"]
mod tests;
