//! Conservative JVM invocation shapes. Unknown types remain viable.
use super::{
    super::{
        receiver_syntax::{child, type_name},
        resolution_contract::ResolutionRequest,
        resolution_syntax::Syntax,
    },
    resolution::CallableIdentity,
};
use crate::{
    domain::models::Symbol,
    syntax::{children, field, named_children, text},
};
use regex::Regex;
use std::{collections::BTreeSet, sync::LazyLock};
use tree_sitter::Node;

#[derive(Clone, Debug)]
pub struct Clause {
    pub minimum: usize,
    pub maximum: usize,
    pub private: bool,
}
#[derive(Clone, Debug)]
pub struct Parameter {
    pub name: String,
    pub annotation: Option<String>,
    pub default: bool,
    pub variadic: bool,
}
#[derive(Clone, Debug)]
pub struct Argument<'a> {
    pub value: Node<'a>,
    pub name: Option<String>,
    pub spread: bool,
}
#[derive(Clone, Copy)]
pub struct TypeContext<'a> {
    pub syntax: &'a Syntax,
    pub ambiguous: &'a BTreeSet<String>,
}
pub struct CallContext<'a> {
    pub source: TypeContext<'a>,
    pub target: TypeContext<'a>,
    pub declaration: Option<Node<'a>>,
}

