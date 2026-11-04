use super::options::{Options, json_dump};
use serde_json::json;

fn options(command: &str, args: &[&str]) -> Result<Options, clap::Error> {
    let matches = super::schema::cli()
        .try_get_matches_from(std::iter::once("matis").chain(command.split_whitespace()).chain(args.iter().copied()))?;
    let mut selected = &matches;
    for name in command.split_whitespace() {
        selected = selected.subcommand_matches(name).unwrap();
    }
    Ok(Options::from_matches(command, selected))
}

#[test]
fn folder_aliases_attached_values_and_literal_arguments() {
    for option in ["-f", "--f", "--folder"] {
        assert_eq!(options("index", &[option, "folder"]).unwrap().get("--folder", "."), "folder");
        assert!(options("index", &[option]).is_err());
    }
    assert_eq!(options("index", &["-ffolder"]).unwrap().get("--folder", "."), "folder");
    assert_eq!(options("graph show", &["--", "--help"]).unwrap().positional, vec!["--help"]);
    let attached = options("report", &["-f/tmp/project", "--format=json"]).unwrap();
    assert_eq!(attached.get("--folder", "."), "/tmp/project");
    assert_eq!(attached.get("--format", "markdown"), "json");
}
#[test]
fn help_tokens_used_as_values_do_not_request_help() {
    assert_eq!(options("report", &["--folder", "--help"]).unwrap().get("--folder", "."), "--help");
    assert_eq!(options("report", &["--format", "--help"]).err().unwrap().kind(), clap::error::ErrorKind::InvalidValue);
    assert_eq!(options("graph find", &["--", "--help"]).unwrap().positional, vec!["--help"]);
    assert_eq!(options("graph find", &["--help"]).err().unwrap().kind(), clap::error::ErrorKind::DisplayHelp);
}
#[test]
fn positionals_flags_defaults_and_repeated_values_reach_handlers() {
    let path = options("graph path", &["from", "to", "--direction", "both"]).unwrap();
    assert_eq!(path.positional, ["from", "to"]);
    assert_eq!(path.get("--max-depth", ""), "6");
    assert_eq!(path.get("--direction", ""), "both");
    let source = options("graph source", &["one", "two", "--format", "text"]).unwrap();
    assert_eq!(source.positional, ["one", "two"]);
    assert_eq!(source.get("--format", ""), "text");
    let report = options("report", &["--save", "--folder", "one", "--folder", "two"]).unwrap();
    assert!(report.has("--save"));
    assert_eq!(report.get("--folder", ""), "two");
    assert!(!options("report", &[]).unwrap().has("--save"));
}
#[test]
fn graph_numeric_bounds_and_choice_errors() {
    let parsed = options("graph find", &["brew", "--limit", "many"]).unwrap();
    assert_eq!(
        parsed.number("--limit", 10, (1, 100)).unwrap_err().to_string(),
        "Invalid value for '--limit': 'many' is not a valid integer range."
    );
    assert!(options("graph source", &["brew", "--format", "JSON"]).is_err());
}
#[test]
fn numeric_options_accept_python_decimal_forms() {
    for (text, expected) in [(" 1_0 ", 10), ("\t+10\n", 10), ("１００", 100), ("٢", 2), ("𝟡", 9), ("१_०", 10)]
    {
        let parsed = options("graph find", &["brew", "--limit", text]).unwrap();
        assert_eq!(parsed.number("--limit", 10, (1, 100)).unwrap(), expected);
    }
}
#[test]
fn range_errors_show_canonical_numbers_without_machine_overflow() {
    for (text, number) in
        [(" -０_１０ ".to_owned(), "-10".to_owned()), (format!("+00{}", "9".repeat(100)), "9".repeat(100))]
    {
        let parsed = options("graph find", &["brew", "--limit", &text]).unwrap();
        assert_eq!(
            parsed.number("--limit", 10, (1, 100)).unwrap_err().to_string(),
            format!("Invalid value for '--limit': {number} is not in the range 1<=x<=100.")
        );
    }
    let huge = "1234567890".repeat(50);
    assert_eq!(
        options("graph source", &["brew", "--start-line", &huge])
            .unwrap()
            .integer("--start-line", 1, 1)
            .unwrap()
            .to_string(),
        huge
    );
}
#[test]
fn compact_json_uses_python_spaces_without_changing_strings() {
    let value = json!({"quoted": "\"a,b:c\\\"", "values": [1, true, null, "café"]});
    assert_eq!(serde_json::from_str::<serde_json::Value>(&json_dump(&value)).unwrap(), value);
    assert!(json_dump(&value).contains("[1, true, null, \"café\"]"));
}
