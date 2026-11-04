//! Python-compatible result and progress formatting.
use serde_json::Value;

/// Serialize JSON with Python-style separator spacing and unescaped Unicode.
///
/// Preserve array and object iteration order; return text without a newline.
pub fn python_json(value: &Value) -> String {
    match value {
        Value::Array(items) => format!("[{}]", items.iter().map(python_json).collect::<Vec<_>>().join(", ")),
        Value::Object(items) => format!(
            "{{{}}}",
            items
                .iter()
                .map(|(key, value)| format!("{}: {}", serde_json::to_string(key).unwrap(), python_json(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => value.to_string(),
    }
}
/// Borrow a named JSON string field, or an empty string if missing or not a string.
pub fn text<'a>(value: &'a Value, name: &str) -> &'a str {
    value[name].as_str().unwrap_or("")
}
/// Return `; ` followed by the string `error` field, or no suffix if absent/nonstring.
pub fn error_suffix(value: &Value) -> String {
    value.get("error").and_then(Value::as_str).map(|error| format!("; {error}")).unwrap_or_default()
}
/// Join arguments with POSIX shell quoting for a readable command diagnostic.
///
/// Quote empty or unsafe arguments and escape embedded single quotes. This
/// formats text only; the runner executes argument vectors without a shell.
pub fn shell_join(arguments: &[String]) -> String {
    arguments
        .iter()
        .map(|arg| {
            if !arg.is_empty() && arg.bytes().all(|ch| ch.is_ascii_alphanumeric() || b"_@%+=:,./-".contains(&ch)) {
                arg.clone()
            } else {
                format!("'{}'", arg.replace('\'', "'\"'\"'"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
/// Quote progress text with Python-style escapes for quotes, backslashes, and controls.
///
/// Leave other Unicode characters unchanged; this is not a source-code serializer.
pub fn repr(value: &str) -> String {
    let quote = if value.contains('\'') && !value.contains('"') { '"' } else { '\'' };
    let escaped: String = value.chars().map(|ch| character(ch, quote)).collect();
    format!("{quote}{escaped}{quote}")
}
fn character(ch: char, quote: char) -> String {
    match ch {
        '\\' => "\\\\".into(),
        '\n' => "\\n".into(),
        '\r' => "\\r".into(),
        '\t' => "\\t".into(),
        ch if ch == quote => format!("\\{ch}"),
        ch if ch.is_control() => format!("\\x{:02x}", ch as u32),
        ch => ch.to_string(),
    }
}
/// Format an OS error as Python-style `[Errno N] message`, without the Rust suffix.
///
/// Preserve errors that have no raw OS code as ordinary `anyhow` errors.
pub fn io_error(error: std::io::Error) -> anyhow::Error {
    let Some(code) = error.raw_os_error() else { return error.into() };
    let message = error.to_string();
    let message = message.split(" (os error ").next().unwrap();
    anyhow::anyhow!("[Errno {code}] {message}")
}

/// Add a quoted command or path to an OS error using Python-style formatting.
///
/// Leave errors without a raw OS code unchanged instead of appending `command`.
pub fn command_error(error: std::io::Error, command: &str) -> anyhow::Error {
    if error.raw_os_error().is_none() {
        return error.into();
    }
    anyhow::anyhow!("{}: {}", io_error(error), repr(command))
}