pub struct Assignment {
    pub parameters: Vec<Parameter>,
    pub assigned: BTreeSet<usize>,
    pub position: usize,
}
impl Assignment {
    /// Start argument assignment for `parameters` with no assigned slots and position zero.
    pub fn new(parameters: Vec<Parameter>) -> Self {
        Self { parameters, assigned: BTreeSet::new(), position: 0 }
    }
    /// Locate a unique named parameter or the next positional slot for `argument`.
    /// Advance past non-variadic positional slots; return `None` for missing/ambiguous names or excess inputs.
    pub fn index(&mut self, argument: &Argument<'_>) -> Option<usize> {
        if let Some(name) = &argument.name {
            let indexes: Vec<_> =
                self.parameters.iter().enumerate().filter(|(_, p)| p.name == *name).map(|(i, _)| i).collect();
            return if indexes.len() == 1 { Some(indexes[0]) } else { None };
        }
        let index = self.position;
        let parameter = self.parameters.get(index)?;
        if !parameter.variadic {
            self.position += 1;
        }
        Some(index)
    }
    /// Record an assignment to a valid parameter `index`; reject repeats except for variadic slots.
    pub fn assign(&mut self, index: usize) -> bool {
        if self.assigned.contains(&index) && !self.parameters[index].variadic {
            return false;
        }
        self.assigned.insert(index);
        true
    }
    /// Test whether every required, non-variadic parameter has an assignment.
    pub fn complete(&self) -> bool {
        self.parameters.iter().enumerate().all(|(i, p)| self.assigned.contains(&i) || p.default || p.variadic)
    }
}
fn sole(matches: Vec<Symbol>) -> Vec<Symbol> {
    if matches.len() == 1 { matches } else { vec![] }
}
/// Return the sole viable JVM callable for `request`, or no candidates if ambiguity remains.
/// Retain candidates with unknown signatures or types; absent argument syntax requires an already unique match.
/// `identity` must contain syntax for the request and candidate files.
pub fn select_callable(
    identity: &CallableIdentity,
    request: &ResolutionRequest<'_>,
    matches: Vec<Symbol>,
) -> Vec<Symbol> {
    let syntax = &identity.syntax[&request.file.source.path];
    let call = syntax.node(request.call.start_byte, request.call.end_byte);
    let Some(arguments) = call_arguments(call, &syntax.source) else {
        return sole(matches);
    };
    let ambiguous = identity.syntax.values().flat_map(|s| s.file.symbols.iter().map(|s| s.name.clone())).collect();
    let source = TypeContext { syntax, ambiguous: &ambiguous };
    sole(
        matches
            .into_iter()
            .filter(|symbol| {
                let target_syntax = &identity.syntax[&symbol.file_path];
                let declaration = target_syntax.node(symbol.start_byte, symbol.end_byte);
                let parameters = callable_parameters(declaration, &target_syntax.source);
                let context = CallContext {
                    source,
                    target: TypeContext { syntax: target_syntax, ambiguous: &ambiguous },
                    declaration,
                };
                parameters.is_none_or(|p| accepts(&context, p, &arguments))
            })
            .collect(),
    )
}
/// Read Java/Kotlin invocation arguments using `source` for Kotlin names.
/// Retain direct name/spread tokens; return `None` for missing, invalid, or unsupported syntax,
/// including Kotlin trailing lambdas.
pub fn call_arguments<'a>(node: Option<Node<'a>>, source: &[u8]) -> Option<Vec<Argument<'a>>> {
    let node = node.filter(|n| !n.has_error())?;
    if ["method_invocation", "object_creation_expression"].contains(&node.kind()) {
        return Some(
            named_children(field(node, &["arguments"])?)
                .into_iter()
                .map(|value| Argument { value, name: None, spread: false })
                .collect(),
        );
    }
    if node.kind() != "call_expression" {
        return None;
    }
    let suffix = child(node, &["call_suffix"])?;
    if child(suffix, &["annotated_lambda"]).is_some() {
        return None;
    }
    let container = child(suffix, &["value_arguments"])?;
    named_children(container)
        .into_iter()
        .map(|entry| {
            if entry.kind() != "value_argument" {
                return None;
            }
            let named = named_children(entry);
            let value = *named.last()?;
            let tokens = children(entry);
            Some(Argument {
                value,
                name: tokens.iter().any(|c| c.kind() == "=").then(|| text(named.first().copied(), source)),
                spread: tokens.iter().any(|c| c.kind() == "*"),
            })
        })
        .collect()
}
/// Read Java/Kotlin parameter names, annotations, defaults, and variadic flags from matching `source`.
/// Return `None` for missing/error syntax, unsupported containers, or explicit receiver parameters.
pub fn callable_parameters(node: Option<Node<'_>>, source: &[u8]) -> Option<Vec<Parameter>> {
    let node = node.filter(|n| !n.has_error())?;
    let container = field(node, &["parameters"]).or_else(|| child(node, &["function_value_parameters"]))?;
    if !["formal_parameters", "function_value_parameters"].contains(&container.kind()) {
        return None;
    }
    let children = children(container);
    let mut result = Vec::new();
    for (index, entry) in children.iter().enumerate() {
        if ["receiver_parameter", "ERROR"].contains(&entry.kind()) {
            return None;
        }
        if let Some(parameter) = parameter(&children, index, source) {
            result.push(parameter);
        }
    }
    Some(result)
}
fn parameter(children: &[Node<'_>], index: usize, source: &[u8]) -> Option<Parameter> {
    let entry = children[index];
    if ["formal_parameter", "spread_parameter"].contains(&entry.kind()) {
        let name = field(entry, &["name"]).or_else(|| child(entry, &["variable_declarator"]));
        let annotation = field(entry, &["type"])
            .or_else(|| child(entry, &["integral_type", "floating_point_type", "type_identifier"]));
        return Some(Parameter {
            name: text(name, source),
            annotation: type_name(annotation, source),
            default: false,
            variadic: entry.kind() == "spread_parameter",
        });
    }
    if entry.kind() != "parameter" {
        return None;
    }
    Some(Parameter {
        name: text(child(entry, &["simple_identifier"]), source),
        annotation: type_name(child(entry, &["user_type", "nullable_type", "function_type"]), source),
        default: children.get(index + 1).is_some_and(|n| n.kind() == "="),
        variadic: index
            .checked_sub(1)
            .and_then(|i| children.get(i))
            .is_some_and(|n| text(Some(*n), source).split_whitespace().any(|s| s == "vararg")),
    })
}
/// Test argument assignment and proven type compatibility using source/target `context`.
/// Honor defaults, named arguments, and variadic parameters; unknown types remain viable.
/// A spread argument keeps the candidate viable without further shape checks.
pub fn accepts(context: &CallContext<'_>, parameters: Vec<Parameter>, arguments: &[Argument<'_>]) -> bool {
    if arguments.iter().any(|a| a.spread) {
        return true;
    }
    let mut assignment = Assignment::new(parameters);
    for argument in arguments {
        if !accept_argument(context, &mut assignment, argument) {
            return false;
        }
    }
    assignment.complete()
}
fn accept_argument(context: &CallContext<'_>, assignment: &mut Assignment, argument: &Argument<'_>) -> bool {
    let Some(index) = assignment.index(argument) else {
        return false;
    };
    if !assignment.assign(index) {
        return false;
    }
    let parameter = &assignment.parameters[index];
    let actual = expression_type(&context.source, argument.value);
    let expected = proven_type(&context.target, parameter.annotation.as_deref(), context.declaration);
    if argument.name.as_deref().is_some_and(|n| !n.is_empty()) && parameter.variadic {
        return true;
    }
    !incompatible(actual.as_deref(), expected.as_deref(), context.source.syntax.adapter.family())
}
/// Infer a supported literal or uniquely declared identifier's type in `context`.
/// Return `None` when the type cannot be established conservatively.
pub fn expression_type(context: &TypeContext<'_>, node: Node<'_>) -> Option<String> {
    let kotlin = context.syntax.adapter.family() == "kotlin";
    match node.kind() {
        "string_literal" | "multiline_string_literal" => Some("String".into()),
        "character_literal" => Some(if kotlin { "Char" } else { "char" }.into()),
        "boolean_literal" | "true" | "false" => Some(if kotlin { "Boolean" } else { "boolean" }.into()),
        "decimal_integer_literal"
        | "hex_integer_literal"
        | "octal_integer_literal"
        | "binary_integer_literal"
        | "integer_literal" => integer_type(&context.syntax.text(node), kotlin),
        "identifier" | "simple_identifier" => identifier_type(context, node),
        _ => None,
    }
}
fn integer_type(value: &str, kotlin: bool) -> Option<String> {
    if value.contains(['u', 'U']) {
        return None;
    }
    Some(
        match (kotlin, value.to_lowercase().ends_with('l')) {
            (true, true) => "Long",
            (true, false) => "Int",
            (false, true) => "long",
            _ => "int",
        }
        .into(),
    )
}
fn identifier_type(context: &TypeContext<'_>, node: Node<'_>) -> Option<String> {
    let syntax = context.syntax;
    let bindings: Vec<_> = syntax
        .nodes()
        .into_iter()
        .flat_map(|n| syntax.adapter.receiver_bindings(syntax, n))
        .filter(|b| {
            b.name == syntax.text(node) && !b.member && !b.inferred && syntax.visible(b.scope, node.start_byte())
        })
        .collect();
    let width = bindings.iter().map(|b| b.scope.end_byte() - b.scope.start_byte()).min()?;
    let nearest: Vec<_> = bindings.iter().filter(|b| b.scope.end_byte() - b.scope.start_byte() == width).collect();
    if nearest.len() != 1 {
        return None;
    }
    proven_type(context, nearest[0].annotation.as_deref(), Some(nearest[0].node))
}
const JAVA_NUMERIC: &[&str] = &["byte", "short", "int", "long", "float", "double", "char"];
const JAVA_PRIMITIVES: &[&str] = &["byte", "short", "int", "long", "float", "double", "char", "boolean"];
const JAVA_STRINGS: &[&str] = &["String", "java.lang.String"];
const KOTLIN_CLOSED: &[&str] = &["String", "Char", "Boolean", "Int", "Long", "Float", "Double", "Byte", "Short"];
/// Retain an annotation only when declaration syntax and context establish it conservatively.
/// Missing inputs, ambiguous names, imports, and generic/nullable syntax can yield `None`;
/// Java primitive annotations are accepted directly when a declaration node is present.
pub fn proven_type(context: &TypeContext<'_>, annotation: Option<&str>, node: Option<Node<'_>>) -> Option<String> {
    let annotation = annotation?;
    let node = node?;
    if context.syntax.adapter.family() == "java" && JAVA_PRIMITIVES.contains(&annotation) {
        return Some(annotation.into());
    }
    if uncertain_type(context, annotation, node) { None } else { Some(annotation.into()) }
}
fn uncertain_type(context: &TypeContext<'_>, annotation: &str, node: Node<'_>) -> bool {
    let syntax = context.syntax;
    if context.ambiguous.contains(annotation) || !syntax.imports().is_empty() {
        return true;
    }
    syntax.nodes().into_iter().any(|n| {
        if ["type_parameter", "type_parameters"].contains(&n.kind()) && mentions_word(&syntax.text(n), annotation) {
            return true;
        }
        node.start_byte() <= n.start_byte()
            && n.start_byte() < node.end_byte()
            && ["nullable_type", "generic_type", "user_type"].contains(&n.kind())
            && syntax.text(n).contains(['?', '<'])
    })
}
fn mentions_word(value: &str, name: &str) -> bool {
    value.char_indices().filter(|(start, _)| value[*start..].starts_with(name)).any(|(start, _)| {
        let end = start + name.len();
        word_boundary(value[..start].chars().next_back(), value[start..].chars().next())
            && word_boundary(value[..end].chars().next_back(), value[end..].chars().next())
    })
}
fn word_boundary(left: Option<char>, right: Option<char>) -> bool {
    python_word(left) != python_word(right)
}
fn python_word(value: Option<char>) -> bool {
    static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\A[\p{L}\p{N}_]\z").unwrap());
    value.is_some_and(|c| WORD.is_match(c.encode_utf8(&mut [0; 4])))
}
/// Test only proven conflicts between closed built-in types in the requested JVM `family`.
/// Unknown types and allowed numeric conversions are not conflicts; non-Kotlin families use Java rules.
pub fn incompatible(actual: Option<&str>, expected: Option<&str>, family: &str) -> bool {
    let (Some(actual), Some(expected)) = (actual, expected) else {
        return false;
    };
    if actual == expected {
        return false;
    }
    if family == "kotlin" { kotlin_incompatible(actual, expected) } else { java_incompatible(actual, expected) }
}
fn kotlin_incompatible(actual: &str, expected: &str) -> bool {
    if actual == "Int" && ["Byte", "Short", "Long"].contains(&expected) {
        return false;
    }
    KOTLIN_CLOSED.contains(&actual) && KOTLIN_CLOSED.contains(&expected)
}
fn java_incompatible(actual: &str, expected: &str) -> bool {
    if JAVA_NUMERIC.contains(&actual) && JAVA_NUMERIC.contains(&expected) {
        return false;
    }
    if JAVA_STRINGS.contains(&actual) && JAVA_STRINGS.contains(&expected) {
        return false;
    }
    (JAVA_PRIMITIVES.contains(&actual) || JAVA_STRINGS.contains(&actual))
        && (JAVA_PRIMITIVES.contains(&expected) || JAVA_STRINGS.contains(&expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::models::{ParsedFile, SourceFile},
        metrics::adapters::adapter_for_language,
    };

    fn syntax(language: &str, source: &str) -> Syntax {
        let file = ParsedFile {
            source: SourceFile {
                path: "sample".into(),
                absolute_path: "sample".into(),
                language: language.into(),
                source: source.as_bytes().to_vec(),
            },
            content_hash: String::new(),
            parse_error: None,
            symbols: vec![],
            imports: vec![],
            calls: vec![],
            inheritances: vec![],
        };
        Syntax::new(&file, adapter_for_language(language))
    }
    #[test]
    fn python_boundaries_exclude_combining_marks_but_include_unicode_numbers() {
        assert!(mentions_word("x\u{301}String\u{301}x", "String"));
        assert!(!mentions_word("ⅧString", "String"));
        assert!(!mentions_word("String_", "String"));
        assert!(mentions_word("xA.A.A", "A.A"));
    }
    #[test]
    fn closed_type_conflicts_remain_conservative() {
        assert!(incompatible(Some("String"), Some("int"), "java"));
        assert!(!incompatible(Some("int"), Some("long"), "java"));
        assert!(!incompatible(Some("String"), Some("Object"), "java"));
        assert!(!incompatible(Some("Int"), Some("Long"), "kotlin"));
        assert!(incompatible(Some("Long"), Some("Int"), "kotlin"));
        assert!(!incompatible(None, Some("String"), "kotlin"));
    }
    #[test]
    fn java_parameters_arguments_and_builtin_type_conflicts() {
        let syntax = syntax("java", "class A { void f(int x, String y) { f(1, true); } }");
        let ambiguous = BTreeSet::new();
        let context = TypeContext { syntax: &syntax, ambiguous: &ambiguous };
        let declaration = syntax.nodes().into_iter().find(|n| n.kind() == "method_declaration").unwrap();
        let call = syntax.nodes().into_iter().find(|n| n.kind() == "method_invocation").unwrap();
        let arguments = call_arguments(Some(call), &syntax.source).unwrap();
        let parameters = callable_parameters(Some(declaration), &syntax.source).unwrap();
        assert_eq!(parameters.len(), 2);
        assert_eq!(parameters[0].annotation.as_deref(), Some("int"));
        let context = CallContext { source: context, target: context, declaration: Some(declaration) };
        assert!(!accepts(&context, parameters, &arguments));
    }
    #[test]
    fn kotlin_named_defaults_and_spreads_preserve_argument_shapes() {
        let syntax = syntax("kotlin", "fun f(x: Int, y: String = \"a\") { f(y = \"b\", x = 1); f(*xs) }");
        let calls: Vec<_> = syntax.nodes().into_iter().filter(|n| n.kind() == "call_expression").collect();
        let arguments = call_arguments(Some(calls[0]), &syntax.source).unwrap();
        assert_eq!(arguments[0].name.as_deref(), Some("y"));
        assert_eq!(arguments[1].name.as_deref(), Some("x"));
        // The reference tests direct '*' tokens, not nested spread_expression nodes.
        assert!(!call_arguments(Some(calls[1]), &syntax.source).unwrap()[0].spread);
        let declaration = syntax.nodes().into_iter().find(|n| n.kind() == "function_declaration").unwrap();
        let parameters = callable_parameters(Some(declaration), &syntax.source).unwrap();
        assert!(!parameters[0].default);
        assert!(parameters[1].default);
    }
    #[test]
    fn duplicate_assignments_and_missing_inputs_fail() {
        let syntax = syntax("java", "class A { void f() { f(1); } }");
        let value = syntax.nodes().into_iter().find(|n| n.kind() == "decimal_integer_literal").unwrap();
        let argument = Argument { value, name: None, spread: false };
        let mut assignment =
            Assignment::new(vec![Parameter { name: "x".into(), annotation: None, default: false, variadic: false }]);
        assert!(!assignment.complete());
        assert_eq!(assignment.index(&argument), Some(0));
        assert!(assignment.assign(0));
        assert!(!assignment.assign(0));
        assert!(assignment.complete());
        assert_eq!(assignment.index(&argument), None);
    }
    #[test]
    fn nullable_and_generic_annotations_do_not_prove_types() {
        let syntax = syntax("kotlin", "fun f(x: String?) {} ");
        let ambiguous = BTreeSet::new();
        let context = TypeContext { syntax: &syntax, ambiguous: &ambiguous };
        let declaration = syntax.nodes().into_iter().find(|n| n.kind() == "function_declaration").unwrap();
        assert_eq!(proven_type(&context, Some("String"), Some(declaration)), None);
    }
}
