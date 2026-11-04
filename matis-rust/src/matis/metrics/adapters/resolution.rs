//! Owned import, module and callable identity indexes.
pub use super::resolution_paths::{
    import_path, namespace_binding, package_targets, path_target_key, relative_targets, top_symbols,
};
pub use super::resolution_policy::ResolutionAdapter;
use super::{callable_selection::Clause, elixir, python, rust};
use crate::domain::models::{Call, Import, ParsedFile, ParsedRepository, Symbol};
use crate::metrics::resolution_contract::{ImportRequest, ResolutionRequest};
use crate::metrics::resolution_syntax::{NodeRef, Syntax};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Binding {
    pub name: String,
    pub targets: Vec<Symbol>,
    pub namespace: bool,
    pub start: usize,
    pub end: usize,
    pub members: Option<BTreeMap<String, Vec<Symbol>>>,
    pub scope: Option<NodeRef>,
}
impl Binding {
    /// Return alias-map members named `name`, or filter targets by name when no alias map exists.
    pub fn member(&self, name: &str) -> Vec<Symbol> {
        self.members
            .as_ref()
            .map(|m| m.get(name).cloned().unwrap_or_default())
            .unwrap_or_else(|| self.targets.iter().filter(|s| s.name == name).cloned().collect())
    }
}
#[derive(Clone)]
pub struct Modules {
    pub repository: ParsedRepository,
    pub syntax: BTreeMap<String, Syntax>,
    pub python: BTreeMap<String, Vec<ParsedFile>>,
    pub rust: Option<rust::RustModules>,
    pub elixir: Option<elixir::ElixirModules>,
    pub packages: BTreeMap<String, String>,
    declarations: BTreeMap<(String, String), Vec<Symbol>>,
}
impl Modules {
    /// Build language module, package, and declaration indexes from repository facts and syntax.
    pub fn new(repository: &ParsedRepository, syntax: &BTreeMap<String, Syntax>) -> Self {
        let has = |language: &str| repository.files.iter().any(|f| f.source.language == language);
        let mut result = Self {
            repository: repository.clone(),
            syntax: syntax.clone(),
            packages: package_index(repository, syntax),
            declarations: BTreeMap::new(),
            python: if has("python") { python::module_index(repository, syntax) } else { BTreeMap::new() },
            rust: has("rust").then(|| rust::RustModules::new(repository, syntax)),
            elixir: has("elixir").then(|| elixir::ElixirModules::new(repository, syntax)),
        };
        for file in &repository.files {
            result.index_file(file);
        }
        result
    }
    fn index_file(&mut self, file: &ParsedFile) {
        let syntax = &self.syntax[&file.source.path];
        let package = self.packages.get(&file.source.path);
        for symbol in &file.symbols {
            let identity = package
                .map(|p| super::java::identity_in_package(symbol, p))
                .unwrap_or_else(|| syntax.adapter.module_identity(file, symbol, syntax));
            self.declarations.entry((file.source.language.clone(), identity)).or_default().push(symbol.clone());
        }
    }
    /// Return declarations matching `source`'s language and exact module-qualified `name`.
    /// Preserve repository order and ambiguity.
    pub fn declared(&self, source: &ParsedFile, name: &str) -> Vec<Symbol> {
        self.declarations.get(&(source.source.language.clone(), name.to_owned())).cloned().unwrap_or_default()
    }
    /// Return named declarations implicitly visible in `source`'s package under its language policy.
    pub fn same_package(&self, source: &ParsedFile, name: &str) -> Vec<Symbol> {
        self.syntax[&source.source.path].adapter.same_package(self, source, name)
    }
}
fn package_index(repository: &ParsedRepository, syntax: &BTreeMap<String, Syntax>) -> BTreeMap<String, String> {
    repository
        .files
        .iter()
        .filter(|file| syntax[&file.source.path].adapter.jvm())
        .map(|file| (file.source.path.clone(), super::java::package(&syntax[&file.source.path])))
        .collect()
}
#[derive(Clone)]
pub struct CallableIdentity {
    pub syntax: BTreeMap<String, Syntax>,
    pub clauses: BTreeMap<String, Clause>,
}
impl CallableIdentity {
    /// Retain shared syntax and index Elixir clause arity/privacy metadata by symbol key.
    pub fn new(repository: &ParsedRepository, syntax: &BTreeMap<String, Syntax>) -> Self {
        Self { syntax: syntax.clone(), clauses: elixir::clauses(repository, syntax) }
    }
    /// Apply `file`'s callable-identity rules to `matches` at `call`; ambiguity can yield no targets.
    pub fn select(&self, file: &ParsedFile, call: &Call, matches: Vec<Symbol>) -> Vec<Symbol> {
        let request =
            ResolutionRequest { file, call, owner: None, name: call.name.clone(), qualifier: call.qualifier.clone() };
        self.syntax[&file.source.path].adapter.select(self, &request, matches)
    }
}
type ImportPathCache = BTreeMap<(String, String), Vec<ParsedFile>>;

