use crate::support::cli::invoke;
use matis::bootstrap::{migrate_database, repository_service};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

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
    fs::write(root.path().join("action7.test.ts"), "window.setTimeout(() => {}, 0);").unwrap();
    fs::write(root.path().join("!action.js"), "setTimeout(() => {}, 0);").unwrap();
    fs::write(root.path().join(".matisignore"), "/tests/ignored.ts\n").unwrap();
    fs::write(root.path().join("tests/ignored.ts"), "window.setTimeout(() => {}, 0);").unwrap();
    write_config(
        root.path(),
        json!({"forbiddenCalls": [{"files": ["tests/**", "**/action[0-9].test.ts", "!action.js"],
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
    assert!(findings.iter().any(|item| item["subject"] == "window.setTimeout"
        && item["files"] == json!([{"path": "action7.test.ts", "line": 1}])));
    assert_eq!(findings.len(), files.len() + 2, "{findings:?}");
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
            "  void qualified() { com.ph.connection.AllowConnection.open(); }\n",
            "  Object member(Outer outer) { return outer.new AllowConnection(); }\n",
            "}\n",
            "class Outer { class AllowConnection {} }\n",
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
            "types": ["com.ph.connection.AllowConnection", "Timer"], "message": "Use a *fake*."}],
        "forbiddenCalls": [
            {"files": ["**/*.java"], "calls": ["com.ph.connection.AllowConnection.open"]},
            {"files": ["*.ts"], "calls": ["Timer"]}
        ]
    });
    write_config(root.path(), config);
    let check = run(root.path(), "json");
    assert_eq!(check.status.code(), Some(1));
    let findings = source_findings(&check.stdout);
    let locations: Vec<_> = findings.iter().map(|item| (&item["rule"], &item["files"][0])).collect();
    assert_eq!(
        locations,
        [
            (&json!("forbiddenConstructors"), &json!({"path": "Action.java", "line": 3})),
            (&json!("forbiddenConstructors"), &json!({"path": "Action.java", "line": 4})),
            (&json!("forbiddenCalls"), &json!({"path": "Action.java", "line": 8})),
            (&json!("forbiddenCalls"), &json!({"path": "Action.java", "line": 11})),
            (&json!("forbiddenConstructors"), &json!({"path": "action.ts", "line": 1})),
            (&json!("forbiddenCalls"), &json!({"path": "action.ts", "line": 2})),
        ]
    );
    assert_eq!(findings[0]["details"], json!(["Use a *fake*."]));
    let markdown = run(root.path(), "markdown");
    assert_eq!(markdown.status.code(), Some(1));
    let markdown = String::from_utf8_lossy(&markdown.stdout);
    assert!(markdown.contains("`Action.java:3`"));
    assert!(markdown.contains(r"Use a \*fake\*\."), "{markdown}");

    write_config(root.path(), json!({"forbiddenConstructors": []}));
    let clean = run(root.path(), "json");
    assert_eq!(clean.status.code(), Some(0), "{}", String::from_utf8_lossy(&clean.stdout));
    assert!(source_findings(&clean.stdout).is_empty());
}

#[test]
fn check_save_keeps_the_graph_when_a_later_policy_is_invalid() {
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path().join("repo");
    fs::create_dir(&root).unwrap();
    let database = workspace.path().join("graph.sqlite");
    migrate_database(Some(&database)).unwrap();
    let mut check = Command::new(env!("CARGO_BIN_EXE_matis"));
    check.current_dir(&root).env("DATABASE_PATH", &database).args(["check", "--save", "--format", "json"]);

    let source = root.join("clock.py");
    fs::write(&source, "def tick():\n    return 1\ntick()\n").unwrap();
    write_config(&root, json!({"forbiddenCalls": [{"files": ["*.py"], "calls": ["tick"]}]}));
    let result = check.output().unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stderr.is_empty(), "{}", String::from_utf8_lossy(&result.stderr));
    assert_eq!(source_findings(&result.stdout)[0]["subject"], "tick");
    let mut service = repository_service(Some(&database)).unwrap();
    let saved = service.list().unwrap().pop().unwrap();
    let graph = service.get_report_facts(&saved.id).unwrap().unwrap();
    assert!(graph["nodes"].as_array().unwrap().iter().any(|node| node["name"] == "tick"));

    fs::write(&source, "def tock():\n    return 2\n").unwrap();
    write_config(
        &root,
        json!({"forbiddenConstructors": [{"files": ["**"], "types": ["Bad Name"]}], "forbiddenCalls": [
            {"files": ["../outside/**"], "calls": ["setTimeout"], "message": "First\u{2028}Second"},
            {"files": ["**"], "calls": ["setTimeout"], "message": "First\u{2029}Second"}
        ]}),
    );
    let result = check.output().unwrap();
    let diagnostic = String::from_utf8_lossy(&result.stderr);
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert!(diagnostic.contains("forbiddenCalls.0.files.0") && diagnostic.contains("project-relative"), "{diagnostic}");
    assert!(
        diagnostic.contains("forbiddenCalls.0.message") && diagnostic.contains("forbiddenCalls.1.message"),
        "{diagnostic}"
    );
    assert!(diagnostic.contains("single-line"), "{diagnostic}");
    assert!(
        diagnostic.contains("forbiddenConstructors.0.types.0") && diagnostic.contains("exact name without whitespace"),
        "{diagnostic}"
    );
    assert_eq!(service.get_report_facts(&saved.id).unwrap().unwrap(), graph);
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

#[test]
fn architecture_policy_rejects_direct_dependencies_but_allows_indirect_dependencies() {
    let root = tempfile::tempdir().unwrap();
    for directory in ["ui", "service", "storage"] {
        fs::create_dir(root.path().join(directory)).unwrap();
    }
    let ui = root.path().join("ui/page.ts");
    let allowed = concat!(
        "import { read } from '../service/reader';\n",
        "import type { Missing } from '../storage/missing';\n",
        "function local() { return read(); }\n",
        "export const result = local();\n",
    );
    fs::write(&ui, allowed).unwrap();
    fs::write(
        root.path().join("service/reader.ts"),
        "import type { Record } from '../storage/model';\nexport function read(): Record { return { id: 1 }; }\n",
    )
    .unwrap();
    fs::write(root.path().join("storage/model.ts"), "export interface Record { id: number }\n").unwrap();
    write_config(
        root.path(),
        json!({"forbiddenDependencies": [{
            "from": ["ui/**"], "to": ["storage/**/*.ts", "ui/**"],
            "message": "Use the service interface."
        }]}),
    );

    let check = run(root.path(), "json");
    assert_eq!(check.status.code(), Some(0));
    assert!(source_findings(&check.stdout).is_empty());

    fs::write(
        &ui,
        format!(
            "{allowed}import type {{ Record }} from '../storage/model';\n\
             import type {{ Record as Row }} from '../storage/model';\n\
             export const rows: [Record, Row] = [result, result];\n"
        ),
    )
    .unwrap();
    let check = run(root.path(), "json");
    assert_eq!(check.status.code(), Some(1));
    let findings = source_findings(&check.stdout);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], "forbiddenDependencies");
    assert_eq!(findings[0]["files"], json!([{"path": "ui/page.ts", "line": 5}, {"path": "storage/model.ts"}]));
    assert_eq!(findings[0]["details"], json!(["Use the service interface."]));

    write_config(root.path(), json!({}));
    let check = run(root.path(), "json");
    assert_eq!(check.status.code(), Some(0));
    assert!(source_findings(&check.stdout).is_empty());
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
