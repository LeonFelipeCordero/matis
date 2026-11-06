use crate::{mutations::config::MutationConfig, parsing::ignore_spec::IgnoreSpec};
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use std::{fs, io::ErrorKind, path::Path};

pub const LIMITS: [&str; 11] = [
    "maxFileLines",
    "maxFunctionLines",
    "maxParameters",
    "maxWeightedComplexity",
    "maxDecisionDepth",
    "maxNestedLoops",
    "maxCircularImportGroups",
    "maxChangeImpact",
    "maxInstability",
    "maxFanIn",
    "maxFanOut",
];

/// Return default check limits; null limits disable change-impact and instability checks.
pub fn default_config() -> Value {
    json!({"maxFileLines": 700, "maxFunctionLines": 150, "maxParameters": 3, "maxWeightedComplexity": 8,
        "maxDecisionDepth": 3, "maxNestedLoops": 0, "maxCircularImportGroups": 0,
        "maxChangeImpact": null, "maxInstability": null, "maxFanIn": 30, "maxFanOut": 20, "fanExclude": []})
}

/// Read `root/matis.json` and return check limits, excluding validated mutation settings.
/// Use defaults if the file is absent; do not search parent directories.
///
/// # Errors
/// Return file-reading, JSON, or project-setting validation errors.
pub fn load_config(root: &Path) -> Result<Value> {
    let mut config = load_project_config(root)?;
    config.as_object_mut().unwrap().remove("mutations");
    Ok(config)
}

/// Read `root/matis.json` into validated check and mutation settings, filling omitted defaults.
/// A missing file uses defaults; no parent directories are searched.
///
/// # Errors
/// Return an error for unreadable files, invalid JSON, or invalid settings.
pub fn load_project_config(root: &Path) -> Result<Value> {
    let path = root.join("matis.json");
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) if error.kind() == ErrorKind::NotFound => return validate_project_config(&json!({})),
        Err(error) => return Err(anyhow!("{}: {error}", path.display())),
    };
    let value = parse_project_source(&source).map_err(|error| {
        let message = validation_error("", (&format!("Invalid JSON: {error}"), "json_invalid"), &json!(source));
        anyhow!("Invalid {}: {}", path.display(), format_errors("ProjectConfig", &[message]))
    })?;
    validate_project_config(&value).map_err(|error| anyhow!("Invalid {}: {error}", path.display()))
}

