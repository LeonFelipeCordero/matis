//! Real process, parser, database, and snapshot contracts. Run separately from unit tests.
#[path = "integration/architecture.rs"]
mod architecture;
#[path = "integration/cli/mod.rs"]
mod cli;
#[path = "integration/mutations/mod.rs"]
mod mutations;
#[path = "integration/parsing/mod.rs"]
mod parsing;
#[path = "integration/persistence/mod.rs"]
mod persistence;
#[path = "integration/pipeline/mod.rs"]
mod pipeline;
#[path = "integration/services/mod.rs"]
mod services;
#[path = "integration/support/mod.rs"]
mod support;
