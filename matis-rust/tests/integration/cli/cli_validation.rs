//! Native clap validation and runtime error regressions through the executable.
use crate::support::cli as cli_support;
use cli_support::invoke;
use std::process::Command;
fn usage_error(args: &[&str], marker: &str) {
    let output = invoke(args);
    assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(marker), "{args:?}: {stderr}");
    assert!(stderr.contains("--help"), "{args:?}: {stderr}");
}

#[test]
fn schema_and_handler_errors_render_usage_through_the_binary() {
    // Full option matrices run in schema_tests and graph_options_tests without subprocesses.
    usage_error(&["graph", "find", "brew", "--limit", "bad", "--bogus"], "error:");
    usage_error(&["graph", "context", "brew", "--depth", "2", "--depth=-1"], "-1 is not in the range");
}

#[test]
fn help_bypasses_numeric_validation() {
    let output = invoke(&["graph", "find", "--limit", "0", "--help"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(String::from_utf8(output.stdout).unwrap().contains("Usage: matis graph find"));
}

#[test]
fn quoted_and_control_values_fail_safely() {
    usage_error(&["report", "--format", "a\nb"], "--format");
    usage_error(&["graph", "find", "brew", "--limit", "a'b"], "--limit");
}

#[test]
fn missing_database_has_sqlalchemy_style_connection_error_without_creating_files() {
    let root = tempfile::tempdir().unwrap();
    let database = root.path().join("absent").join("graph.sqlite");
    let output = Command::new(env!("CARGO_BIN_EXE_matis"))
        .env("DATABASE_PATH", &database)
        .args(["graph", "overview"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap(),
        serde_json::json!({"error": format!(
            "Cannot read graph database {}: (sqlite3.OperationalError) unable to open database file\n(Background on this error at: https://sqlalche.me/e/21/e3q8). Index the project after explicitly migrating the database.", database.display()
        )})
    );
    assert!(!database.parent().unwrap().exists());
}

#[test]
fn config_read_failure_includes_config_path() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("README.md");
    std::fs::write(&file, "not a directory").unwrap();
    let output = invoke(&["check", "-f", file.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr).unwrap().contains(file.join("matis.json").to_str().unwrap()));
}
