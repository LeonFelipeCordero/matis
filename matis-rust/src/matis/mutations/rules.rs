//! Fixed, versioned syntactic edits, not arbitrary replacement templates.
use crate::domain::mutation_candidates::MutationKind;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;

pub const LANGUAGES: &[&str] = &["python", "javascript", "typescript", "tsx", "java", "kotlin", "rust", "go", "elixir"];
const NON_ELIXIR: &[&str] = &["python", "javascript", "typescript", "tsx", "java", "kotlin", "rust", "go"];
const STRICT: &[&str] = &["javascript", "typescript", "tsx", "elixir"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleMapping {
    pub language: String,
    pub original: String,
    pub replacement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationRule {
    pub id: String,
    #[serde(default = "default_version")]
    pub version: i64,
    pub kind: MutationKind,
    pub mappings: Vec<RuleMapping>,
    #[serde(default = "default_limitations")]
    pub limitations: String,
}

fn default_version() -> i64 {
    1
}
fn default_limitations() -> String {
    "Syntactic only; no type or runtime validity guarantee.".into()
}

fn rule(id: &str, edit: (MutationKind, &str, &str), languages: &[&str]) -> MutationRule {
    let (kind, original, replacement) = edit;
    MutationRule {
        id: id.into(),
        version: default_version(),
        kind,
        mappings: languages
            .iter()
            .map(|language| RuleMapping {
                language: (*language).into(),
                original: original.into(),
                replacement: replacement.into(),
            })
            .collect(),
        limitations: default_limitations(),
    }
}

fn logical(id: &str, edit: (&str, &str), python_edit: (&str, &str)) -> MutationRule {
    let mut result = rule(id, (MutationKind::Logical, edit.0, edit.1), NON_ELIXIR);
    result.mappings[0].original = python_edit.0.into();
    result.mappings[0].replacement = python_edit.1.into();
    result
}

fn arithmetic_rules() -> Vec<MutationRule> {
    [
        ("arithmetic.add-to-subtract", "+", "-"),
        ("arithmetic.subtract-to-add", "-", "+"),
        ("arithmetic.multiply-to-divide", "*", "/"),
        ("arithmetic.divide-to-multiply", "/", "*"),
    ]
    .into_iter()
    .map(|(id, original, replacement)| rule(id, (MutationKind::Arithmetic, original, replacement), LANGUAGES))
    .collect()
}

fn comparison_rules() -> Vec<MutationRule> {
    [
        ("comparison.equal-to-not-equal", "==", "!=", LANGUAGES),
        ("comparison.not-equal-to-equal", "!=", "==", LANGUAGES),
        ("comparison.less-to-less-equal", "<", "<=", LANGUAGES),
        ("comparison.less-equal-to-less", "<=", "<", LANGUAGES),
        ("comparison.greater-to-greater-equal", ">", ">=", LANGUAGES),
        ("comparison.greater-equal-to-greater", ">=", ">", LANGUAGES),
        ("comparison.less-to-greater", "<", ">", LANGUAGES),
        ("comparison.greater-to-less", ">", "<", LANGUAGES),
        ("comparison.strict-equal-to-strict-not-equal", "===", "!==", STRICT),
        ("comparison.strict-not-equal-to-strict-equal", "!==", "===", STRICT),
    ]
    .into_iter()
    .map(|(id, original, replacement, languages)| {
        rule(id, (MutationKind::Comparison, original, replacement), languages)
    })
    .collect()
}

fn logical_rules() -> Vec<MutationRule> {
    use MutationKind::Logical;
    vec![
        logical("logical.and-to-or", ("&&", "||"), ("and", "or")),
        logical("logical.or-to-and", ("||", "&&"), ("or", "and")),
        rule("logical.strict-and-to-or", (Logical, "and", "or"), &["elixir"]),
        rule("logical.strict-or-to-and", (Logical, "or", "and"), &["elixir"]),
        rule("logical.truthy-and-to-or", (Logical, "&&", "||"), &["elixir"]),
        rule("logical.truthy-or-to-and", (Logical, "||", "&&"), &["elixir"]),
    ]
}

fn assignment_rules() -> Vec<MutationRule> {
    [
        ("assignment.add-to-subtract", "+=", "-="),
        ("assignment.subtract-to-add", "-=", "+="),
        ("assignment.multiply-to-divide", "*=", "/="),
        ("assignment.divide-to-multiply", "/=", "*="),
    ]
    .into_iter()
    .map(|(id, original, replacement)| rule(id, (MutationKind::Assignment, original, replacement), NON_ELIXIR))
    .collect()
}

pub static REGISTRY: LazyLock<Vec<MutationRule>> =
    LazyLock::new(|| [arithmetic_rules(), comparison_rules(), logical_rules(), assignment_rules()].concat());

/// Reject the first rule ID absent from the versioned registry.
///
/// Accept empty selections and duplicates without changing their order or input.
pub fn validate_rules(rules: &[String]) -> Result<()> {
    for id in rules {
        if !REGISTRY.iter().any(|rule| rule.id == *id) {
            bail!("Unknown mutation rule: {id}");
        }
    }
    Ok(())
}

/// Read a JSON array of known string rule IDs, preserving order and duplicates.
///
/// Return an empty vector for an empty array. Reject nonarrays, nonstring items,
/// and unknown rule IDs without returning a partial selection.
pub fn validate_rules_value(value: &Value) -> Result<Vec<String>> {
    let Some(items) = value.as_array() else {
        bail!("Mutation rules must be a list of rule IDs");
    };
    let mut result = Vec::new();
    for item in items {
        let Some(id) = item.as_str() else {
            bail!("Unknown mutation rule: {item}");
        };
        result.push(id.to_owned());
    }
    validate_rules(&result)?;
    Ok(result)
}
