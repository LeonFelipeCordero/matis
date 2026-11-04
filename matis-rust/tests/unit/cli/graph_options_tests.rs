use super::{
    graph_options::{GraphOptions, Operation},
    options::UsageError,
    schema,
};

fn parse(args: &[&str]) -> anyhow::Result<GraphOptions> {
    let matches = schema::cli().try_get_matches_from(["matis", "graph"].into_iter().chain(args.iter().copied()))?;
    let (command, matches) = matches.subcommand_matches("graph").unwrap().subcommand().unwrap();
    GraphOptions::from_matches(command, matches)
}

fn usage_error(args: &[&str], marker: &str) {
    let error = match parse(args) {
        Ok(_) => panic!("expected usage error: {args:?}"),
        Err(error) => error,
    };
    let error = error.downcast_ref::<UsageError>().unwrap();
    assert_eq!(error.command, format!("graph {}", args[0]));
    assert!(error.message.contains(marker), "{args:?}: {error}");
}

#[test]
fn decimal_forms_build_find_requests_without_navigation() {
    for (text, expected) in [
        (" 1_0 ", 10),
        ("\t+2\n", 2),
        ("１００", 100),
        ("٢", 2),
        ("𝟚", 2),
        ("𝟤", 2),
        ("𝟮", 2),
        ("𝟸", 2),
        ("\u{85}١٠\u{a0}", 10),
    ] {
        let Operation::Find(query, limit) = parse(&["find", "brew", "--limit", text]).unwrap().operation else {
            panic!("expected find request");
        };
        assert_eq!(query, "brew");
        assert_eq!(limit, expected, "{text:?}");
    }
}

#[test]
fn relation_lists_strip_python_whitespace_and_normalize_each_part() {
    for whitespace in ['\u{1c}', '\u{1d}', '\u{1e}', '\u{1f}', '\u{85}', '\u{a0}', '\u{2003}'] {
        let relations = format!("{whitespace}calls{whitespace}, {whitespace}, references, CALLS");
        let Operation::Context(request) = parse(&["context", "brew", "--relations", &relations]).unwrap().operation
        else {
            panic!("expected context request");
        };
        assert_eq!(request.relations, ["CALLS", "REFERENCES"]);
        usage_error(&["context", "brew", "--relations", &whitespace.to_string()], "Choose comma-separated relations");
    }
}

#[test]
fn malformed_forms_and_bounded_overflow_are_usage_errors() {
    for text in ["_1", "1_", "1__0", "²", "1.0", "0x10", "a'b", "a\"b", "a\\b", "a\nb", "a\tb", "a\rb", "a\u{1b}b"] {
        usage_error(
            &["find", "brew", "--limit", text],
            &format!("Invalid value for '--limit': {} is not a valid integer range.", super::integers::repr(text)),
        );
    }
    for (text, number) in [(" -０_１０ ".into(), "-10".into()), ("9".repeat(100), "9".repeat(100))] {
        usage_error(
            &["find", "brew", "--limit", &text],
            &format!("Invalid value for '--limit': {number} is not in the range 1<=x<=100."),
        );
    }
}

fn invalid_values(args: &[&str], option: &str, values: &[&str]) {
    for value in values {
        let mut args = args.to_vec();
        args.extend([option, value]);
        usage_error(&args, &format!("Invalid value for '{option}'"));
    }
}

#[test]
fn numeric_bounds_fail_before_navigation() {
    for (args, option, values) in [
        (vec!["find", "brew"], "--limit", vec!["0", "101", "many"]),
        (vec!["children", "brew"], "--limit", vec!["0", "101", "many"]),
        (vec!["children", "brew"], "--offset", vec!["-1", "many"]),
        (vec!["context", "brew"], "--depth", vec!["-1", "21", "many"]),
        (vec!["path", "brew", "serve"], "--max-depth", vec!["-1", "21", "many"]),
        (vec!["source", "brew"], "--max-lines", vec!["0", "501", "many"]),
        (vec!["source", "brew"], "--start-line", vec!["0", "many"]),
    ] {
        invalid_values(&args, option, &values);
    }
}

#[test]
fn traversal_bounds_fail_before_navigation() {
    for command in [vec!["context", "brew"], vec!["path", "brew", "serve"]] {
        invalid_values(&command, "--relations", &["", " , , ", "CALLS,BOGUS", "CALL"]);
        invalid_values(&command, "--max-nodes", &["0", "301", "many"]);
        invalid_values(&command, "--max-edges", &["0", "1001", "many"]);
    }
}

#[test]
fn repeated_options_use_last_value_and_still_require_arguments() {
    usage_error(&["context", "brew", "--depth", "2", "--depth=-1"], "-1 is not in the range");
    let Operation::Context(request) = parse(&["context", "brew", "--depth", "-1", "--depth=2"]).unwrap().operation
    else {
        panic!("expected context request");
    };
    assert_eq!(request.depth, 2);
    for (args, marker) in [
        (vec!["context", "brew", "--depth", "-1", "--direction", "bad", "--depth=2"], "--direction"),
        (vec!["find", "--limit", "0", "--limit=1"], "<QUERY>"),
    ] {
        let error = match parse(&args) {
            Ok(_) => panic!("expected syntax error"),
            Err(error) => error,
        };
        let error = error.downcast_ref::<clap::Error>().unwrap();
        assert_eq!(error.exit_code(), 2);
        assert!(error.to_string().contains(marker), "{error}");
    }
}