struct ConfigTokens {
    strings: Vec<String>,
    nonfinite: Vec<(std::ops::Range<usize>, &'static str)>,
}

fn parse_project_source(source: &str) -> serde_json::Result<Value> {
    let tokens = config_tokens(source);
    if tokens.nonfinite.is_empty() {
        return serde_json::from_str(source);
    }
    let mut marker = "\0matis:nonfinite:".to_owned();
    while tokens.strings.iter().any(|text| text.starts_with(&marker)) {
        marker.push('_');
    }
    let mut normalized = String::new();
    let mut offset = 0;
    for (span, token) in tokens.nonfinite {
        normalized.push_str(&source[offset..span.start]);
        normalized.push_str(&serde_json::to_string(&format!("{marker}{token}")).unwrap());
        offset = span.end;
    }
    normalized.push_str(&source[offset..]);
    let mut value = serde_json::from_str(&normalized)?;
    restore_nonfinite(&mut value, &marker);
    Ok(value)
}
fn config_tokens(source: &str) -> ConfigTokens {
    let mut tokens = ConfigTokens { strings: Vec::new(), nonfinite: Vec::new() };
    let bytes = source.as_bytes();
    let mut offset = 0;
    while offset < bytes.len() {
        if bytes[offset] == b'"' {
            let end = quoted_end(bytes, offset);
            if let Ok(text) = serde_json::from_str(&source[offset..end]) {
                tokens.strings.push(text);
            }
            offset = end;
            continue;
        }
        let token =
            ["-Infinity", "Infinity", "NaN"].into_iter().find(|token| bytes[offset..].starts_with(token.as_bytes()));
        if let Some(token) = token {
            tokens.nonfinite.push((offset..offset + token.len(), token));
            offset += token.len();
            continue;
        }
        offset += 1;
    }
    tokens
}
fn quoted_end(bytes: &[u8], start: usize) -> usize {
    let mut offset = start + 1;
    while offset < bytes.len() {
        if bytes[offset] == b'"' {
            return offset + 1;
        }
        offset += if bytes[offset] == b'\\' { 2 } else { 1 };
    }
    bytes.len()
}
fn restore_nonfinite(value: &mut Value, marker: &str) {
    match value {
        Value::String(text) if text.starts_with(marker) => {
            // These Python JSON tokens exist only until strict config validation rejects them.
            *value = Value::Number(serde_json::Number::from_string_unchecked(text[marker.len()..].to_owned()));
        }
        Value::Array(values) => {
            for value in values {
                restore_nonfinite(value, marker);
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                restore_nonfinite(value, marker);
            }
        }
        _ => {}
    }
}

/// Validate check limits and fill omitted defaults without modifying `value`.
/// Null limits disable their checks.
///
/// # Errors
/// Reject unknown fields, wrong types, negative limits, and instability outside 0..=1.
pub fn validate_config(value: &Value) -> Result<Value> {
    validate(value, false)
}
/// Validate project check and mutation settings and return a default-filled copy.
///
/// # Errors
/// Reject unknown fields, invalid limits, and invalid mutation settings.
pub fn validate_project_config(value: &Value) -> Result<Value> {
    validate(value, true)
}

fn validate(value: &Value, project: bool) -> Result<Value> {
    let title = if project { "ProjectConfig" } else { "CheckConfig" };
    if !value.is_object() {
        bail!("{}", format_errors(title, &[validation_error("", ("Input should be an object", "model_type"), value)]));
    }
    let mut config = default_config();
    let mut errors = extra_errors(value, project);
    errors.extend(apply_limits(value, &mut config));
    errors.extend(apply_help_links(value, &mut config));
    errors.extend(apply_fan_exclude(value, &mut config));
    if project {
        errors.extend(apply_mutations(value, &mut config)?);
    }
    if !errors.is_empty() {
        bail!("{}", format_errors(title, &errors));
    }
    Ok(config)
}
fn extra_errors(value: &Value, project: bool) -> Vec<String> {
    value
        .as_object()
        .unwrap()
        .iter()
        .filter(|(name, _)| {
            !(LIMITS.contains(&name.as_str())
                || *name == "helpLinks"
                || *name == "fanExclude"
                || project && *name == "mutations")
        })
        .map(|(name, input)| validation_error(name, ("Extra inputs are not permitted", "extra_forbidden"), input))
        .collect()
}
fn apply_help_links(value: &Value, config: &mut Value) -> Vec<String> {
    let Some(links) = value.get("helpLinks") else {
        return vec![];
    };
    let Some(entries) = links.as_object() else {
        return vec![validation_error("helpLinks", ("Input should be an object", "model_type"), links)];
    };
    let mut errors = Vec::new();
    for (rule, link) in entries {
        let error = if !LIMITS.contains(&rule.as_str()) && rule != "analysis" {
            Some(("Unknown check rule", "extra_forbidden"))
        } else if !link.as_str().is_some_and(|url| {
            url::Url::parse(url).is_ok_and(|parsed| matches!(parsed.scheme(), "http" | "https"))
                && url
                    .split_once("://")
                    .is_some_and(|(_, rest)| rest.split(['/', '?', '#']).next().is_some_and(|host| !host.is_empty()))
                && !url.chars().any(|ch| ch.is_whitespace() || ch.is_control() || matches!(ch, '<' | '>'))
        }) {
            Some(("Use a nonempty HTTP or HTTPS link without whitespace or angle brackets", "value_error"))
        } else {
            None
        };
        if let Some(error) = error {
            errors.push(validation_error(&format!("helpLinks.{rule}"), error, link));
        }
    }
    config["helpLinks"] = links.clone();
    errors
}
fn apply_limits(value: &Value, config: &mut Value) -> Vec<String> {
    let mut errors = Vec::new();
    for name in LIMITS {
        let Some(input) = value.get(name) else {
            continue;
        };
        if let Some(error) = limit_error(name, input) {
            errors.push(error);
            continue;
        }
        config[name] = normalized_limit(name, input);
    }
    errors
}
fn apply_fan_exclude(value: &Value, config: &mut Value) -> Vec<String> {
    let Some(input) = value.get("fanExclude") else {
        return vec![];
    };
    let Some(patterns) = input.as_array() else {
        return vec![validation_error("fanExclude", ("Input should be a valid array", "list_type"), input)];
    };
    let mut errors = Vec::new();
    for (index, pattern) in patterns.iter().enumerate() {
        let field = format!("fanExclude.{index}");
        match pattern.as_str() {
            None => errors.push(validation_error(&field, ("Input should be a valid string", "string_type"), pattern)),
            Some(text) => {
                if let Err(error) = IgnoreSpec::new(text) {
                    errors.push(validation_error(
                        &field,
                        (&format!("Invalid gitignore pattern: {error}"), "value_error"),
                        pattern,
                    ));
                }
            }
        }
    }
    config["fanExclude"] = input.clone();
    errors
}
fn normalized_limit(name: &str, input: &Value) -> Value {
    if name == "maxInstability" && !input.is_null() {
        return json!(input.as_f64().unwrap());
    }
    if integer_text(input) == Some("-0") {
        return json!(0);
    }
    input.clone()
}
fn apply_mutations(value: &Value, config: &mut Value) -> Result<Vec<String>> {
    let mutations = value.get("mutations").cloned().unwrap_or_else(|| json!({}));
    match MutationConfig::from_value(&mutations) {
        Ok(mutations) => {
            config["mutations"] = serde_json::to_value(mutations)?;
            Ok(vec![])
        }
        Err(_) => Ok(mutation_errors(&mutations)),
    }
}

fn limit_error(name: &str, input: &Value) -> Option<String> {
    if input.is_null() {
        return None;
    }
    if let Some(error) = numeric_type_error(name, input) {
        return Some(error);
    }
    let constraint = limit_constraint(name, input)?;
    Some(validation_error(name, constraint, input))
}
fn numeric_type_error(name: &str, input: &Value) -> Option<String> {
    let (valid, message) = if name == "maxInstability" {
        (input.is_number(), ("Input should be a valid number", "float_type"))
    } else {
        (integer_text(input).is_some(), ("Input should be a valid integer", "int_type"))
    };
    (!valid).then(|| validation_error(name, message, input))
}
/// Borrow a JSON number's integer spelling, including `-0`, or return `None`.
/// Decimal and exponent spellings are not integers for strict configuration validation.
pub(crate) fn integer_text(value: &Value) -> Option<&str> {
    let text = value.as_number()?.as_str();
    let digits = text.strip_prefix('-').unwrap_or(text);
    (!digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())).then_some(text)
}
fn limit_constraint(name: &str, input: &Value) -> Option<(&'static str, &'static str)> {
    if name == "maxInstability" {
        return number_constraint(input.as_f64().unwrap_or(f64::NAN));
    }
    integer_text(input)
        .filter(|text| text.starts_with('-') && *text != "-0")
        .map(|_| ("Input should be greater than or equal to 0", "greater_than_equal"))
}
fn number_constraint(number: f64) -> Option<(&'static str, &'static str)> {
    if !number.is_finite() {
        return Some(("Input should be a finite number", "finite_number"));
    }
    if number < 0.0 {
        return Some(("Input should be greater than or equal to 0", "greater_than_equal"));
    }
    if number > 1.0 {
        return Some(("Input should be less than or equal to 1", "less_than_equal"));
    }
    None
}

