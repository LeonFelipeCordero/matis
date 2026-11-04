//! Explicit Rust inline/external module identities and direct path attributes.
#[path = "rust_unicode.rs"]
mod unicode;
use crate::metrics::{
    adapters::module_index::{ModuleIndex, ModuleScope},
    resolution_syntax::Syntax,
};
use crate::{
    domain::models::{ParsedFile, ParsedRepository, Symbol},
    syntax::{field, named_children, text},
};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
    sync::Arc,
};
use tree_sitter::Node;

#[derive(Clone)]
pub struct RustModules {
    pub index: ModuleIndex,
    pub syntax: BTreeMap<String, Syntax>,
    pub repository: ParsedRepository,
    files: Vec<Arc<ParsedFile>>,
    paths: BTreeMap<PathBuf, Vec<Arc<ParsedFile>>>,
}
impl RustModules {
    /// Index explicit inline/external Rust modules from retained files and matching `syntax`.
    /// Prefer `lib.rs`/`main.rs` roots; honor direct path attributes without guessing `cfg_attr`.
    /// Path normalization can query the filesystem; source contents come from the repository.
    pub fn new(repository: &ParsedRepository, syntax: &BTreeMap<String, Syntax>) -> Self {
        let mut result = Self {
            index: ModuleIndex::new(),
            syntax: syntax.clone(),
            repository: repository.clone(),
            files: repository
                .files
                .iter()
                .filter(|file| file.source.language == "rust")
                .map(|file| Arc::new(file.clone()))
                .collect(),
            paths: BTreeMap::new(),
        };
        for file in &result.files {
            result.paths.entry(resolve(&file.source.absolute_path)).or_default().push(Arc::clone(file));
        }
        for root in result.roots() {
            result.index.add_scopes(result.walk(&root));
        }
        result
    }
    fn roots(&self) -> Vec<Arc<ParsedFile>> {
        let standard: Vec<_> = self
            .files
            .iter()
            .filter(|f| {
                matches!(f.source.absolute_path.file_name().and_then(|n| n.to_str()), Some("lib.rs" | "main.rs"))
            })
            .cloned()
            .collect();
        if !standard.is_empty() {
            return standard;
        }
        let candidates: Vec<_> = self
            .files
            .iter()
            .filter(|f| {
                self.syntax
                    .get(&f.source.path)
                    .and_then(Syntax::root)
                    .is_some_and(|n| named_children(n).iter().any(|n| n.kind() == "mod_item"))
            })
            .cloned()
            .collect();
        let referenced: std::collections::BTreeSet<_> = candidates
            .iter()
            .flat_map(|f| {
                self.walk(f)
                    .into_iter()
                    .filter(|s| s.file.source.path != f.source.path)
                    .map(|s| s.file.source.path.clone())
            })
            .collect();
        candidates.into_iter().filter(|f| !referenced.contains(&f.source.path)).collect()
    }
    fn walk(&self, file: &Arc<ParsedFile>) -> Vec<ModuleScope> {
        let Some(body) = self.syntax.get(&file.source.path).and_then(Syntax::root) else {
            return vec![];
        };
        let scope = ModuleScope {
            file: file.clone(),
            identity: "crate".into(),
            body: self.syntax[&file.source.path].reference(body),
            span: (body.start_byte(), body.end_byte()),
            directory: file.source.absolute_path.parent().unwrap_or(Path::new("")).into(),
        };
        ModuleIndex::walk_scopes(scope, &self.syntax, |s| self.children(s))
    }
    fn children(&self, scope: &ModuleScope) -> Vec<ModuleScope> {
        scope
            .node(&self.syntax)
            .map(|body| {
                named_children(body)
                    .into_iter()
                    .filter(|n| n.kind() == "mod_item")
                    .flat_map(|n| self.child_scopes(scope, n))
                    .collect()
            })
            .unwrap_or_default()
    }
    fn child_scopes(&self, scope: &ModuleScope, node: Node<'_>) -> Vec<ModuleScope> {
        let source = &self.syntax[&scope.file.source.path].source;
        let name = text(field(node, &["name"]), source);
        let identity = format!("{}::{name}", scope.identity);
        let paths = path_attributes(node, source);
        let attribute_dir = if scope.node(&self.syntax).is_some_and(|n| n.kind() == "source_file") {
            scope.file.source.absolute_path.parent().unwrap_or(Path::new(""))
        } else {
            &scope.directory
        };
        if let Some(body) = field(node, &["body"]) {
            let directories = if paths.is_empty() {
                vec![scope.directory.join(&name)]
            } else {
                paths.iter().map(|p| attribute_dir.join(p)).collect()
            };
            return directories
                .into_iter()
                .map(|directory| ModuleScope {
                    file: scope.file.clone(),
                    identity: identity.clone(),
                    body: self.syntax[&scope.file.source.path].reference(body),
                    span: (body.start_byte(), body.end_byte()),
                    directory,
                })
                .collect();
        }
        let locations = if paths.is_empty() {
            vec![scope.directory.join(format!("{name}.rs")), scope.directory.join(&name).join("mod.rs")]
        } else {
            paths.iter().map(|p| attribute_dir.join(p)).collect()
        };
        locations
            .iter()
            .flat_map(|p| self.paths.get(&resolve(p)).into_iter().flatten())
            .filter_map(|file| {
                let root = self.syntax.get(&file.source.path)?.root()?;
                let path = &file.source.absolute_path;
                let directory = if path.file_name().is_some_and(|n| n == "mod.rs") {
                    path.parent().unwrap_or(Path::new("")).into()
                } else {
                    path.with_extension("")
                };
                Some(ModuleScope {
                    file: file.clone(),
                    identity: identity.clone(),
                    body: self.syntax[&file.source.path].reference(root),
                    span: (root.start_byte(), root.end_byte()),
                    directory,
                })
            })
            .collect()
    }
    fn identities(&self, source: &ParsedFile, name: &str, position: Option<usize>) -> Vec<String> {
        if name == "crate" || name.starts_with("crate::") {
            return vec![name.into()];
        }
        let mut result = Vec::new();
        for scope in self.index.scopes_at(source, position, &self.syntax) {
            if let Some(identity) = relative_identity(&scope.identity, name)
                && !result.contains(&identity)
            {
                result.push(identity);
            }
        }
        result
    }
    fn target(&self, source: &ParsedFile, name: &str, position: Option<usize>) -> Option<&ModuleScope> {
        self.index.unique_scope(&self.identities(source, name, position))
    }
    /// Return the unique module file/declaration key for `name` relative to `source` and byte `position`.
    /// No position uses file-root scopes; unresolved or ambiguous paths yield `None`.
    pub fn rust_key(&self, source: &ParsedFile, name: &str, position: Option<usize>) -> Option<String> {
        ModuleIndex::scope_key(self.target(source, name, position), &self.syntax)
    }
    /// Return the unique external module file for `name` in the source's lexical position.
    /// No position uses file roots; inline, missing, or ambiguous modules produce no files.
    pub fn rust_import(&self, source: &ParsedFile, name: &str, position: Option<usize>) -> Vec<ParsedFile> {
        self.target(source, name, position)
            .filter(|s| s.node(&self.syntax).is_some_and(|n| n.kind() == "source_file"))
            .map(|s| vec![s.file.as_ref().clone()])
            .unwrap_or_default()
    }
    /// Return direct declarations of the unique module named relative to `source` and byte `position`.
    /// No position uses file roots; missing or ambiguous modules produce no symbols.
    pub fn rust_symbols(&self, source: &ParsedFile, name: &str, position: Option<usize>) -> Vec<Symbol> {
        ModuleIndex::scope_symbols(self.target(source, name, position), &self.syntax)
    }
    /// Return a unique file-root module for absolute identity `name`; inline or ambiguous scopes yield none.
    pub fn rust_files(&self, name: &str) -> Vec<ParsedFile> {
        self.index.scope_files(name, &self.syntax)
    }
}
fn relative_identity(identity: &str, name: &str) -> Option<String> {
    let mut parts: Vec<_> = identity.split("::").collect();
    let mut suffix = name.split("::").peekable();
    if suffix.peek() == Some(&"self") {
        suffix.next();
    }
    while suffix.peek() == Some(&"super") {
        parts.pop();
        suffix.next();
    }
    if parts.is_empty() {
        return None;
    }
    parts.extend(suffix);
    Some(parts.join("::"))
}
fn resolve(path: &Path) -> PathBuf {
    let mut resolved = if path.is_absolute() { PathBuf::new() } else { std::env::current_dir().unwrap_or_default() };
    for component in path.components() {
        match component {
            Component::ParentDir => {
                resolved.pop();
            }
            Component::CurDir => {}
            other => {
                resolved.push(other.as_os_str());
                if let Ok(actual) = resolved.canonicalize() {
                    resolved = actual;
                }
            }
        }
    }
    resolved
}

