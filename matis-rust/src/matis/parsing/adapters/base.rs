use super::{elixir, go, java, javascript, kotlin, python, rust, typescript};
use crate::parsing::{
    discovery::require_supported_language,
    syntax::{self, CallSite, CallTarget, CallVisitor, DeclarationCarrier, DeclarationHeader, InheritancePolicy},
};
use crate::syntax::walk;
use anyhow::Result;
use tree_sitter::Node;

#[derive(Clone, Copy)]
pub struct LanguageAdapter<'a> {
    pub language: &'a str,
}

/// Select extraction policies for a Rust-importer language ID.
///
/// # Errors
/// Returns an error if the language ID is unsupported.
pub fn get_adapter(language: &str) -> Result<LanguageAdapter<'_>> {
    require_supported_language(language)?;
    Ok(LanguageAdapter { language })
}

impl LanguageAdapter<'_> {
    /// Test whether this language has dedicated extraction hooks rather than fallback policies.
    pub fn is_dedicated(self) -> bool {
        matches!(
            self.language,
            "javascript" | "jsx" | "typescript" | "tsx" | "java" | "kotlin" | "go" | "rust" | "elixir" | "python"
        )
    }
    /// Return supplemental declaration query text, or an empty string for fallback languages.
    /// Bundled tags are added separately by structure extraction.
    pub fn declaration_query(self) -> &'static str {
        const QUERIES: &[(&[&str], &str)] = &[
            (&["javascript", "jsx"], javascript::QUERY),
            (&["typescript", "tsx"], typescript::QUERY),
            (&["java"], java::QUERY),
            (&["kotlin"], kotlin::QUERY),
            (&["go"], go::QUERY),
            (&["rust"], rust::QUERY),
            (&["elixir"], elixir::QUERY),
            (&["python"], python::QUERY),
        ];
        QUERIES
            .iter()
            .find(|(languages, _)| languages.contains(&self.language))
            .map(|(_, query)| *query)
            .unwrap_or_default()
    }
    /// Return syntax node kinds treated as callback carriers, or an empty slice for fallback languages.
    pub fn callback_types(self) -> &'static [&'static str] {
        match self.language {
            "javascript" | "jsx" | "typescript" | "tsx" => javascript::CALLBACK_TYPES,
            "java" => java::CALLBACK_TYPES,
            "kotlin" => kotlin::CALLBACK_TYPES,
            "go" => go::CALLBACK_TYPES,
            "rust" => rust::CALLBACK_TYPES,
            "elixir" => elixir::CALLBACK_TYPES,
            "python" => python::CALLBACK_TYPES,
            _ => &[],
        }
    }
    /// Normalize a declaration's carrier, name, kind, and decorators using matching source bytes.
    /// Return `None` for TypeScript property signatures without a function type.
    pub fn normalize_declaration<'tree>(self, node: Node<'tree>, source: &[u8]) -> Option<DeclarationCarrier<'tree>> {
        if matches!(self.language, "javascript" | "jsx" | "typescript" | "tsx") {
            return self.script_declaration(node, source);
        }
        match self.language {
            "java" => Some(java::declaration(node, source)),
            "kotlin" => Some(kotlin::declaration(node, source)),
            "go" => Some(go::declaration(node, source)),
            "rust" => Some(rust::declaration(node, source)),
            "elixir" => Some(elixir::declaration(node, source)),
            "python" => Some(python::declaration(node, source)),
            _ => Some(syntax::named_carrier(node, source)),
        }
    }
    fn script_declaration<'tree>(self, node: Node<'tree>, source: &[u8]) -> Option<DeclarationCarrier<'tree>> {
        if matches!(self.language, "typescript" | "tsx") {
            typescript::declaration(node, source)
        } else {
            Some(javascript::declaration(node, source))
        }
    }
    /// Invoke `visitor` for language-specific call/reference sites using the root's source bytes.
    /// Rust synthetic macro sites include byte shifts and matching synthetic source bytes.
    pub fn visit_calls(self, root: Node<'_>, source: &[u8], visitor: &mut CallVisitor<'_>) {
        match self.language {
            "javascript" | "jsx" | "typescript" | "tsx" => javascript::visit_calls(root, source, visitor),
            "elixir" => elixir::visit_calls(root, source, visitor),
            "rust" => rust::visit_calls(root, source, visitor),
            _ => self.visit_standard_calls(root, source, visitor),
        }
    }
    fn visit_standard_calls(self, root: Node<'_>, source: &[u8], visitor: &mut CallVisitor<'_>) {
        let types = match self.language {
            "python" => &["call"][..],
            "go" => &["call_expression"][..],
            "java" => &["method_invocation", "object_creation_expression", "method_reference"][..],
            "kotlin" => &["call_expression", "callable_reference"][..],
            _ => syntax::FALLBACK_CALL_TYPES,
        };
        for node in walk(root).filter(|node| types.contains(&node.kind())) {
            visitor(CallSite::new(node), source);
        }
    }
    /// Return the optional target name, bare-name flag, and syntactic qualifier for a site.
    /// `source` must match the site's tree, including synthetic macro payload trees.
    pub fn normalize_call(self, site: &CallSite<'_>, source: &[u8]) -> CallTarget {
        match self.language {
            "javascript" | "jsx" | "typescript" | "tsx" => javascript::normalize_call(site, source),
            "kotlin" => kotlin::normalize_call(site, source),
            "rust" => rust::normalize_call(site, source),
            "elixir" => elixir::normalize_call(site, source),
            _ => syntax::normalize_call(site, source),
        }
    }
    /// Return `REFERENCES` for method/callable references and script JSX tags; otherwise `CALLS`.
    pub fn call_relationship(self, site: &CallSite<'_>) -> &'static str {
        if matches!(self.language, "javascript" | "jsx" | "typescript" | "tsx") && javascript::is_jsx_site(site.node) {
            "REFERENCES"
        } else {
            syntax::relationship(site)
        }
    }
    /// Return explicit visibility and annotation text using this language's modifier vocabulary.
    /// `source` must match the node's tree; default visibility is not inferred.
    pub fn metadata(self, node: Node<'_>, source: &[u8]) -> (Option<String>, Vec<String>) {
        let words: &[&str] = match self.language {
            "javascript" | "jsx" | "typescript" | "tsx" | "java" => &["public", "private", "protected"],
            "kotlin" => &["public", "private", "protected", "internal"],
            "rust" => &["pub"],
            _ => &["public", "private", "protected", "internal", "pub"],
        };
        syntax::modifier_metadata(node, source, words)
    }
    /// Supplement native imports with Kotlin import headers or Rust external module declarations.
    /// Use matching root/source bytes and one-based lines; deduplicate additions and sort by line.
    /// Return `native` unchanged for languages without supplemental import nodes.
    pub fn imports(self, root: Node<'_>, source: &[u8], native: Vec<(String, usize)>) -> Vec<(String, usize)> {
        let node_type = match self.language {
            "kotlin" => Some("import_header"),
            "rust" => Some("mod_item"),
            _ => None,
        };
        let policy = syntax::ImportPolicy {
            node_type,
            accept: |node| node.kind() != "mod_item" || node.child_by_field_name("body").is_none(),
        };
        syntax::supplement_imports((root, source), native, policy)
    }
    /// Extract syntactic base names under this language's policy; parser failures yield an empty list.
    pub fn inheritance(self, header: DeclarationHeader<'_>) -> Vec<String> {
        syntax::inheritance_names(header, self.inheritance_policy())
    }
    /// Return this language's header-only inheritance rules, or the broad fallback policy.
    pub fn inheritance_policy(self) -> InheritancePolicy {
        let mut policy = InheritancePolicy::new(&[]);
        match self.language {
            "javascript" | "jsx" | "typescript" | "tsx" => {
                policy.clauses = &["extends_clause", "extends_type_clause", "implements_clause"];
                policy.complete_header = true;
                policy.heritage_clause = Some("class_heritage");
            }
            "java" => {
                policy.clauses = &["superclass", "super_interfaces", "extends_interfaces"];
                policy.complete_header = true;
            }
            "kotlin" => {
                policy.clauses = &["delegation_specifier"];
                policy.complete_header = true;
                policy.kotlin_base = true;
            }
            "python" => policy.superclass_field = Some("superclasses"),
            "go" | "rust" | "elixir" => {}
            _ => policy = InheritancePolicy::default(),
        }
        policy
    }
}
