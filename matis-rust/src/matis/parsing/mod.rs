pub mod adapters;
mod catalog;
pub mod discovery;
mod ignore_spec;
mod inheritance;
pub mod mutation_candidates;
pub mod parser;
mod queries;
pub mod structure;
pub mod syntax;
#[cfg(test)]
#[path = "../../../tests/unit/parsing/tests.rs"]
mod tests;
