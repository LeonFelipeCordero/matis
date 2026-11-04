use regex::Regex;
use std::{cmp::Ordering, sync::LazyLock};

const MAX_DECIMAL_DIGITS: usize = 4300; // CPython's default integer-string conversion limit.
static DECIMAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\A\p{Nd}\z").unwrap());
static NONPRINTING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\A[\p{C}\p{Z}]\z").unwrap());

/// Normalize a Python-style signed decimal to ASCII, removing leading zeros.
///
/// Accept surrounding Unicode whitespace, Unicode decimal digits, and single
/// underscores between digits. Return `None` for invalid syntax or more than
/// 4300 digits before zero removal; normalize negative zero to `0`.
pub fn normalize(value: &str) -> Option<String> {
    let value = value.trim_matches(char::is_whitespace);
    let (negative, body) = sign(value);
    let mut digits = String::new();
    let mut previous_digit = false;
    for character in body.chars() {
        if character == '_' && previous_digit {
            previous_digit = false;
            continue;
        }
        digits.push(decimal_digit(character)?);
        previous_digit = true;
    }
    if !previous_digit || digits.len() > MAX_DECIMAL_DIGITS {
        return None;
    }
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Some("0".into());
    }
    Some(format!("{}{digits}", if negative { "-" } else { "" }))
}
fn sign(value: &str) -> (bool, &str) {
    if let Some(body) = value.strip_prefix('-') {
        return (true, body);
    }
    (false, value.strip_prefix('+').unwrap_or(value))
}
fn decimal_digit(character: char) -> Option<char> {
    if character.is_ascii_digit() {
        return Some(character);
    }
    if !matches(&DECIMAL, character) {
        return None;
    }
    let code = character as u32;
    let mut first = code;
    while first > 0 && char::from_u32(first - 1).is_some_and(|previous| matches(&DECIMAL, previous)) {
        first -= 1;
    }
    char::from_u32(u32::from(b'0') + (code - first) % 10)
}
fn matches(pattern: &Regex, character: char) -> bool {
    pattern.is_match(character.encode_utf8(&mut [0; 4]))
}
/// Test a normalized decimal against inclusive bounds without integer overflow.
///
/// `value` must be canonical ASCII decimal as returned by [`normalize`]. Negative
/// values fail; `None` for the upper bound means no maximum.
pub fn in_range(value: &str, range: (usize, Option<usize>)) -> bool {
    !value.starts_with('-')
        && magnitude(value, range.0) != Ordering::Less
        && range.1.is_none_or(|maximum| magnitude(value, maximum) != Ordering::Greater)
}
fn magnitude(value: &str, bound: usize) -> Ordering {
    let bound = bound.to_string();
    value.len().cmp(&bound.len()).then_with(|| value.cmp(&bound))
}
/// Format inclusive bounds as `min<=x<=max`, or `x>=min` without a maximum.
pub fn range_description(range: (usize, Option<usize>)) -> String {
    match range {
        (minimum, Some(maximum)) => format!("{minimum}<=x<={maximum}"),
        (minimum, None) => format!("x>={minimum}"),
    }
}
/// Quote a value for CLI validation errors using Python-style string escapes.
///
/// Escape backslashes, the selected quote, and nonprinting Unicode characters.
pub fn repr(value: &str) -> String {
    let quote = if value.contains('\'') && !value.contains('"') { '"' } else { '\'' };
    let contents: String = value.chars().map(|character| escaped(character, quote)).collect();
    format!("{quote}{contents}{quote}")
}
fn escaped(character: char, quote: char) -> String {
    match character {
        '\n' => "\\n".into(),
        '\r' => "\\r".into(),
        '\t' => "\\t".into(),
        '\\' => "\\\\".into(),
        character if character == quote => format!("\\{character}"),
        character => escaped_other(character),
    }
}
fn escaped_other(character: char) -> String {
    if character != ' ' && matches(&NONPRINTING, character) {
        escaped_codepoint(character as u32)
    } else {
        character.to_string()
    }
}
fn escaped_codepoint(code: u32) -> String {
    if code < 256 {
        format!("\\x{code:02x}")
    } else if code <= 0xffff {
        format!("\\u{code:04x}")
    } else {
        format!("\\U{code:08x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn python_decimal_lexical_forms() {
        for (text, expected) in [
            (" 1_0 ", "10"),
            ("１００", "100"),
            ("٢", "2"),
            ("𝟚𝟜", "24"),
            ("+०_२", "2"),
            ("-０_０", "0"),
            ("\u{85}١٠\u{a0}", "10"),
        ] {
            assert_eq!(normalize(text).as_deref(), Some(expected), "{text:?}");
        }
    }
    #[test]
    fn rejects_nondecimal_and_bad_separators() {
        for text in ["", " ", "+", "_1", "1_", "1__0", "1 0", "²", "四", "1.0", "0x10", "−1", "\u{1c}1", "1\u{1f}"]
        {
            assert!(normalize(text).is_none(), "{text:?}");
        }
    }
    #[test]
    fn matches_cpython_default_decimal_digit_limit_before_zero_trimming() {
        assert!(normalize(&"9".repeat(4300)).is_some());
        assert!(normalize(&"9".repeat(4301)).is_none());
        assert!(normalize(&format!("{}1", "0".repeat(4300))).is_none());
    }
    #[test]
    fn canonical_range_checks_do_not_overflow() {
        let huge = "1".repeat(100);
        assert!(!in_range(&huge, (1, Some(100))));
        assert!(in_range(&huge, (1, None)));
        assert!(!in_range(&format!("-{huge}"), (0, None)));
        assert_eq!(repr("\t'\n"), "\"\\t'\\n\"");
        assert_eq!(repr("\u{a0}"), "'\\xa0'");
    }
}
