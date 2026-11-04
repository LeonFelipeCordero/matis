//! GitIgnoreSpec-compatible matching, including directory-marker priority.
//! A plain Git matcher differs for `a/**` followed by a directory negation.
use anyhow::{Result, bail};
use regex::Regex;

const DIRECTORY: &str = "(?P<directory>/)";
const OPTIONAL_DIRECTORY: &str = "(?:(?P<directory>/)|\\n?\\z)";

#[derive(Clone)]
struct Pattern {
    regex: Regex,
    include: bool,
}

#[derive(Clone, Default)]
pub struct IgnoreSpec {
    patterns: Vec<Pattern>,
}

impl IgnoreSpec {
    /// Compile GitIgnoreSpec-compatible rules from ignore-file text.
    /// Blank lines and comments add no rules; negations can re-include paths.
    ///
    /// # Errors
    /// Returns an error if a pattern is invalid or its generated regular expression cannot compile.
    pub fn new(content: &str) -> Result<Self> {
        let lines = content.split(|c| {
            matches!(
                c,
                '\n' | '\r' | '\u{000b}' | '\u{000c}' | '\u{001c}'..='\u{001e}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
            )
        });
        let patterns = lines.map(compile_pattern).collect::<Result<Vec<_>>>()?.into_iter().flatten().collect();
        Ok(Self { patterns })
    }

    /// Match a `/`-separated path relative to this ignore file; append `/` for directories.
    /// Return `Some(true)` to ignore, `Some(false)` to re-include, or `None` for no match.
    /// Later rules take priority, except direct file matches override directory-marker matches.
    pub fn check_file(&self, path: &str) -> Option<bool> {
        let mut result = None;
        for pattern in self.patterns.iter().rev() {
            let Some(captures) = pattern.regex.captures(path) else {
                continue;
            };
            if captures.name("directory").is_none() {
                return Some(pattern.include);
            }
            if result.is_none() {
                result = Some(pattern.include);
            }
        }
        result
    }
}

fn compile_pattern(pattern: &str) -> Result<Option<Pattern>> {
    let pattern = if pattern.ends_with("\\ ") { pattern } else { pattern.trim_end_matches(python_whitespace) };
    if noop_pattern(pattern) {
        return Ok(None);
    }
    let (include, value) = match pattern.strip_prefix('!') {
        Some(value) => (false, value),
        None => (true, pattern),
    };
    let Some(regex) = pattern_regex(value)? else {
        return Ok(None);
    };
    Ok(Some(Pattern { regex: Regex::new(&regex)?, include }))
}

fn noop_pattern(pattern: &str) -> bool {
    pattern.is_empty() || pattern.starts_with('#') || pattern == "/"
}

fn pattern_regex(pattern: &str) -> Result<Option<String>> {
    let is_directory = pattern.ends_with('/');
    let segments = normalize_segments(pattern)?;
    match regex_override(&segments, is_directory) {
        Some(regex) => Ok(Some(regex.to_owned())),
        None => translate_segments(&segments, is_directory),
    }
}

fn python_whitespace(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{001c}'..='\u{001f}')
}

fn normalize_segments(pattern: &str) -> Result<Vec<&str>> {
    let mut segments: Vec<_> = pattern.split('/').collect();
    if segments[0].is_empty() {
        segments.remove(0);
    } else if single_segment(&segments) && segments[0] != "**" {
        segments.insert(0, "**");
    }
    if segments.is_empty() {
        bail!("Invalid git pattern: {pattern:?}");
    }
    if segments.last() == Some(&"") {
        *segments.last_mut().unwrap() = "**";
    }
    segments.dedup_by(|a, b| *a == "**" && *b == "**");
    Ok(segments)
}

fn single_segment(segments: &[&str]) -> bool {
    segments.len() == 1 || segments.len() == 2 && segments[1].is_empty()
}

fn regex_override(segments: &[&str], is_directory: bool) -> Option<&'static str> {
    match segments {
        ["**"] => Some(if is_directory { DIRECTORY } else { "." }),
        ["**", "*"] => Some("."),
        ["**", "*", "**"] => Some(if is_directory { DIRECTORY } else { "/" }),
        _ => None,
    }
}

#[derive(Default)]
struct SegmentTranslation {
    regex: String,
    need_slash: bool,
}
impl SegmentTranslation {
    fn append(&mut self, segment: &str, (index, last, is_directory): (usize, bool, bool)) -> Result<bool> {
        if segment == "**" {
            self.regex.push_str(double_star(index, last, is_directory));
            if index > 0 && !last {
                self.need_slash = true;
            }
            return Ok(true);
        }
        let Some(glob) = segment_glob(segment)? else {
            return Ok(false);
        };
        if index == 0 {
            self.regex.push('^');
        }
        if self.need_slash {
            self.regex.push('/');
        }
        self.regex.push_str(&glob);
        if last {
            self.regex.push_str(OPTIONAL_DIRECTORY);
        }
        self.need_slash = true;
        Ok(true)
    }
}

