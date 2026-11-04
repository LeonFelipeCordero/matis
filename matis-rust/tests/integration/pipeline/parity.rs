//! Python reference snapshots restricted to the Rust importer's dedicated language catalog.
use crate::support;
use matis::{
    indexer,
    metrics::{check, config, report},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    process::{Child, Command, ExitStatus, Output, Stdio},
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};
use support::{assert_artifact, assert_json, fixtures, snapshot};

const LANGUAGES: &[&str] = &["java", "kotlin", "go", "rust", "javascript", "typescript", "elixir", "python", "tsx"];

fn graph(language: &str) -> Value {
    snapshot(&format!("coffee/{language}/graph.json"))
}

fn cli_snapshot(language: &str, action: &str, format: &str) -> Value {
    snapshot(&format!("coffee/{language}/cli.json"))[format!("{action}-{format}")].clone()
}

#[test]
fn tsx_artifact() {
    // Other languages check the production artifact before saving it for navigation.
    assert_artifact("tsx", &indexer::analyze_repository(&fixtures().join("tsx")).unwrap());
}

fn report_snapshot(language: &str) {
    // Isolate metrics parity from parser/resolver parity.
    let report = report::build_artifact_report(&graph(language)).unwrap();
    assert_json(&report, &cli_snapshot(language, "report", "json")["output"], "");
    assert_eq!(
        format!("{}\n", report::render_markdown(&report)),
        cli_snapshot(language, "report", "markdown")["output"]
    );
}

fn check_snapshot(language: &str) {
    let report = cli_snapshot(language, "report", "json")["output"].clone();
    let config = config::load_config(&fixtures().join(language)).unwrap();
    let result = check::build_check(&report, &config).unwrap();
    assert_json(&result, &cli_snapshot(language, "check", "json")["output"], "");
    assert_eq!(format!("{}\n", check::render_check(&result)), cli_snapshot(language, "check", "markdown")["output"]);
}

fn invoke(root: &Path, database: &Path, args: &[&str]) -> Output {
    // Files prevent a full stdout pipe from blocking timeout detection.
    let logs = tempfile::tempdir().unwrap();
    let stdout = logs.path().join("stdout");
    let stderr = logs.path().join("stderr");
    let mut child = Command::new(env!("CARGO_BIN_EXE_matis"))
        .args(args)
        .current_dir(root)
        .env("DATABASE_PATH", database)
        .env("XDG_DATA_HOME", database.parent().unwrap().join("data"))
        .stdout(Stdio::from(fs::File::create(&stdout).unwrap()))
        .stderr(Stdio::from(fs::File::create(&stderr).unwrap()))
        .spawn()
        .expect("run native matis");
    let status = wait_for_child(&mut child, &format!("{args:?}"));
    Output { status, stdout: fs::read(stdout).unwrap(), stderr: fs::read(stderr).unwrap() }
}

fn wait_for_child(child: &mut Child, description: &str) -> ExitStatus {
    let seconds = std::env::var("MATIS_PARITY_TIMEOUT_SECS")
        .map(|value| value.parse::<u64>().expect("MATIS_PARITY_TIMEOUT_SECS must be seconds"))
        .unwrap_or(60);
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if started.elapsed() >= Duration::from_secs(seconds) {
            child.kill().expect("kill timed-out native process");
            child.wait().unwrap();
            panic!("native CLI timed out after {seconds}s: {description}");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn cli_snapshots(language: &str) {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("absent/matis.sqlite");
    let root = fixtures().join(language);
    let mut outputs = serde_json::Map::new();
    for (action, format) in [("report", "json"), ("report", "markdown"), ("check", "json"), ("check", "markdown")] {
        let result = invoke(&root, &database, &[action, "--folder", root.to_str().unwrap(), "--format", format]);
        let expected = cli_snapshot(language, action, format);
        assert_eq!(
            result.status.code(),
            expected["exit_code"].as_i64().map(|n| n as i32),
            "{action}-{format}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let stdout = String::from_utf8(result.stdout).unwrap();
        let output = if format == "json" { serde_json::from_str::<Value>(&stdout).unwrap() } else { json!(stdout) };
        let record = json!({"exit_code": result.status.code(), "output": output, "stderr": String::from_utf8(result.stderr).unwrap()});
        let label = format!("{action}-{format}");
        outputs.insert(label, record);
        assert!(!database.exists(), "read-only analysis created a database");
    }
    assert_json(&outputs.into(), &snapshot(&format!("coffee/{language}/cli.json")), "");
}

fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

fn all_rule_ids() -> &'static BTreeSet<String> {
    static RULE_IDS: OnceLock<BTreeSet<String>> = OnceLock::new();
    RULE_IDS.get_or_init(|| {
        LANGUAGES
            .iter()
            .flat_map(|language| {
                snapshot(&format!("mutations/{language}-operators.json"))["mutations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|m| m["ruleId"].as_str().unwrap().to_owned())
                    .collect::<Vec<_>>()
            })
            .collect()
    })
}

fn mutation_snapshot(language: &str, operators: bool) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join(language);
    let source = if operators { fixtures().join("mutations").join(language) } else { fixtures().join(language) };
    copy_tree(&source, &root);
    fs::write(root.join("matis.json"), serde_json::to_vec(&json!({"mutations": {"rules": all_rule_ids()}})).unwrap())
        .unwrap();
    let database = temp.path().join("absent/matis.sqlite");
    let result = invoke(&root, &database, &["mutations", "-f", root.to_str().unwrap()]);
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert!(result.stderr.is_empty());
    let suffix = if operators { "-operators" } else { "" };
    let expected = snapshot(&format!("mutations/{language}{suffix}.json"));
    assert_json(&serde_json::from_slice::<Value>(&result.stdout).unwrap(), &expected, "");
    assert!(!database.exists(), "mutations created a database");
}

macro_rules! language_tests {
    ($($language:ident),+ $(,)?) => {$ (
        mod $language {
            #[test] fn report() { super::report_snapshot(stringify!($language)); }
            #[test] fn check() { super::check_snapshot(stringify!($language)); }
            #[test] fn cli() { super::cli_snapshots(stringify!($language)); }
            #[test] fn mutations() { super::mutation_snapshot(stringify!($language), false); }
            #[test] fn mutation_operators() { super::mutation_snapshot(stringify!($language), true); }
        }
    )+};
}

language_tests!(java, kotlin, go, rust, javascript, typescript, elixir, python, tsx);

#[test]
fn fixtures_are_local_directories() {
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    for name in ["code_samples", "snapshots"] {
        assert!(fs::symlink_metadata(tests.join(name)).unwrap().file_type().is_dir());
    }
}

#[test]
fn comparison_preserves_graph_order_and_fields() {
    let same = json!({"nodes": [{"stableKey":"z"}, {"stableKey":"a"}], "confidence":"INFERRED"});
    assert_json(&same, &same, "");
    for changed in [
        json!({"nodes":[{"stableKey":"a"},{"stableKey":"z"}],"confidence":"INFERRED"}),
        json!({"nodes":[{"stableKey":"z"},{"stableKey":"a"}]}),
    ] {
        assert!(std::panic::catch_unwind(|| assert_json(&changed, &same, "")).is_err());
    }
}