/// Decode direct path attributes preceding `node`, nearest first, using matching `source` bytes.
/// Skip comments and unrelated attributes; do not infer paths from `cfg_attr`.
/// Raw strings are retained and quoted strings use Python-compatible literal escapes.
pub fn path_attributes(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut result = Vec::new();
    let mut sibling = node.prev_named_sibling();
    while let Some(previous) = sibling {
        if !["attribute_item", "line_comment", "block_comment"].contains(&previous.kind()) {
            break;
        }
        if previous.kind() == "attribute_item"
            && let Some(value) = path_attribute(previous, source)
        {
            result.push(value);
        }
        sibling = previous.prev_named_sibling();
    }
    result
}
fn path_attribute(node: Node<'_>, source: &[u8]) -> Option<String> {
    let attribute = named_children(node).first().copied()?;
    let value = field(attribute, &["value"])?;
    if text(named_children(attribute).first().copied(), source) != "path" {
        return None;
    }
    if value.kind() == "raw_string_literal" {
        return Some(text(named_children(value).into_iter().find(|c| c.kind() == "string_content"), source));
    }
    decode_python_string(&text(Some(value), source))
}
// The reference uses ast.literal_eval, not Rust string escape rules.
fn decode_python_string(value: &str) -> Option<String> {
    let quote = value.chars().next()?;
    if !['\'', '"'].contains(&quote) || !value.ends_with(quote) || value.len() < 2 {
        return None;
    }
    let mut chars = value[1..value.len() - 1].chars().peekable();
    let mut result = String::new();
    while let Some(c) = chars.next() {
        if c == '\\' {
            push_escape(&mut result, &mut chars)?;
        } else {
            push_literal(&mut result, c, quote)?;
        }
    }
    Some(result)
}
type LiteralChars<'a> = std::iter::Peekable<std::str::Chars<'a>>;
fn push_literal(result: &mut String, c: char, quote: char) -> Option<()> {
    if c == quote || ['\n', '\r'].contains(&c) {
        return None;
    }
    result.push(c);
    Some(())
}
fn push_escape(result: &mut String, chars: &mut LiteralChars<'_>) -> Option<()> {
    let escaped = chars.next()?;
    const SIMPLE: &[(char, char)] = &[
        ('\\', '\\'),
        ('\'', '\''),
        ('"', '"'),
        ('n', '\n'),
        ('r', '\r'),
        ('t', '\t'),
        ('a', '\x07'),
        ('b', '\x08'),
        ('f', '\x0c'),
        ('v', '\x0b'),
    ];
    if let Some((_, value)) = SIMPLE.iter().find(|(key, _)| *key == escaped) {
        result.push(*value);
        return Some(());
    }
    match escaped {
        '\n' => {}
        '\r' => {
            chars.next_if_eq(&'\n');
        }
        'N' => result.push(named_escape(chars)?),
        'x' | 'u' | 'U' => result.push(hex_escape(escaped, chars)?),
        '0'..='7' => result.push(octal_escape(escaped, chars)?),
        other => {
            result.push('\\');
            result.push(other);
        }
    }
    Some(())
}
fn named_escape(chars: &mut LiteralChars<'_>) -> Option<char> {
    if chars.next()? != '{' {
        return None;
    }
    let mut name = String::new();
    loop {
        match chars.next()? {
            '}' => break,
            value => name.push(value),
        }
    }
    unicode::character(&name)
}
fn hex_escape(escaped: char, chars: &mut LiteralChars<'_>) -> Option<char> {
    let count = match escaped {
        'x' => 2,
        'u' => 4,
        _ => 8,
    };
    let digits: String = (0..count).map(|_| chars.next()).collect::<Option<_>>()?;
    char::from_u32(u32::from_str_radix(&digits, 16).ok()?)
}
fn octal_escape(escaped: char, chars: &mut LiteralChars<'_>) -> Option<char> {
    let mut digits = escaped.to_string();
    for _ in 0..2 {
        digits.extend(chars.next_if(|c| matches!(c, '0'..='7')));
    }
    char::from_u32(u32::from_str_radix(&digits, 8).ok()?)
}