fn mutation_errors(value: &Value) -> Vec<String> {
    let Some(object) = value.as_object() else {
        return vec![validation_error("mutations", ("Input should be an object", "model_type"), value)];
    };
    let mut errors = Vec::new();
    for (name, input) in object {
        if name != "rules" && name != "targets" {
            errors.push(validation_error(
                &format!("mutations.{name}"),
                ("Extra inputs are not permitted", "extra_forbidden"),
                input,
            ));
        }
    }
    if let Some(rules) = object.get("rules") {
        errors.extend(mutation_rule_errors(rules));
    }
    if let Some(targets) = object.get("targets") {
        errors.extend(mutation_target_errors(targets));
    }
    errors
}
fn mutation_target_errors(targets: &Value) -> Vec<String> {
    let Some(entries) = targets.as_object() else {
        return vec![validation_error(
            "mutations.targets",
            ("Input should be a valid dictionary", "dict_type"),
            targets,
        )];
    };
    let mut errors = Vec::new();
    for (name, target) in entries {
        errors.extend(target_errors(&format!("mutations.targets.{name}"), target));
    }
    if errors.is_empty()
        && entries.keys().any(|name| name.is_empty() || name != name.trim() || name.contains(',') || name == "all")
    {
        errors.push(validation_error(
            "mutations.targets",
            (
                "Value error, Target names must be nonempty, trimmed, contain no commas, and must not be 'all'",
                "value_error",
            ),
            targets,
        ));
    }
    errors
}
fn target_errors(prefix: &str, target: &Value) -> Vec<String> {
    let Some(object) = target.as_object() else {
        return vec![validation_error(prefix, ("Input should be an object", "model_type"), target)];
    };
    let mut errors = Vec::new();
    for name in ["sources", "cwd", "command", "timeoutSeconds"] {
        errors.extend(target_field_errors(prefix, name, target));
    }
    for (name, input) in object {
        if !["sources", "cwd", "command", "timeoutSeconds"].contains(&name.as_str()) {
            errors.push(validation_error(
                &format!("{prefix}.{name}"),
                ("Extra inputs are not permitted", "extra_forbidden"),
                input,
            ));
        }
    }
    errors
}
fn target_field_errors(prefix: &str, name: &str, target: &Value) -> Vec<String> {
    let field = format!("{prefix}.{name}");
    let Some(input) = target.get(name) else {
        return if name == "sources" || name == "command" {
            vec![validation_error(&field, ("Field required", "missing"), target)]
        } else {
            vec![]
        };
    };
    match name {
        "sources" | "command" => target_list_errors(&field, name, input),
        "cwd" if !input.is_string() => {
            vec![validation_error(&field, ("Input should be a valid string", "string_type"), input)]
        }
        "timeoutSeconds" => {
            timeout_error(input).map(|constraint| validation_error(&field, constraint, input)).into_iter().collect()
        }
        _ => vec![],
    }
}
fn timeout_error(input: &Value) -> Option<(&'static str, &'static str)> {
    match integer_text(input) {
        None => Some(("Input should be a valid integer", "int_type")),
        Some(number) if number.starts_with('-') || number == "0" => {
            Some(("Input should be greater than 0", "greater_than"))
        }
        _ => None,
    }
}
fn target_list_errors(field: &str, name: &str, input: &Value) -> Vec<String> {
    let Some(items) = input.as_array() else {
        return vec![validation_error(field, ("Input should be a valid array", "list_type"), input)];
    };
    if items.is_empty() {
        return vec![validation_error(
            field,
            ("List should have at least 1 item after validation, not 0", "too_short"),
            input,
        )];
    }
    let mut errors = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if !item.is_string() {
            errors.push(validation_error(
                &format!("{field}.{index}"),
                ("Input should be a valid string", "string_type"),
                item,
            ));
        }
    }
    if !errors.is_empty() {
        return errors;
    }
    errors.extend(
        target_list_constraint(name, items).map(|message| validation_error(field, (message, "value_error"), input)),
    );
    errors
}
fn target_list_constraint(name: &str, items: &[Value]) -> Option<&'static str> {
    let strings: Vec<_> = items.iter().map(|item| item.as_str().unwrap()).collect();
    if strings.iter().any(|item| item.trim().is_empty()) {
        Some("Value error, Source patterns and command arguments must not be empty")
    } else if name == "sources"
        && strings.iter().any(|pattern| {
            pattern.starts_with('/') || pattern.contains('\\') || pattern.split('/').any(|part| part == "..")
        })
    {
        Some("Value error, Source patterns must be project-relative POSIX globs")
    } else {
        None
    }
}

