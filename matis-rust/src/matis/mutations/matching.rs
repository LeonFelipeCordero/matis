//! Rooted POSIX globs, with Python fnmatchcase semantics within each component.
use super::config::{MutationConfig, MutationTarget};
use indexmap::IndexMap;

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
/// Match a whole POSIX path against a rooted, case-sensitive source glob.
///
/// `**` matches zero or more components. Within components, use Python
/// `fnmatchcase`-style `*`, `?`, and bracket classes, including dot names.
/// Split on `/` without normalizing paths or accessing the filesystem.
pub fn matches(path: &str, pattern: &str) -> bool {
    let parts: Vec<_> = path.split('/').collect();
    let patterns: Vec<_> = pattern.split('/').collect();
    let memo = vec![vec![None; patterns.len() + 1]; parts.len() + 1];
    RootMatch { parts, patterns, memo }.at(0, 0)
}
struct RootMatch<'a> {
    parts: Vec<&'a str>,
    patterns: Vec<&'a str>,
    memo: Vec<Vec<Option<bool>>>,
}
impl RootMatch<'_> {
    fn at(&mut self, i: usize, j: usize) -> bool {
        if let Some(result) = self.memo[i][j] {
            return result;
        }
        let result = match self.patterns.get(j) {
            None => i == self.parts.len(),
            Some(&"**") => self.at(i, j + 1) || i < self.parts.len() && self.at(i + 1, j),
            Some(_) => self.part(i, j),
        };
        self.memo[i][j] = Some(result);
        result
    }
    fn part(&mut self, i: usize, j: usize) -> bool {
        if i == self.parts.len() {
            return false;
        }
        component(self.parts[i], self.patterns[j]) && self.at(i + 1, j + 1)
    }
}
fn component(text: &str, pattern: &str) -> bool {
    let text: Vec<_> = text.chars().collect();
    let pattern: Vec<_> = pattern.chars().collect();
    let memo = vec![vec![None; pattern.len() + 1]; text.len() + 1];
    ComponentMatch { text, pattern, memo }.at(0, 0)
}
struct ComponentMatch {
    text: Vec<char>,
    pattern: Vec<char>,
    memo: Vec<Vec<Option<bool>>>,
}
impl ComponentMatch {
    fn at(&mut self, i: usize, j: usize) -> bool {
        if let Some(value) = self.memo[i][j] {
            return value;
        }
        let result = self.step(i, j);
        self.memo[i][j] = Some(result);
        result
    }
    fn step(&mut self, i: usize, j: usize) -> bool {
        match self.pattern.get(j) {
            None => i == self.text.len(),
            Some('*') => self.star(i, j),
            Some(_) if i == self.text.len() => false,
            Some('?') => self.at(i + 1, j + 1),
            Some('[') => self.bracket(i, j),
            Some(ch) => self.text[i] == *ch && self.at(i + 1, j + 1),
        }
    }
    fn star(&mut self, i: usize, j: usize) -> bool {
        self.at(i, j + 1) || i < self.text.len() && self.at(i + 1, j)
    }
    fn bracket(&mut self, i: usize, j: usize) -> bool {
        if let Some(end) = class_end(&self.pattern, j) {
            return in_class(self.text[i], &self.pattern[j + 1..end]) && self.at(i + 1, end + 1);
        }
        self.text[i] == '[' && self.at(i + 1, j + 1)
    }
}
fn class_end(pattern: &[char], start: usize) -> Option<usize> {
    let mut index = start + 1;
    if pattern.get(index) == Some(&'!') {
        index += 1;
    }
    if pattern.get(index) == Some(&']') {
        index += 1;
    }
    (index..pattern.len()).find(|&index| pattern[index] == ']')
}
fn in_class(ch: char, class: &[char]) -> bool {
    let tokens = class_tokens(class);
    let negate = tokens.first() == Some(&('!', true));
    let tokens = if negate { &tokens[1..] } else { &tokens };
    let mut found = false;
    let mut index = 0;
    while index < tokens.len() {
        if index + 2 < tokens.len() && !tokens[index + 1].1 {
            found |= tokens[index].0 <= ch && ch <= tokens[index + 2].0;
            index += 3;
        } else {
            found |= ch == tokens[index].0;
            index += 1;
        }
    }
    found != negate
}

// Python removes descending ranges before recognizing a leading negation marker.
fn class_tokens(class: &[char]) -> Vec<(char, bool)> {
    if !class.contains(&'-') {
        return class.iter().map(|&ch| (ch, true)).collect();
    }
    let mut chunks = class_chunks(class);
    for index in (1..chunks.len()).rev() {
        if chunks[index - 1].last() > chunks[index].first() {
            let next = chunks.remove(index);
            chunks[index - 1].pop();
            chunks[index - 1].extend(&next[1..]);
        }
    }
    let mut tokens = Vec::new();
    for (index, chunk) in chunks.iter().enumerate() {
        if index > 0 {
            tokens.push(('-', false));
        }
        tokens.extend(chunk.iter().map(|&ch| (ch, true)));
    }
    tokens
}

fn class_chunks(class: &[char]) -> Vec<Vec<char>> {
    let mut chunks = Vec::new();
    let (mut start, mut search) = (0, if class.first() == Some(&'!') { 2 } else { 1 });
    while let Some(index) = (search..class.len()).find(|&index| class[index] == '-') {
        chunks.push(class[start..index].to_vec());
        start = index + 1;
        search = index + 3;
    }
    if start < class.len() {
        chunks.push(class[start..].to_vec());
    } else {
        chunks.last_mut().unwrap().push('-');
    }
    chunks
}