#[derive(Clone)]
pub struct ImportBindings {
    pub repository: ParsedRepository,
    pub syntax: BTreeMap<String, Syntax>,
    pub modules: Modules,
    pub bindings: BTreeMap<String, Vec<Binding>>,
    paths: Arc<Mutex<ImportPathCache>>,
}
impl ImportBindings {
    /// Build scoped import and implicit module bindings from retained facts, syntax, and module indexes.
    /// Initialize a shared path-lookup cache; language path policies may query filesystem metadata.
    pub fn new(repository: &ParsedRepository, syntax: &BTreeMap<String, Syntax>, modules: &Modules) -> Self {
        let mut result = Self {
            repository: repository.clone(),
            syntax: syntax.clone(),
            modules: modules.clone(),
            bindings: BTreeMap::new(),
            paths: Arc::new(Mutex::new(BTreeMap::new())),
        };
        for file in &repository.files {
            result.bindings.insert(file.source.path.clone(), result.file_bindings(file));
        }
        result
    }
    /// Resolve `path` within `file`'s language family and cache results by importer and path.
    /// Return retained file copies; path policies can query filesystem metadata.
    pub fn path_files(&self, file: &ParsedFile, path: &str) -> Vec<ParsedFile> {
        let key = (file.source.path.clone(), path.to_owned());
        if let Some(files) = self.paths.lock().unwrap().get(&key) {
            return files.clone();
        }
        let adapter = &self.syntax[&file.source.path].adapter;
        let family = if adapter.family().is_empty() { &file.source.language } else { adapter.family() };
        let files: Vec<_> = self
            .repository
            .files
            .iter()
            .filter(|f| {
                let candidate = &self.syntax[&f.source.path].adapter;
                (if candidate.family().is_empty() { &f.source.language } else { candidate.family() }) == family
            })
            .cloned()
            .collect();
        let result = adapter.paths(file, path, &files);
        self.paths.lock().unwrap().insert(key, result.clone());
        result
    }
    /// Resolve import files with language-specific module rules at optional byte `position`.
    pub fn files(&self, file: &ParsedFile, path: &str, position: Option<usize>) -> Vec<ParsedFile> {
        let request = ImportRequest { file, path: path.into(), position, source: String::new() };
        self.syntax[&file.source.path].adapter.import_files(self, &request)
    }
    /// Find imported symbols named `name` using the request file's language policy.
    pub fn members(&self, request: &ImportRequest<'_>, name: &str) -> Vec<Symbol> {
        self.syntax[&request.file.source.path].adapter.members(self, request, name)
    }
    /// Alias for [`Self::members`] used by import adapter helpers.
    pub fn _members(&self, request: &ImportRequest<'_>, name: &str) -> Vec<Symbol> {
        self.members(request, name)
    }
    fn file_bindings(&self, file: &ParsedFile) -> Vec<Binding> {
        let length = self.syntax[&file.source.path].source.len();
        let mut result: Vec<_> =
            file.imports.iter().flat_map(|item| self.directive_bindings(file, item, length)).collect();
        result.extend(self.syntax[&file.source.path].adapter.module_bindings(self, file, length));
        result
    }
    fn directive_bindings(&self, file: &ParsedFile, item: &Import, length: usize) -> Vec<Binding> {
        let line = item.line;
        let source = item.source.as_str();
        let syntax = &self.syntax[&file.source.path];
        let adapter = &syntax.adapter;
        let directive = syntax.import_node(line, Some(source));
        if adapter.require_import_node() && directive.is_none() {
            return vec![];
        }
        let scope = syntax.import_scope(line, Some(source));
        let (start, end) = scope.map(|n| (n.start_byte(), n.end_byte())).unwrap_or((0, length));
        let request = ImportRequest {
            file,
            path: adapter.import_path(source),
            position: directive.map(|n| n.start_byte()),
            source: source.into(),
        };
        adapter
            .import_item(self, &request)
            .into_iter()
            .map(|(name, targets, namespace)| {
                let members = if namespace { adapter.namespace_members(self, &request, &targets) } else { None };
                Binding { name, targets, namespace, start, end, members, scope: scope.map(|n| syntax.reference(n)) }
            })
            .collect()
    }
    /// Return the narrowest visible import binding for `name` at byte `position` in `file`.
    /// No binding yields `None`; equally narrow bindings yield an empty sentinel that blocks fallback.
    pub fn get(&self, file: &ParsedFile, name: &str, position: usize) -> Option<Binding> {
        let syntax = &self.syntax[&file.source.path];
        let matches: Vec<_> = self
            .bindings
            .get(&file.source.path)?
            .iter()
            .filter(|b| {
                b.name == name
                    && b.start <= position
                    && position < b.end
                    && b.scope.as_ref().is_none_or(|reference| {
                        syntax.dereference(reference).is_some_and(|n| syntax.visible(n, position))
                    })
            })
            .collect();
        let width = matches.iter().map(|b| b.end - b.start).min()?;
        let matches: Vec<_> = matches.into_iter().filter(|b| b.end - b.start == width).collect();
        Some(if matches.len() == 1 {
            matches[0].clone()
        } else {
            Binding {
                name: name.into(),
                targets: vec![],
                namespace: false,
                start: 0,
                end: 0,
                members: None,
                scope: None,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::SourceFile;
    fn file(path: &str) -> ParsedFile {
        ParsedFile {
            source: SourceFile {
                path: path.into(),
                absolute_path: std::path::Path::new("/project").join(path),
                language: "javascript".into(),
                source: b"x();".to_vec(),
            },
            content_hash: String::new(),
            parse_error: None,
            symbols: vec![],
            imports: vec![],
            calls: vec![],
            inheritances: vec![],
        }
    }
    fn context() -> (ParsedFile, ImportBindings) {
        let source = file("main.js");
        let repository = ParsedRepository { files: vec![source.clone(), file("lib.js")] };
        let syntax = repository
            .files
            .iter()
            .map(|f| (f.source.path.clone(), Syntax::new(f, super::super::adapter_for_language(&f.source.language))))
            .collect();
        let modules = Modules::new(&repository, &syntax);
        (source, ImportBindings::new(&repository, &syntax, &modules))
    }
    #[test]
    fn declaration_index_preserves_language_ambiguity_and_repository_order() {
        let parse = |path: &str, language: &str| {
            crate::parsing::parser::parse_file(SourceFile {
                path: path.into(),
                absolute_path: std::path::Path::new("/project").join(path),
                language: language.into(),
                source: b"package p; class A {}".to_vec(),
            })
            .unwrap()
        };
        let first = parse("one/A.java", "java");
        let second = parse("two/A.java", "java");
        let repository = ParsedRepository { files: vec![second.clone(), first.clone()] };
        let syntax = repository
            .files
            .iter()
            .map(|f| (f.source.path.clone(), Syntax::new(f, super::super::adapter_for_language(&f.source.language))))
            .collect();
        let mut modules = Modules::new(&repository, &syntax);
        let matches = modules.declared(&first, "p.A");
        assert_eq!(matches.iter().map(|s| s.file_path.as_str()).collect::<Vec<_>>(), ["two/A.java", "one/A.java"]);
        modules.repository.files.clear();
        assert_eq!(modules.declared(&first, "p.A").len(), 2);
        assert_eq!(modules.same_package(&first, "A").len(), 2);
        let mut foreign = first;
        foreign.source.language = "kotlin".into();
        assert!(modules.declared(&foreign, "p.A").is_empty());
    }
    #[test]
    fn path_cache_is_keyed_by_importer_and_directive_path() {
        let (file, mut context) = context();
        assert_eq!(context.path_files(&file, "./lib").len(), 1);
        context.repository.files.clear();
        assert_eq!(context.path_files(&file, "./lib").len(), 1);
        assert!(context.path_files(&file, "./other").is_empty());
        assert_eq!(context.paths.lock().unwrap().len(), 2);
    }
    #[test]
    fn narrow_bindings_win_and_equal_width_matches_block_fallback() {
        let (file, mut context) = context();
        let binding = |start, end| Binding {
            name: "x".into(),
            targets: vec![],
            namespace: true,
            start,
            end,
            members: None,
            scope: None,
        };
        context.bindings.insert(file.source.path.clone(), vec![binding(0, 4), binding(1, 3)]);
        assert_eq!(context.get(&file, "x", 2).unwrap().start, 1);
        context.bindings.get_mut(&file.source.path).unwrap().push(binding(1, 3));
        let blocked = context.get(&file, "x", 2).unwrap();
        assert_eq!(blocked.end, 0);
        assert!(!blocked.namespace);
        assert!(context.get(&file, "x", 4).is_none());
    }
}
