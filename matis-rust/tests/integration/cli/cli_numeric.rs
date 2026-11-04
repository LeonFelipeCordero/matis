//! Actual native CLI regressions. Expected forms and errors were paired with Click's CLI oracle.
use crate::support::project;
use project::Project;
use serde_json::{Value, json};
use std::process::{Command, Output};

impl Project {
    fn invoke(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_matis"))
            .current_dir(&self.root)
            .env("DATABASE_PATH", &self.database)
            .env("COLUMNS", "80")
            .args(["graph", "--project", &self.saved.repository_id])
            .args(args)
            .output()
            .unwrap()
    }
}
fn payload(output: Output) -> Value {
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn python_decimal_lexical_forms_reach_actual_navigation() {
    let fixture = Project::default();
    // Lexical variants run against GraphOptions in unit tests; retain service handoff checks.
    for (text, number) in [(" 1_0 ", 10), ("𝟸", 2)] {
        let result = payload(fixture.invoke(&["find", "brew", "--limit", text]));
        assert_eq!(result["filters"]["limit"], number);
        assert!(result["nodes"].as_array().unwrap().iter().any(|node| node["stableKey"] == fixture.key("brew")));
    }
    assert_eq!(payload(fixture.invoke(&["context", fixture.key("brew"), "--depth", "٢"]))["filters"]["depth"], 2);
    let first = payload(fixture.invoke(&["children", "file:brew.py", "--offset", "０"]));
    assert_eq!(first["nodes"][0]["stableKey"], fixture.key("brew"));
    assert_eq!(payload(fixture.invoke(&["children", "file:brew.py", "--offset", " +０_１ "]))["nodes"], json!([]));
}
#[test]
fn relation_lists_strip_python_whitespace_and_normalize_each_part() {
    let fixture = Project::default();
    let relations = "\u{1c}calls\u{1c}, \u{1c}, references, CALLS";
    let result = payload(fixture.invoke(&["context", fixture.key("brew"), "--relations", relations]));
    assert_eq!(result["filters"]["relations"], json!(["CALLS", "REFERENCES"]));
    let empty = fixture.invoke(&["context", fixture.key("brew"), "--relations", "\u{1c}"]);
    assert_eq!(empty.status.code(), Some(2));
    assert!(String::from_utf8(empty.stderr).unwrap().contains("Choose comma-separated relations"));
}
#[test]
fn huge_source_start_lines_stay_exact_in_json_and_text() {
    let fixture = Project::default();
    for decimal in ["18446744073709551616".into(), "1234567890".repeat(50)] {
        check_huge_source(&fixture, &decimal);
    }
}
fn check_huge_source(fixture: &Project, decimal: &str) {
    let result = payload(fixture.invoke(&["source", fixture.key("brew"), "--start-line", decimal]));
    assert_eq!(result["filters"]["startLine"].to_string(), decimal);
    assert_eq!(result["source"]["startLine"].to_string(), decimal);
    assert_eq!(result["source"]["endLine"], 2);
    assert_eq!(result["source"]["lines"], json!([]));
    assert_eq!(result["source"]["nextStartLine"], Value::Null);
    assert_eq!(result["source"]["truncated"], false);
    let text = fixture.invoke(&["source", fixture.key("brew"), "--start-line", decimal, "--format", "text"]);
    assert!(text.status.success());
    assert!(text.stderr.is_empty());
    assert_eq!(String::from_utf8(text.stdout).unwrap().matches(&format!("\"startLine\": {decimal}")).count(), 2);
}
#[test]
fn sqlite_offset_overflow_fails_without_wrapping_to_another_page() {
    let fixture = Project::default();
    for value in ["9223372036854775808".into(), "18446744073709551616".into(), "9".repeat(100)] {
        let result = fixture.invoke(&["children", "file:brew.py", "--offset", &value]);
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
        assert_eq!(
            serde_json::from_slice::<Value>(&result.stderr).unwrap(),
            json!({"error": "Python int too large to convert to SQLite INTEGER"})
        );
    }
}
