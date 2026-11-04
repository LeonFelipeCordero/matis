//! Strict, opt-in mutation rules and ordered, trusted test commands.
use super::rules::validate_rules;
use anyhow::{Result, bail};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationTarget {
    pub sources: Vec<String>,
    #[serde(default = "default_cwd")]
    pub cwd: String,
    pub command: Vec<String>,
    #[serde(default = "default_timeout", rename = "timeoutSeconds")]
    pub timeout_seconds: Number,
}
fn default_cwd() -> String {
    ".".into()
}
fn default_timeout() -> Number {
    120.into()
}

impl MutationTarget {
    /// Check source globs, command arguments, and the timeout without doing I/O.
    ///
    /// Reject empty lists or blank items, absolute/backslash/parent-traversing
    /// source patterns, and nonpositive or noninteger timeouts. Working directory
    /// containment and command executability are checked elsewhere.
    pub fn validate(&self) -> Result<()> {
        if self.sources.is_empty() || self.command.is_empty() {
            bail!("Source patterns and commands must not be empty");
        }
        if self.sources.iter().chain(&self.command).any(|item| item.trim().is_empty()) {
            bail!("Source patterns and command arguments must not be empty");
        }
        if self.sources.iter().any(|pattern| {
            pattern.starts_with('/') || pattern.contains('\\') || pattern.split('/').any(|part| part == "..")
        }) {
            bail!("Source patterns must be project-relative POSIX globs");
        }
        let timeout = self.timeout_seconds.as_str();
        if !timeout.bytes().all(|byte| byte.is_ascii_digit()) {
            bail!("timeoutSeconds must be a positive integer");
        }
        if timeout.bytes().all(|byte| byte == b'0') {
            bail!("timeoutSeconds must be greater than 0");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationConfig {
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub targets: IndexMap<String, MutationTarget>,
}

impl MutationConfig {
    /// Deserialize a strict mutation configuration and validate its rules and targets.
    ///
    /// Missing fields use empty defaults. Reject nonobjects, unknown fields,
    /// incorrect field types, unknown rules, and invalid targets or target names.
    pub fn from_value(value: &Value) -> Result<Self> {
        if !value.is_object() {
            bail!("Mutation configuration must be an object");
        }
        let config: Self = serde_json::from_value(value.clone())?;
        config.validate()?;
        Ok(config)
    }

    /// Check rule IDs, selectable target names, and each target without doing I/O.
    ///
    /// Accept empty rules/targets and duplicate rules. Reject unknown rules,
    /// blank or untrimmed names, commas, the reserved name `all`, or invalid targets.
    pub fn validate(&self) -> Result<()> {
        validate_rules(&self.rules)?;
        for (name, target) in &self.targets {
            if name.is_empty() || name != name.trim() || name.contains(',') || name == "all" {
                bail!("Target names must be nonempty, trimmed, contain no commas, and must not be 'all'");
            }
            target.validate()?;
        }
        Ok(())
    }
}

/// Read the mutation section of a project object, defaulting to no rules or targets.
///
/// Reject a nonobject project or an invalid mutation section. Other project fields
/// are not validated here; this function does not read configuration files.
pub fn from_project(value: &Value) -> Result<MutationConfig> {
    let Some(project) = value.as_object() else {
        bail!("Project configuration must be an object");
    };
    project.get("mutations").map(MutationConfig::from_value).unwrap_or_else(|| Ok(MutationConfig::default()))
}
