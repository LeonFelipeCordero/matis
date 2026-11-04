use super::schema::cli;
use clap::error::ErrorKind;

#[test]
fn schema_is_consistent() {
    cli().debug_assert();
}

#[test]
fn version_reports_package_version_without_a_command() {
    for flag in ["--version", "-V"] {
        let error = cli().try_get_matches_from(["matis", flag]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::DisplayVersion);
        assert_eq!(error.exit_code(), 0);
        assert!(!error.use_stderr());
        assert_eq!(error.to_string(), format!("matis {}\n", env!("CARGO_PKG_VERSION")));
    }
}

#[test]
fn all_commands_offer_generated_help() {
    for path in [
        "",
        "index",
        "projects",
        "report",
        "check",
        "mutations",
        "mutations run",
        "graph",
        "graph overview",
        "graph find",
        "graph show",
        "graph children",
        "graph context",
        "graph path",
        "graph source",
    ] {
        let error = cli()
            .try_get_matches_from(std::iter::once("matis").chain(path.split_whitespace()).chain(["--help"]))
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::DisplayHelp, "{path}");
        assert_eq!(error.exit_code(), 0, "{path}");
        assert!(!error.use_stderr(), "{path}");
        let help = error.to_string();
        assert!(help.contains("Usage:"), "{path}: {help}");
        assert!(help.contains("--help"), "{path}: {help}");
    }
}

#[test]
fn syntax_errors_have_usage_status_and_diagnostics() {
    for args in [
        vec![],
        vec!["graph"],
        vec!["unknown"],
        vec!["index", "--folder"],
        vec!["projects", "extra"],
        vec!["check", "--unknown"],
        vec!["check", "--format", "xml"],
        vec!["graph", "show"],
        vec!["graph", "path", "from"],
        vec!["graph", "find", "brew", "--limit", "bad", "--bogus"],
        vec!["graph", "find", "brew", "--bogus", "--limit", "bad"],
        vec!["graph", "path", "--max-depth", "bad"],
        vec!["graph", "overview", "extra", "--bogus"],
        vec!["report", "--format", "yaml", "--bogus"],
        vec!["graph", "context", "brew", "--depth", "bad", "--direction", "wrong"],
    ] {
        let error = cli().try_get_matches_from(std::iter::once("matis").chain(args.iter().copied())).unwrap_err();
        assert_eq!(error.exit_code(), 2, "{args:?}");
        assert!(error.use_stderr(), "{args:?}");
        assert!(error.to_string().contains("--help"), "{args:?}: {error}");
    }
}

#[test]
fn quoted_and_control_values_fail_safely() {
    let cases = ["a'b", "a\"b", "a\\b", "a\nb", "a\tb", "a\rb", "a\u{1b}b"]
        .into_iter()
        .flat_map(|value| [vec!["report", "--format", value], vec!["check", "--format", value], vec![value]]);
    for args in cases {
        let error = cli().try_get_matches_from(std::iter::once("matis").chain(args.iter().copied())).unwrap_err();
        assert_eq!(error.exit_code(), 2, "{args:?}");
        assert!(error.use_stderr());
        let message = error.to_string();
        assert!(message.contains("error:"), "{message}");
        assert!(message.contains("--help"), "{message}");
        if args.len() > 1 {
            assert!(message.contains("--format"), "{message}");
        }
    }
}

#[test]
fn help_bypasses_numeric_validation() {
    let error = cli().try_get_matches_from(["matis", "graph", "find", "--limit", "0", "--help"]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::DisplayHelp);
    assert!(error.to_string().contains("Usage: matis graph find"));
}
