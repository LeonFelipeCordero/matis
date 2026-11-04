use super::resolution_policy::ResolutionAdapter;
use crate::metrics::resolution_contract::SCOPES;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

const POLICY_LANGUAGES: &[&str] = &["", "python", "javascript", "java", "kotlin", "go", "rust", "elixir"];
macro_rules! kind_set {
    ($name:ident,$raw:ident) => {
        /// Return this hook's immutable grammar/name set for the canonical language family.
        /// Initialize shared family sets on first access; unsupported hooks can be empty.
        ///
        /// | Hook | Set contents |
        /// | --- | --- |
        /// | `scopes` | Lexical scope boundaries. |
        /// | `import_nodes` | Import directive node kinds. |
        /// | `self_qualifiers` | Implicit receiver spellings such as `self` or `this`. |
        /// | `constructors` | Constructor invocation node kinds. |
        /// | `receiver_class_nodes` | Expressions eligible for constructor-based type inference. |
        /// | `receiver_names` | Special receiver node kinds accepted by qualified lookup. |
        /// | `assignments` | Assignment node kinds used for bindings and writes. |
        /// | `declarations` | Variable declaration node kinds. |
        /// | `loop_bindings` | Loop node kinds with binding targets. |
        /// | `static_nodes` | Direct static marker node kinds. |
        /// | `static_containers` | Modifier containers that can contain static text. |
        /// | `member_writes` | Nodes that can replace or delete members. |
        /// | `receiver_updates` | Update-expression node kinds that can modify receivers. |
        /// | `rebound_declarations` | Declarations exempt from class-body rebinding checks. |
        /// | `single_parameter_functions` | Functions with a possible bare parameter node. |
        /// | `macro_nodes` | Macro invocation node kinds excluded from callable resolution. |
        pub fn $name(&self) -> &'static BTreeSet<&'static str> {
            static SETS: LazyLock<BTreeMap<&'static str, BTreeSet<&'static str>>> = LazyLock::new(|| {
                POLICY_LANGUAGES
                    .iter()
                    .map(|language| {
                        let adapter = ResolutionAdapter { language: (*language).into() };
                        (*language, adapter.$raw().iter().copied().collect())
                    })
                    .collect()
            });
            &SETS[self.family()]
        }
    };
}
impl ResolutionAdapter {
    kind_set!(scopes, scopes_raw);
    kind_set!(import_nodes, import_nodes_raw);
    kind_set!(self_qualifiers, self_qualifiers_raw);
    kind_set!(constructors, constructors_raw);
    kind_set!(receiver_class_nodes, receiver_class_nodes_raw);
    kind_set!(receiver_names, receiver_names_raw);
    kind_set!(assignments, assignments_raw);
    kind_set!(declarations, declarations_raw);
    kind_set!(loop_bindings, loop_bindings_raw);
    kind_set!(static_nodes, static_nodes_raw);
    kind_set!(static_containers, static_containers_raw);
    kind_set!(member_writes, member_writes_raw);
    kind_set!(receiver_updates, receiver_updates_raw);
    kind_set!(rebound_declarations, rebound_declarations_raw);
    kind_set!(single_parameter_functions, single_parameter_functions_raw);
    kind_set!(macro_nodes, macro_nodes_raw);
    /// Return the canonical policy family, merging script aliases; unknown languages return an empty string.
    pub fn family(&self) -> &str {
        match self.language.as_str() {
            "python" => "python",
            "javascript" | "jsx" | "typescript" | "tsx" => "javascript",
            "java" => "java",
            "kotlin" => "kotlin",
            "go" => "go",
            "rust" => "rust",
            "elixir" => "elixir",
            _ => "",
        }
    }
    /// Test whether this language has a dedicated resolution family rather than generic fallback.
    pub fn dedicated(&self) -> bool {
        !self.family().is_empty()
    }
    /// Test whether the policy belongs to the JavaScript/JSX/TypeScript/TSX family.
    pub fn script(&self) -> bool {
        self.family() == "javascript"
    }
    /// Test whether this policy uses Java or Kotlin resolution rules.
    pub fn jvm(&self) -> bool {
        ["java", "kotlin"].contains(&self.language.as_str())
    }
    fn scopes_raw(&self) -> Vec<&'static str> {
        let extra: &[&str] = match self.family() {
            "python" => {
                return vec![
                    "function_definition",
                    "lambda",
                    "class_definition",
                    "list_comprehension",
                    "set_comprehension",
                    "dictionary_comprehension",
                    "generator_expression",
                ];
            }
            "rust" => &["declaration_list"],
            "java" => &["for_statement", "enhanced_for_statement", "catch_clause"],
            "kotlin" => &["secondary_constructor", "function_body", "control_structure_body", "for_statement"],
            "go" => &["for_statement"],
            _ => &[],
        };
        SCOPES.iter().chain(extra).copied().collect()
    }
    fn import_nodes_raw(&self) -> &[&'static str] {
        match self.language.as_str() {
            "python" => &["import_statement", "import_from_statement"],
            "javascript" | "jsx" | "typescript" | "tsx" => &["import_statement"],
            "java" => &["import_declaration"],
            "kotlin" => &["import_header"],
            "go" => &["import_declaration"],
            "rust" => &["use_declaration", "mod_item"],
            _ => &[],
        }
    }
    /// Indicate whether lexical lookup must skip type scopes, as in Python and script languages.
    pub fn skip_type_scope(&self) -> bool {
        self.language == "python" || self.script()
    }
    /// Indicate whether import resolution requires a matched syntax node, as for Rust.
    pub fn require_import_node(&self) -> bool {
        self.language == "rust"
    }
    fn self_qualifiers_raw(&self) -> &[&'static str] {
        match self.language.as_str() {
            "python" | "rust" => &["self"],
            "java" | "kotlin" | "javascript" | "jsx" | "typescript" | "tsx" => &["this"],
            _ => &[],
        }
    }
    fn constructors_raw(&self) -> &[&'static str] {
        match self.language.as_str() {
            "java" | "kotlin" => &["object_creation_expression"],
            "javascript" | "jsx" | "typescript" | "tsx" => &["new_expression"],
            _ => &[],
        }
    }
    fn receiver_class_nodes_raw(&self) -> &[&'static str] {
        match self.language.as_str() {
            "python" => &["call"],
            "java" => &["object_creation_expression"],
            "kotlin" => &["object_creation_expression", "call_expression"],
            "javascript" | "jsx" | "typescript" | "tsx" => &["new_expression"],
            "rust" => &["struct_expression"],
            "go" => &["composite_literal", "unary_expression"],
            _ => &[],
        }
    }
    fn receiver_names_raw(&self) -> &[&'static str] {
        match self.language.as_str() {
            "java" | "kotlin" | "javascript" | "jsx" | "typescript" | "tsx" => &["this"],
            "elixir" => &["alias"],
            _ => &[],
        }
    }
    /// Allow bare field names to serve as instance receivers for JVM languages.
    pub fn bare_instance_fields(&self) -> bool {
        self.jvm()
    }
    /// Indicate whether declared receiver annotations can be trusted without Python's extra checks.
    pub fn trust_receiver_annotations(&self) -> bool {
        self.language != "python"
    }
    /// Require initializer compatibility checks for annotated Python and script receivers.
    pub fn validate_receiver_initializer(&self) -> bool {
        self.language == "python" || self.script()
    }
    fn assignments_raw(&self) -> &[&'static str] {
        match self.language.as_str() {
            "python" => &["assignment", "augmented_assignment", "named_expression"],
            "javascript" | "jsx" | "typescript" | "tsx" => {
                &["assignment_expression", "augmented_assignment_expression"]
            }
            "java" => &["assignment_expression"],
            "kotlin" => &["assignment"],
            "go" => &["assignment_statement", "short_var_declaration"],
            "rust" => &["assignment_expression", "compound_assignment_expr"],
            _ => &[],
        }
    }
    fn declarations_raw(&self) -> &[&'static str] {
        match self.language.as_str() {
            "javascript" | "jsx" | "typescript" | "tsx" => &["variable_declarator"],
            "java" => &["variable_declarator", "init_declarator"],
            "kotlin" => &["variable_declaration"],
            "go" => &["var_spec"],
            "rust" => &["let_declaration"],
            _ => &[],
        }
    }
    fn loop_bindings_raw(&self) -> &[&'static str] {
        match self.language.as_str() {
            "python" => &["for_statement", "for_in_clause"],
            "javascript" | "jsx" | "typescript" | "tsx" => &["for_statement", "for_in_statement"],
            "java" => &["for_statement"],
            _ => &[],
        }
    }
    fn static_nodes_raw(&self) -> &[&'static str] {
        if self.script() || self.jvm() || self.language == "rust" { &["static"] } else { &[] }
    }
    fn static_containers_raw(&self) -> &[&'static str] {
        match self.language.as_str() {
            "rust" => &["storage_class_specifier", "modifiers"],
            _ if self.jvm() || self.script() => &["modifiers"],
            _ => &[],
        }
    }
    fn member_writes_raw(&self) -> &[&'static str] {
        match self.language.as_str() {
            "python" => &["assignment", "augmented_assignment", "delete_statement"],
            "javascript" | "jsx" | "typescript" | "tsx" => {
                &["assignment_expression", "augmented_assignment_expression", "update_expression", "unary_expression"]
            }
            "java" => &["assignment_expression", "update_expression"],
            "kotlin" => &["assignment", "prefix_expression", "postfix_expression"],
            _ => &[],
        }
    }
    fn receiver_updates_raw(&self) -> &[&'static str] {
        match self.language.as_str() {
            "kotlin" => &["prefix_expression", "postfix_expression"],
            "go" => &["inc_statement", "dec_statement"],
            _ => &["update_expression"],
        }
    }
    fn rebound_declarations_raw(&self) -> &[&'static str] {
        if self.language == "python" { &["function_definition", "class_definition"] } else { &[] }
    }
    fn single_parameter_functions_raw(&self) -> &[&'static str] {
        match self.language.as_str() {
            "java" | "kotlin" => &["lambda_expression"],
            _ if self.script() => &["arrow_function"],
            _ => &[],
        }
    }
    fn macro_nodes_raw(&self) -> &[&'static str] {
        if self.language == "rust" { &["macro_invocation"] } else { &[] }
    }
    /// Return `(node kind, receiver fields, callee fields)` rules for member-expression extraction.
    /// Families without shared member-node mappings return an empty slice.
    pub fn member_nodes(&self) -> &[(&str, &[&str], &[&str])] {
        match self.language.as_str() {
            "python" => &[("attribute", &["object"], &["attribute"])],
            "javascript" | "jsx" | "typescript" | "tsx" => &[("member_expression", &["object"], &["property"])],
            "go" => &[("selector_expression", &["operand"], &["field"])],
            "rust" => &[("field_expression", &["value"], &["field"])],
            "elixir" => &[("dot", &["left"], &["right"])],
            _ => &[],
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_aliases_share_immutable_policy_sets() {
        let script = ResolutionAdapter { language: "javascript".into() };
        let typescript = ResolutionAdapter { language: "tsx".into() };
        assert!(std::ptr::eq(script.assignments(), typescript.assignments()));
        let generic = ResolutionAdapter { language: "unknown".into() };
        assert!(generic.assignments().is_empty());
        assert!(generic.scopes().contains("block"));
    }
}
