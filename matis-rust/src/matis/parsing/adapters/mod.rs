//! Dedicated extraction adapters and a generic fallback for the pinned catalog.
pub mod base;
pub mod elixir;
pub mod go;
pub mod java;
pub mod javascript;
pub mod kotlin;
pub mod python;
pub mod rust;
pub mod typescript;
pub use base::{LanguageAdapter, get_adapter};
#[cfg(test)]
#[path = "../../../../tests/unit/parsing/adapters/tests.rs"]
mod tests;
