use crate::support::cli::invoke;
use serde_json::{Value, json};
use std::{fs, path::Path};

#[test]
fn source_policies_check_calls_in_every_discovered_language() {
    let root = tempfile::tempdir().unwrap();
    let files = [
        ("action.py", "# clock.sleep()\nclock.sleep()", "clock.sleep"),
        ("action.js", "// setTimeout()\nsetTimeout(() => {}, 0);", "setTimeout"),
        ("action.jsx", "const label = <Timer />;\nReact /* comment */ .useEffect(() => {});", "React.useEffect"),
        (
            "action.test.ts",
            "const label = 'window.setTimeout()';\nwindow.setTimeout(() => {}, 0);",
            "window.setTimeout",
        ),
        ("action.tsx", "const label = <Widget />;\nWidget.tick();", "Widget.tick"),
        (
            "Action.java",
            "import com.ph.connection.AllowConnection;\nclass Action { void run() { AllowConnection.open(); } }",
            "com.ph.connection.AllowConnection.open",
        ),
        ("action.kt", "// Thread.sleep(1)\nfun run() { Thread.sleep(1) }", "Thread.sleep"),
        ("action.go", "package action\nfunc run() { clock.Sleep(1) }", "clock.Sleep"),
        ("action.rs", "// clock::sleep(1)\nfn run() { clock::sleep(1); }", "clock::sleep"),
        ("action.ex", "# Process.sleep(1)\nProcess.sleep(1)", "Process.sleep"),
    ];
    fs::create_dir(root.path().join("tests")).unwrap();
    for (file, source, _) in files {
        fs::write(root.path().join("tests").join(file), source).unwrap();
    }
    fs::write(root.path().join("action.ts"), "window.setTimeout(() => {}, 0);").unwrap();
    fs::write(root.path().join("!action.js"), "setTimeout(() => {}, 0);").unwrap();
    fs::write(root.path().join(".matisignore"), "/tests/ignored.ts\n").unwrap();
    fs::write(root.path().join("tests/ignored.ts"), "window.setTimeout(() => {}, 0);").unwrap();
    write_config(
        root.path(),
        json!({"forbiddenCalls": [{"files": ["tests/**", "**/*.test.ts", "!action.js"],
        "calls": files.map(|(_, _, call)| call).into_iter().chain(["Timer", "Widget"]).collect::<Vec<_>>(),
        "message": "Use the project clock."}]}),
    );

    let check = run(root.path(), "json");
    assert_eq!(check.status.code(), Some(1));
    let findings = source_findings(&check.stdout);
    for (file, _, name) in files {
        let path = format!("tests/{file}");
        assert!(
            findings.iter().any(|item| item["subject"] == name
                && item["files"] == json!([{"path": path, "line": 2}])
                && item["details"] == json!(["Use the project clock."])),
            "Missing {file}: {findings:?}"
        );
    }
    assert!(
        findings
            .iter()
            .any(|item| item["subject"] == "setTimeout" && item["files"] == json!([{"path": "!action.js", "line": 1}]))
    );
    assert_eq!(findings.len(), files.len() + 1, "{findings:?}");
}

#[test]
fn constructor_policy_checks_explicit_java_type_and_not_references() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("Action.java"),
        concat!(
            "import com.ph.connection.AllowConnection;\n",
            "class Action {\n",
            "  Object first() { return new AllowConnection(); }\n",
            "  Object second() { return new com.ph.connection.AllowConnection(); }\n",
            "  Object other() { return new other.AllowConnection(); }\n",
            "  Object ref() { return (java.util.function.Supplier<Object>) AllowConnection::new; }\n",
            "  Object local() { class AllowConnection {} return new AllowConnection(); }\n",
            "  void run() { AllowConnection.open(); }\n",
            "  void shadow(Safe AllowConnection) { AllowConnection.open(); }\n",
            "  java.util.function.Consumer<Safe> use = AllowConnection -> AllowConnection.open();\n",
            "}\n",
        ),
    )
    .unwrap();
    fs::write(
        root.path().join("Other.java"),
        "import other.AllowConnection;\nclass Other { Object run() { return new AllowConnection(); } }",
    )
    .unwrap();
    fs::write(
        root.path().join("Shadow.java"),
        concat!(
            "import com.ph.connection.AllowConnection;\n",
            "class Shadow { static class AllowConnection {} Object run() { return new AllowConnection(); } }",
        ),
    )
    .unwrap();
    fs::write(root.path().join("action.ts"), "const timer = new Timer();\nTimer();").unwrap();
    let config = json!({
        "forbiddenConstructors": [{"files": ["**/*.java", "**/*.ts"],
            "types": ["com.ph.connection.AllowConnection", "Timer"], "message": "Use a fake."}],
        "forbiddenCalls": [{"files": ["**/*.java"], "calls": ["com.ph.connection.AllowConnection.open"]}]
    });
    write_config(root.path(), config);
    let check = run(root.path(), "json");
    assert_eq!(check.status.code(), Some(1));
    let locations: Vec<_> = source_findings(&check.stdout).into_iter().map(|item| item["files"][0].clone()).collect();
    assert_eq!(
        locations,
        [
            json!({"path": "Action.java", "line": 3}),
            json!({"path": "Action.java", "line": 4}),
            json!({"path": "Action.java", "line": 8}),
            json!({"path": "action.ts", "line": 1})
        ]
    );
    let markdown = run(root.path(), "markdown");
    assert!(String::from_utf8_lossy(&markdown.stdout).contains("`Action.java:3`"));

    write_config(root.path(), json!({"forbiddenConstructors": [{"files": ["src/**"], "types": ["Timer"]}]}));
    let clean = run(root.path(), "json");
    assert_eq!(clean.status.code(), Some(0), "{}", String::from_utf8_lossy(&clean.stdout));
}

