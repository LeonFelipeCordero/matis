use crate::domain::models::{ParsedFile, Symbol, file_key};
use crate::metrics::resolution_contract::ImportRequest;
use regex::Regex;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

static QUOTED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"['\"]([^'\"]+)['\"]"#).unwrap());
static INCLUDE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"#[\s\x1c-\x1f]*include[\s\x1c-\x1f]*[<\"]([^>\"]+)[>\"]"#).unwrap());
static IMPORT_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[^\p{L}\p{N}_])from[\s\x1c-\x1f]+([\p{L}\p{N}_.]+)|(?:^|[^\p{L}\p{N}_])(?:import[\s\x1c-\x1f]+(?:static[\s\x1c-\x1f]+)?|(?:use|alias|require|mod)[\s\x1c-\x1f]+)([\p{L}\p{N}_.:/]+)").unwrap()
});
static FROM_PREFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\s\x1c-\x1f]*from(?:$|[^\p{L}\p{N}_])").unwrap());
static ALIAS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|[^\p{L}\p{N}_])as[\s\x1c-\x1f]+([A-Za-z_$][\p{L}\p{N}_$]*)").unwrap());
static PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^[\s\x1c-\x1f]*([A-Za-z_$][\p{L}\p{N}_$]*)[\s\x1c-\x1f]+['\"]"#).unwrap());
static BLANK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^[\s\x1c-\x1f]*(?:_|\.)[\s\x1c-\x1f]+['\"]"#).unwrap());
static DEFAULT_IMPORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[\s\x1c-\x1f]*import[\s\x1c-\x1f]+[\p{L}\p{N}_]+[\s\x1c-\x1f]+from(?:$|[^\p{L}\p{N}_])").unwrap()
});

