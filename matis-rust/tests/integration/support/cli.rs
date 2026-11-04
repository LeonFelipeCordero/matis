//! Validation commands must not touch a database or depend on the caller's environment.
use std::process::{Command, Output};

pub fn invoke(args: &[&str]) -> Output {
    let root = tempfile::tempdir().unwrap();
    let database = root.path().join("must-not-exist.sqlite");
    let output = Command::new(env!("CARGO_BIN_EXE_matis"))
        .current_dir(root.path())
        .env("DATABASE_PATH", &database)
        .env("XDG_DATA_HOME", root.path().join("data"))
        .env("COLUMNS", "80")
        .env("NO_COLOR", "1")
        .args(args)
        .output()
        .unwrap();
    assert!(!database.exists(), "command opened the database: {args:?}");
    output
}