#[test]
fn invalid_policy_is_rejected_before_analysis() {
    let root = tempfile::tempdir().unwrap();
    write_config(
        root.path(),
        json!({"forbiddenCalls": [
            {"files": ["../outside/**"], "calls": ["setTimeout"], "message": "First\u{2028}Second"},
            {"files": ["**"], "calls": ["setTimeout"], "message": "First\u{2029}Second"}
        ]}),
    );
    let result = invoke(&["check", "--folder", root.path().to_str().unwrap()]);
    let diagnostic = String::from_utf8_lossy(&result.stderr);
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert!(diagnostic.contains("forbiddenCalls.0.files.0") && diagnostic.contains("project-relative"), "{diagnostic}");
    assert!(
        diagnostic.contains("forbiddenCalls.0.message") && diagnostic.contains("forbiddenCalls.1.message"),
        "{diagnostic}"
    );
    assert!(diagnostic.contains("single-line"), "{diagnostic}");
}

#[test]
fn elixir_policy_distinguishes_calls_from_specs_and_captures() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("timer.ex"),
        concat!(
            "defmodule Timer do\n",
            "  @spec tick(integer()) :: atom()\n",
            "  def tick(n), do: n\n",
            "  @callback tick(integer()) :: atom()\n",
            "  @type duration :: integer()\n",
            "  def run do\n",
            "    ref = &Timer.tick/1\n",
            "    fun = &Timer.tick(&1)\n",
            "    tick(1)\n",
            "  end\n",
            "end\n",
        ),
    )
    .unwrap();
    write_config(
        root.path(),
        json!({"forbiddenCalls": [{"files": ["*.ex"], "calls": ["tick", "Timer.tick", "integer", "atom"]}]}),
    );
    let check = run(root.path(), "json");
    assert_eq!(check.status.code(), Some(1));
    let findings: Vec<_> = source_findings(&check.stdout)
        .into_iter()
        .map(|item| (item["subject"].clone(), item["files"][0].clone()))
        .collect();
    assert_eq!(
        findings,
        [
            (json!("Timer.tick"), json!({"path": "timer.ex", "line": 8})),
            (json!("tick"), json!({"path": "timer.ex", "line": 9}))
        ]
    );
}

#[test]
fn java_record_and_resource_shadows_do_not_hide_imports_outside_their_scope() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("Timer.java"),
        concat!(
            "import demo.Clock;\n",
            "record Timer(Safe Clock) { void run() { Clock.tick(); } }\n",
            "class Later {\n",
            "  void run() throws Exception {\n",
            "    try (Safe Clock = acquire()) { Clock.tick(); }\n",
            "    Clock.tick();\n",
            "  }\n",
            "  Safe acquire() { return null; }\n",
            "}\n",
        ),
    )
    .unwrap();
    write_config(root.path(), json!({"forbiddenCalls": [{"files": ["*.java"], "calls": ["demo.Clock.tick"]}]}));
    let check = run(root.path(), "json");
    assert_eq!(check.status.code(), Some(1));
    let findings = source_findings(&check.stdout);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["files"], json!([{"path": "Timer.java", "line": 6}]));
}

fn write_config(root: &Path, mut config: Value) {
    for limit in matis::metrics::config::LIMITS {
        config[limit] = Value::Null;
    }
    fs::write(root.join("matis.json"), config.to_string()).unwrap();
}

fn run(root: &Path, format: &str) -> std::process::Output {
    let output = invoke(&["check", "--folder", root.to_str().unwrap(), "--format", format]);
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    output
}

fn source_findings(output: &[u8]) -> Vec<Value> {
    let check: Value = serde_json::from_slice(output).unwrap();
    check["findings"].as_array().unwrap().clone()
}