/// Return declarations directly parented by `file`, preserving symbol order.
pub fn top_symbols(file: &ParsedFile) -> Vec<Symbol> {
    file.symbols.iter().filter(|s| s.parent_key == file_key(&file.source.path)).cloned().collect()
}
/// Return the last nonempty quoted substring of `source`, or `None` when absent.
pub fn quoted_path(source: &str) -> Option<String> {
    QUOTED.captures_iter(source).last().map(|m| m[1].to_owned())
}
/// Extract an include path, last quoted path, or directive name, in that order.
/// Fall back to trimmed `source` when no supported pattern matches.
pub fn import_path(source: &str) -> String {
    if let Some(c) = INCLUDE.captures(source) {
        return c[1].to_owned();
    }
    if let Some(path) = quoted_path(source) {
        return path;
    }
    IMPORT_PATH
        .captures(source)
        .and_then(|m| m.get(1).or_else(|| m.get(2)))
        .map(|m| m.as_str().to_owned())
        .unwrap_or_else(|| source.trim().to_owned())
}
/// Return an absolute normalized path, resolving existing prefixes and symlinks where possible.
/// Use the current directory for relative paths and normalize missing components lexically.
/// Filesystem lookup failures are ignored, as with non-strict path resolution.
pub fn resolve_path(path: &Path) -> PathBuf {
    let path =
        if path.is_absolute() { path.to_owned() } else { std::env::current_dir().unwrap_or_default().join(path) };
    if let Ok(resolved) = path.canonicalize() {
        return resolved;
    }
    let mut result = PathBuf::new();
    for part in path.components() {
        resolve_component(&mut result, part);
    }
    result
}
fn resolve_component(result: &mut PathBuf, part: Component<'_>) {
    match part {
        Component::CurDir => {}
        Component::ParentDir => {
            result.pop();
        }
        part => {
            result.push(part.as_os_str());
            if let Ok(resolved) = result.canonicalize() {
                *result = resolved;
            }
        }
    }
}
fn parent(path: &str) -> String {
    Path::new(path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".into())
}
/// Match `request.path` relative to its importing file against retained `files`.
/// Accept one direct match with optional extension omission; reject multiple direct matches.
/// Otherwise return directory files, limited to `index` stems when `index_only` is true.
/// Path normalization may query the filesystem but does not read file contents.
pub fn relative_targets(request: &ImportRequest<'_>, files: &[ParsedFile], index_only: bool) -> Vec<ParsedFile> {
    let base = resolve_path(&request.file.source.absolute_path.parent().unwrap_or(Path::new(".")).join(&request.path));
    let matches: Vec<_> = files
        .iter()
        .filter(|file| {
            let path = resolve_path(&file.source.absolute_path);
            path == base || path.with_extension("") == base
        })
        .cloned()
        .collect();
    if !matches.is_empty() {
        return if matches.len() == 1 { matches } else { vec![] };
    }
    files
        .iter()
        .filter(|f| {
            resolve_path(&f.source.absolute_path).parent() == Some(base.as_path())
                && (!index_only || f.source.absolute_path.file_stem().is_some_and(|s| s == "index"))
        })
        .cloned()
        .collect()
}
/// Match `name` to one exact file, then one package directory; ambiguous matches yield no files.
/// Normalize `::` and dotted names to paths and allow directory suffix matches.
pub fn package_targets(name: &str, files: &[ParsedFile]) -> Vec<ParsedFile> {
    let path = name.replace("::", "/");
    let path = if path.contains('/') { path } else { path.replace('.', "/") };
    let path = path.trim_matches('/');
    let exact: Vec<_> = files
        .iter()
        .filter(|f| f.source.path == path || Path::new(&f.source.path).with_extension("").to_string_lossy() == path)
        .cloned()
        .collect();
    if !exact.is_empty() {
        return if exact.len() == 1 { exact } else { vec![] };
    }
    let directories: BTreeSet<_> = files
        .iter()
        .map(|f| parent(&f.source.path))
        .filter(|p| p == path || p.ends_with(&format!("/{path}")))
        .collect();
    if directories.len() != 1 {
        return vec![];
    }
    files.iter().filter(|f| directories.contains(&parent(&f.source.path))).cloned().collect()
}
/// Return a sole file key or a shared-directory module key for `files`.
/// Empty or mixed-directory targets yield `None`.
pub fn path_target_key(files: &[ParsedFile]) -> Option<String> {
    if files.len() == 1 {
        return Some(file_key(&files[0].source.path));
    }
    let parents: BTreeSet<_> = files.iter().map(|f| parent(&f.source.path)).collect();
    (parents.len() == 1).then(|| format!("module:{}", parents.first().unwrap()))
}
/// Infer a namespace alias or final `path` component from import `source`.
/// Reject brace/from/default imports and recognized blank/dot forms; explicit prefixes win first.
pub fn namespace_binding(source: &str, path: &str) -> Option<String> {
    if source.contains('{') || FROM_PREFIX.is_match(source) {
        return None;
    }
    if let Some(m) = ALIAS.captures(source) {
        return Some(m[1].to_owned());
    }
    if let Some(m) = PREFIX.captures(source)
        && !["import", "require"].contains(&&m[1])
    {
        return Some(m[1].to_owned());
    }
    if BLANK.is_match(source) || DEFAULT_IMPORT.is_match(source) {
        return None;
    }
    Some(path.replace("::", "/").replace('.', "/").trim_end_matches('/').rsplit('/').next().unwrap_or("").to_owned())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn file(path: &str) -> ParsedFile {
        ParsedFile {
            source: crate::domain::models::SourceFile {
                path: path.into(),
                absolute_path: Path::new("/project").join(path),
                language: "javascript".into(),
                source: vec![],
            },
            content_hash: String::new(),
            parse_error: None,
            symbols: vec![],
            imports: vec![],
            calls: vec![],
            inheritances: vec![],
        }
    }
    #[test]
    fn resolves_only_unique_direct_files_and_directory_packages() {
        let source = file("main.js");
        let request = ImportRequest { file: &source, path: "./a".into(), position: None, source: String::new() };
        assert_eq!(relative_targets(&request, &[file("a.js")], true)[0].source.path, "a.js");
        assert!(relative_targets(&request, &[file("a.js"), file("a.ts")], true).is_empty());
        let files = vec![file("a/index.js"), file("a/other.js")];
        assert_eq!(relative_targets(&request, &files, true).len(), 1);
        assert_eq!(relative_targets(&request, &files, false).len(), 2);
        assert_eq!(path_target_key(&files), Some("module:a".into()));
        assert_eq!(package_targets("a", &files).len(), 2);
        assert!(package_targets("a", &[file("x/a/f.js"), file("y/a/f.js")]).is_empty());
    }
    #[test]
    fn import_word_classes_match_python_unicode_categories() {
        assert_eq!(import_path("import a\u{301}.b"), "a");
        assert_eq!(import_path("x\u{301}import foo"), "foo");
        assert_eq!(import_path("import a².b"), "a².b");
        assert_eq!(import_path("import\u{1f}a"), "a");
        assert_eq!(namespace_binding("as x\u{301}", "a"), Some("x".into()));
    }
    #[test]
    fn extracts_paths_and_namespaces() {
        assert_eq!(import_path("#include <sys/a.h>"), "sys/a.h");
        assert_eq!(quoted_path("import x from 'a'; 'b'"), Some("b".into()));
        assert_eq!(namespace_binding("import * as x from 'a'", "a"), Some("x".into()));
        assert_eq!(namespace_binding("from a import b", "a"), None);
        assert_eq!(namespace_binding("_ \"a\"", "a"), Some("_".into()));
        assert_eq!(namespace_binding(". \"a\"", "a"), None);
        assert_eq!(path_target_key(&[]), None);
    }
}
