use crate::support::cli::invoke;
use serde_json::{Value, json};
use std::fs;

#[test]
fn file_fan_checks_use_real_imports_and_exclusions_preserve_peer_counts() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("a.py"), "import b\nimport c\n").unwrap();
    fs::write(root.path().join("b.py"), "import c\n").unwrap();
    fs::write(root.path().join("c.py"), "VALUE = 1\n").unwrap();
    for patterns in [json!([]), json!(["/b.py"]), json!(["b*.py"])] {
        fs::write(
            root.path().join("matis.json"),
            json!({"maxFanIn": 0, "maxFanOut": 0, "maxFileLines": 0, "fanExclude": patterns}).to_string(),
        )
        .unwrap();
        let output = invoke(&["check", "--folder", root.path().to_str().unwrap(), "--format", "json"]);
        assert_eq!(output.status.code(), Some(1), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.stderr.is_empty());
        let check: Value = serde_json::from_slice(&output.stdout).unwrap();
        let findings = check["findings"].as_array().unwrap();
        let fan: Vec<_> = findings
            .iter()
            .filter(|finding| matches!(finding["rule"].as_str(), Some("maxFanIn" | "maxFanOut")))
            .map(|finding| {
                (
                    finding["subject"].as_str().unwrap(),
                    finding["rule"].as_str().unwrap(),
                    finding["actual"].as_u64().unwrap(),
                )
            })
            .collect();
        let mut expected = vec![("a.py", "maxFanOut", 2)];
        if patterns.as_array().unwrap().is_empty() {
            expected.extend([("b.py", "maxFanIn", 1), ("b.py", "maxFanOut", 1)]);
        }
        expected.push(("c.py", "maxFanIn", 2));
        assert_eq!(fan, expected, "patterns: {patterns}");
        let lines: Vec<_> = findings
            .iter()
            .filter(|finding| finding["rule"] == "maxFileLines")
            .map(|finding| finding["subject"].as_str().unwrap())
            .collect();
        assert_eq!(lines, ["a.py", "b.py", "c.py"], "exclusions must suppress only fan rules");
        assert_eq!(findings.len(), fan.len() + lines.len());
    }
}