fn mutation_rule_errors(rules: &Value) -> Vec<String> {
    let Some(entries) = rules.as_array() else {
        return vec![validation_error("mutations.rules", ("Input should be a valid array", "list_type"), rules)];
    };
    let mut errors = Vec::new();
    for (index, rule) in entries.iter().enumerate() {
        if !rule.is_string() {
            errors.push(validation_error(
                &format!("mutations.rules.{index}"),
                ("Input should be a valid string", "string_type"),
                rule,
            ));
        }
    }
    if errors.is_empty() {
        errors.extend(unknown_rule_error(entries, rules));
    }
    errors
}
fn unknown_rule_error(entries: &[Value], rules: &Value) -> Vec<String> {
    let ids: Vec<_> = entries.iter().map(|rule| rule.as_str().unwrap().to_owned()).collect();
    crate::mutations::rules::validate_rules(&ids)
        .err()
        .map(|error| validation_error("mutations.rules", (&format!("Value error, {error}"), "value_error"), rules))
        .into_iter()
        .collect()
}

fn validation_error(field: &str, error: (&str, &str), input: &Value) -> String {
    let (message, kind) = error;
    let heading = if field.is_empty() { String::new() } else { format!("{field}\n") };
    format!(
        "{heading}  {message} [type={kind}, input_value={}, input_type={}]\n    For further information visit https://errors.pydantic.dev/2.14/v/{kind}",
        abbreviated_repr(input),
        input_type(input)
    )
}
fn input_type(value: &Value) -> &str {
    match value {
        Value::Null => "NoneType",
        Value::Bool(_) => "bool",
        Value::String(_) => "str",
        Value::Array(_) => "list",
        Value::Object(_) => "dict",
        Value::Number(_) if integer_text(value).is_some() => "int",
        Value::Number(_) => "float",
    }
}
fn format_errors(title: &str, errors: &[String]) -> String {
    format!(
        "{} validation error{} for {title}\n{}",
        errors.len(),
        if errors.len() == 1 { "" } else { "s" },
        errors.join("\n")
    )
}
fn python_string(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') { '"' } else { '\'' };
    let escaped: String = text.chars().map(|ch| python_character(ch, quote)).collect();
    format!("{quote}{escaped}{quote}")
}
fn python_character(ch: char, quote: char) -> String {
    match ch {
        '\\' => "\\\\".into(),
        '\n' => "\\n".into(),
        '\r' => "\\r".into(),
        '\t' => "\\t".into(),
        _ if ch == quote => format!("\\{quote}"),
        _ if ch.is_control() => format!("\\x{:02x}", ch as u32),
        _ => ch.to_string(),
    }
}
fn python_repr(value: &Value) -> String {
    match value {
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::String(text) => python_string(text),
        Value::Array(values) => format!("[{}]", values.iter().map(python_repr).collect::<Vec<_>>().join(", ")),
        Value::Object(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(key, value)| format!("{}: {}", python_string(key), python_repr(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Number(number) => python_number(number),
    }
}
fn python_number(number: &serde_json::Number) -> String {
    let value = Value::Number(number.clone());
    if integer_text(&value).is_some() {
        return number.to_string();
    }
    let number = number.as_str().parse::<f64>().unwrap_or(f64::NAN);
    if number.is_nan() {
        return "nan".into();
    }
    let text = format!("{number:?}");
    let Some((mantissa, exponent)) = text.split_once('e') else {
        return text;
    };
    format!("{mantissa}e{:+03}", exponent.parse::<i32>().unwrap())
}
fn abbreviated_repr(value: &Value) -> String {
    let text = python_repr(value);
    let chars: Vec<_> = text.chars().collect();
    if chars.len() <= 50 {
        return text;
    }
    format!("{}...{}", chars[..25].iter().collect::<String>(), chars[chars.len() - 24..].iter().collect::<String>())
}
