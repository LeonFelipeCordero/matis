//! Verified mutation manifests and opt-in, restoring command execution.
pub mod config;
mod formatting;
pub mod manifests;
pub mod matching;
mod process;
pub mod report;
pub mod rules;
pub mod runner;
mod workspace;

use serde_json::Value;
use sha2::{Digest, Sha256};

/// Return the lowercase hexadecimal SHA-256 digest of the exact input bytes.
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Serialize a copy of JSON with recursively sorted keys and no extra whitespace.
///
/// Preserve Unicode without ASCII escaping. This canonical representation is
/// used for manifest identity hashes; the input value is not changed.
pub fn canonical_json(value: &Value) -> String {
    let mut sorted = value.clone();
    sorted.sort_all_objects();
    sorted.to_string()
}

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