fn translate_segments(segments: &[&str], is_directory: bool) -> Result<Option<String>> {
    let mut translation = SegmentTranslation::default();
    for (index, segment) in segments.iter().enumerate() {
        if !translation.append(segment, (index, index + 1 == segments.len(), is_directory))? {
            return Ok(None);
        }
    }
    Ok(Some(translation.regex))
}

fn double_star(index: usize, last: bool, is_directory: bool) -> &'static str {
    if index == 0 {
        return "^(?:.+/)?";
    }
    if !last {
        return "(?:/.+)?";
    }
    if is_directory { DIRECTORY } else { "/" }
}

fn segment_glob(segment: &str) -> Result<Option<String>> {
    if segment == "*" {
        return Ok(Some("[^/]+".to_owned()));
    }
    let characters: Vec<_> = segment.chars().collect();
    let mut index = 0;
    let mut regex = String::new();
    while index < characters.len() {
        let Some(part) = glob_character(&characters, &mut index)? else {
            return Ok(None);
        };
        regex.push_str(&part);
    }
    Ok(Some(regex))
}

fn glob_character(characters: &[char], index: &mut usize) -> Result<Option<String>> {
    let character = characters[*index];
    *index += 1;
    match character {
        '\\' => Ok(Some(escaped_character(characters, index)?)),
        '*' => Ok(Some("[^/]*".to_owned())),
        '?' => Ok(Some("[^/]".to_owned())),
        '[' => Ok(bracket_expression(characters, index)),
        character => Ok(Some(regex::escape(&character.to_string()))),
    }
}

fn escaped_character(characters: &[char], index: &mut usize) -> Result<String> {
    let Some(character) = characters.get(*index) else {
        bail!("Escape character found with no next character to escape");
    };
    *index += 1;
    Ok(regex::escape(&character.to_string()))
}

fn bracket_expression(characters: &[char], index: &mut usize) -> Option<String> {
    let start = *index;
    let negated = characters.get(start).is_some_and(|c| matches!(c, '!' | '^'));
    let body_start = start + usize::from(negated);
    let search_start = body_start + usize::from(characters.get(body_start) == Some(&']'));
    let end = (search_start..characters.len()).find(|i| characters[*i] == ']')?;
    let body = bracket_body(&characters[body_start..end]);
    *index = end + 1;
    Some(format!("[{}{body}]", if negated { "^" } else { "" }))
}

fn bracket_body(characters: &[char]) -> String {
    let mut index = 0;
    let mut body = String::new();
    while index < characters.len() {
        body.push_str(&bracket_unit(characters, &mut index));
    }
    body
}

fn bracket_unit(characters: &[char], index: &mut usize) -> String {
    let first = regex::escape(&characters[*index].to_string());
    *index += 1;
    if characters.get(*index) == Some(&'-') && *index + 1 < characters.len() {
        let last = regex::escape(&characters[*index + 1].to_string());
        *index += 2;
        return format!("{first}-{last}");
    }
    first
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_matches_take_priority_over_directory_negations() -> Result<()> {
        let spec = IgnoreSpec::new("a/**\n!a/\n!a/b.py")?;
        assert_eq!(spec.check_file("a/"), Some(true));
        assert_eq!(spec.check_file("a/b.py"), Some(false));
        Ok(())
    }
    fn assert_class((pattern, members, negated): (&str, &str, bool)) -> Result<()> {
        let spec = IgnoreSpec::new(pattern)?;
        for character in (32..127).filter_map(char::from_u32).filter(|c| *c != '/') {
            let matches = spec.check_file(&format!("{character}.py")) == Some(true);
            assert_eq!(matches, members.contains(character) != negated, "{pattern}: {character:?}");
        }
        Ok(())
    }
    #[test]
    fn class_operators_match_pinned_pathspec_ascii_oracle() -> Result<()> {
        let cases = [
            ("[a-z--x].py", "-.0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz", false),
            ("[a-z&&x].py", "&abcdefghijklmnopqrstuvwxyz", false),
            ("[a-z~~x].py", "abcdefghijklmnopqrstuvwxyz~", false),
            ("[a-z||x].py", "abcdefghijklmnopqrstuvwxyz|", false),
            ("[^a-c].py", "abc", true),
            ("[!a-c].py", "abc", true),
            ("[[a-c].py", "[abc", false),
            ("[a-z--].py", "-abcdefghijklmnopqrstuvwxyz", false),
            ("[-a-c].py", "-abc", false),
            ("[a-c-].py", "-abc", false),
            ("[a^c].py", "^ac", false),
        ];
        for case in cases {
            assert_class(case)?;
        }
        Ok(())
    }
    #[test]
    fn invalid_patterns_and_literal_brackets() -> Result<()> {
        assert!(IgnoreSpec::new("!").is_err());
        assert!(IgnoreSpec::new("bad\\").is_err());
        assert_eq!(IgnoreSpec::new("[bad")?.check_file("[bad"), None);
        assert_eq!(IgnoreSpec::new("[[]a.py")?.check_file("[a.py"), Some(true));
        Ok(())
    }
}
