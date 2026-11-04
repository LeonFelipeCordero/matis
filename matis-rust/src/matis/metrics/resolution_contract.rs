use crate::domain::models::{Call, ParsedFile, Symbol};
use tree_sitter::Node;

#[derive(Clone)]
pub struct ImportRequest<'a> {
    pub file: &'a ParsedFile,
    pub path: String,
    pub position: Option<usize>,
    pub source: String,
}
#[derive(Clone)]
pub struct ResolutionRequest<'a> {
    pub file: &'a ParsedFile,
    pub call: &'a Call,
    pub owner: Option<&'a Symbol>,
    pub name: String,
    pub qualifier: Option<String>,
}
#[derive(Clone)]
pub struct CallTarget<'a> {
    pub callee: Option<Node<'a>>,
    pub receiver: Option<Node<'a>>,
    pub name: String,
    pub qualifier: Option<String>,
}
pub struct DeclarationLookup<'a> {
    pub name: &'a str,
    pub node: Node<'a>,
    pub position: usize,
}
pub const FUNCTIONS: &[&str] = &[
    "function_definition",
    "function_declaration",
    "function_expression",
    "arrow_function",
    "method_definition",
    "method_declaration",
    "constructor_declaration",
    "function_item",
    "lambda",
    "lambda_expression",
    "lambda_literal",
    "closure_expression",
    "func_literal",
    "method",
    "singleton_method",
];
pub const SCOPES: &[&str] = &[
    "function_definition",
    "function_declaration",
    "function_expression",
    "arrow_function",
    "method_definition",
    "method_declaration",
    "constructor_declaration",
    "function_item",
    "lambda",
    "lambda_expression",
    "lambda_literal",
    "closure_expression",
    "func_literal",
    "method",
    "singleton_method",
    "class_definition",
    "class_declaration",
    "class_specifier",
    "struct_specifier",
    "block",
    "statement_block",
    "compound_statement",
    "do_block",
];
