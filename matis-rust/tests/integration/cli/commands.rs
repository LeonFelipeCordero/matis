//! Exercise clap through the real binary without touching the caller's database.
use crate::support::cli as cli_support;
use cli_support::invoke;

#[test]
fn representative_commands_offer_generated_help_without_side_effects() {
    // The schema unit tests cover every command; keep process routing and stream checks here.
    for path in ["", "mutations run", "graph source"] {
        let mut args: Vec<_> = path.split_whitespace().collect();
        args.push("--help");
        let result = invoke(&args);
        assert_eq!(result.status.code(), Some(0), "{path}");
        assert!(result.stderr.is_empty(), "{path}");
        let help = String::from_utf8(result.stdout).unwrap();
        assert!(help.contains("Usage:"), "{path}: {help}");
        assert!(help.contains("--help"), "{path}: {help}");
    }
}

#[test]
fn syntax_errors_exit_two_before_opening_a_database() {
    for args in [
        vec![],
        vec!["check", "--format", "xml"],
        vec!["graph", "find", "brew", "--limit", "0"],
        vec!["graph", "context", "brew", "--relations", "INVALID"],
    ] {
        let result = invoke(&args);
        assert_eq!(result.status.code(), Some(2), "{args:?}");
        assert!(result.stdout.is_empty(), "{args:?}");
        assert!(!result.stderr.is_empty(), "{args:?}");
    }
}

#[test]
fn rules_bypass_the_filesystem_and_database() {
    let output = invoke(&["mutations", "--folder", "/does-not-exist", "--rules", "--saved"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(payload["rules"].is_array());
}
