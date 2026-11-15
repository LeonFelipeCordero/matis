//! Regression tests for mutation output preservation and the analysis lock boundary.
use matis::mutations::{config::MutationConfig, digest, runner::MutationRunner};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};

const SOURCE: &[u8] = b"value = 1 < 2\n";
// Forks in other tests can retain directory locks until exec closes inherited descriptors.
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn config() -> MutationConfig {
    MutationConfig::from_value(&json!({"targets": {"unit": {
        "sources": ["source.py"], "command": ["python3", "-c", "print('ok')"]
    }}}))
    .unwrap()
}

fn manifest() -> Value {
    json!({"id": "m", "path": "source.py", "ruleId": "comparison.less-to-greater",
        "sourceHash": digest(SOURCE), "span": {"startByte": 10, "endByte": 11, "startLine": 1},
        "originalBytes": "PA==", "replacementBytes": "Pg==", "originalText": "<", "replacement": ">"})
}

fn command(root: &Path, output: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_matis"));
    command.args(["mutations", "run", "--folder"]).arg(root).args(["--target", "unit", "--output", output]);
    command.env("DATABASE_PATH", root.join("database.sqlite"));
    command
}

fn project_config(root: &Path, rules: Value, script: &str) {
    fs::write(
        root.join("matis.json"),
        serde_json::to_vec(&json!({"mutations": {"rules": rules, "targets": {"unit": {
            "sources": ["source.py"], "command": ["python3", "-c", script], "timeoutSeconds": 5
        }}}}))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn cli_runs_only_selected_targets_even_when_sources_overlap() {
    let _guard = TEST_LOCK.lock().unwrap();
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(root.join("source.py"), SOURCE).unwrap();
    let target = |script: &str| {
        json!({
            "sources": ["source.py"], "command": ["python3", "-c", script], "timeoutSeconds": 5
        })
    };
    fs::write(
        root.join("matis.json"),
        serde_json::to_vec(&json!({"mutations": {
            "rules": ["comparison.less-to-greater"],
            "targets": {
                "unit": target("print('selected')"),
                "unselected": target("raise RuntimeError('unselected target ran')")
            }
        }}))
        .unwrap(),
    )
    .unwrap();
    let output = command(root, "results").output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let results = fs::read_to_string(root.join("results")).unwrap();
    let results: Vec<Value> = results.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["outcome"], "survived");
    assert_eq!(results[0]["targets"].as_array().unwrap().len(), 1);
    assert_eq!(results[0]["targets"][0]["name"], "unit");
    assert_eq!(fs::read(root.join("source.py")).unwrap(), SOURCE);

    let database = root.join("database.sqlite");
    matis::bootstrap::migrate_database(Some(&database)).unwrap();
    let artifact = matis::indexer::analyze_repository(root).unwrap();
    matis::bootstrap::repository_service(Some(&database)).unwrap().save_folder(root, &artifact).unwrap();
    for source in [SOURCE, b"value = 3 < 4\n".as_slice()] {
        fs::write(root.join("source.py"), source).unwrap();
        assert_saved_cli_uses_saved_candidates(root, &database, source);
    }
    let live = command(root, "live-results").output().unwrap();
    assert!(live.status.success(), "{}", String::from_utf8_lossy(&live.stderr));
}

fn assert_saved_cli_uses_saved_candidates(root: &Path, database: &Path, source: &[u8]) {
    for run in [false, true] {
        let mut saved = Command::new(env!("CARGO_BIN_EXE_matis"));
        saved.args(["mutations", "--saved", "--folder"]).arg(root).env("DATABASE_PATH", database);
        if run {
            saved.args(["run", "--target", "unit", "--output", "saved-results"]);
        }
        let output = saved.output().unwrap();
        let error = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.success(), source == SOURCE, "{error}");
        if source != SOURCE {
            assert!(error.contains("Stale or invalid mutation candidate"), "{error}");
        }
        assert_eq!(fs::read(root.join("source.py")).unwrap(), source);
    }
}

#[test]
fn api_protects_sources_without_manifests_or_target_mapping() {
    let _guard = TEST_LOCK.lock().unwrap();
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(root.join("source.py"), SOURCE).unwrap();
    fs::write(root.join("unselected.py"), b"value = 42\n").unwrap();
    let settings = config();
    let runner = MutationRunner::new(root, &settings).unwrap();
    assert!(MutationRunner::new(root, &settings).is_err());
    let mutations = [manifest()];
    for (manifests, name, original) in [
        (&[][..], "source.py", SOURCE),
        (&[], "unselected.py", b"value = 42\n".as_slice()),
        (mutations.as_slice(), "source.py", SOURCE),
        (mutations.as_slice(), "unselected.py", b"value = 42\n".as_slice()),
    ] {
        assert!(runner.run(manifests, &root.join(name)).is_err());
        assert_eq!(fs::read(root.join(name)).unwrap(), original);
    }
    let unmapped = MutationConfig::default();
    drop(runner);
    let runner = MutationRunner::new(root, &unmapped).unwrap();
    assert!(runner.run(&[manifest()], &root.join("unselected.py")).is_err());
    assert_eq!(fs::read(root.join("unselected.py")).unwrap(), b"value = 42\n");
}

#[cfg(unix)]
#[test]
fn ignored_sources_and_resolved_aliases_are_protected_by_api_and_cli() {
    let _guard = TEST_LOCK.lock().unwrap();
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::create_dir(root.join("ignored")).unwrap();
    fs::write(root.join("source.py"), SOURCE).unwrap();
    fs::write(root.join("ignored/source.py"), SOURCE).unwrap();
    fs::write(root.join(".matisignore"), "source.py\nignored/\n").unwrap();
    std::os::unix::fs::symlink("ignored", root.join("alias")).unwrap();
    project_config(root, json!([]), "pass");
    let settings = config();
    for output in ["source.py", "ignored/source.py", "alias/source.py"] {
        let runner = MutationRunner::new(root, &settings).unwrap();
        assert!(runner.run(&[], &root.join(output)).is_err());
        drop(runner);
        let result = command(root, output).output().unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("cannot overwrite source"));
        assert_eq!(fs::read(root.join(output)).unwrap(), SOURCE);
        assert!(!root.join(format!("{output}.logs")).exists());
    }
}

#[test]
fn nested_roots_conflict_in_both_directions_before_cli_analysis() {
    let _guard = TEST_LOCK.lock().unwrap();
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    let sub = root.join("sub");
    fs::create_dir(&sub).unwrap();
    for directory in [root, sub.as_path()] {
        fs::write(directory.join("source.py"), SOURCE).unwrap();
        project_config(directory, json!([]), "pass");
    }
    let settings = config();
    for (held, competing) in [(root, sub.as_path()), (sub.as_path(), root)] {
        let runner = MutationRunner::new(held, &settings).unwrap();
        assert!(MutationRunner::new(competing, &settings).is_err());
        let result = command(competing, "results").output().unwrap();
        assert!(!result.status.success());
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(error.contains("project is locked"), "{error}");
        assert!(!error.contains("Analyzing source files"), "{error}");
        assert!(!competing.join("results").exists());
        assert_eq!(fs::read(sub.join("source.py")).unwrap(), SOURCE);
        drop(runner);
        assert!(MutationRunner::new(competing, &settings).is_ok());
    }
}

#[cfg(unix)]
#[test]
fn symlinked_results_preserve_the_destination_even_without_manifests() {
    let _guard = TEST_LOCK.lock().unwrap();
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(root.join("keep.txt"), b"keep").unwrap();
    std::os::unix::fs::symlink(root.join("keep.txt"), root.join("results")).unwrap();
    let settings = config();
    let runner = MutationRunner::new(root, &settings).unwrap();
    assert!(runner.run(&[], &root.join("results")).is_err());
    assert_eq!(fs::read(root.join("keep.txt")).unwrap(), b"keep");
    assert!(root.join("results").is_symlink());
}

#[test]
fn cli_protects_empty_rules_and_unselected_candidate_free_sources() {
    let _guard = TEST_LOCK.lock().unwrap();
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(root.join("source.py"), SOURCE).unwrap();
    fs::write(root.join("unselected.py"), b"value = 42\n").unwrap();
    for (rules, name, original) in [
        (json!([]), "source.py", SOURCE),
        (json!([]), "unselected.py", b"value = 42\n".as_slice()),
        (json!(["comparison.less-to-greater"]), "source.py", SOURCE),
        (json!(["comparison.less-to-greater"]), "unselected.py", b"value = 42\n".as_slice()),
    ] {
        project_config(root, rules, "pass");
        let output = command(root, name).output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("cannot overwrite source"));
        assert_eq!(fs::read(root.join(name)).unwrap(), original);
    }
}

#[test]
fn hardlinked_results_and_logs_never_truncate_sources_or_config_and_allow_reruns() {
    let _guard = TEST_LOCK.lock().unwrap();
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(root.join("source.py"), SOURCE).unwrap();
    fs::write(root.join("matis.json"), b"{}\n").unwrap();
    fs::create_dir(root.join("results.logs")).unwrap();
    let settings = config();
    let runner = MutationRunner::new(root, &settings).unwrap();
    for (result_link, log_link) in [("source.py", "matis.json"), ("matis.json", "source.py")] {
        let _ = fs::remove_file(root.join("results"));
        let _ = fs::remove_file(root.join("results.logs/0-0.log"));
        fs::hard_link(root.join(result_link), root.join("results")).unwrap();
        fs::hard_link(root.join(log_link), root.join("results.logs/0-0.log")).unwrap();
        assert_preserved_run(&runner, root);
        assert_preserved_run(&runner, root);
    }
}

fn assert_preserved_run(runner: &MutationRunner<'_>, root: &Path) {
    assert_eq!(runner.run(&[manifest()], &root.join("results")).unwrap(), json!({"survived": 1}));
    assert_eq!(fs::read(root.join("source.py")).unwrap(), SOURCE);
    assert_eq!(fs::read(root.join("matis.json")).unwrap(), b"{}\n");
    assert_eq!(fs::read(root.join("results.logs/0-0.log")).unwrap(), b"ok\n");
    let result: Value = serde_json::from_slice(&fs::read(root.join("results")).unwrap()).unwrap();
    assert_eq!(result["outcome"], "survived");
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn await_ready(process: &mut Process, root: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !root.join("ready").exists() {
        assert!(process.0.try_wait().unwrap().is_none(), "runner exited before command");
        assert!(Instant::now() < deadline, "runner readiness timeout");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn sibling_campaigns_run_concurrently() {
    let _guard = TEST_LOCK.lock().unwrap();
    let project = tempfile::tempdir().unwrap();
    let first_root = project.path().join("first");
    let second_root = project.path().join("second");
    for root in [&first_root, &second_root] {
        fs::create_dir(root).unwrap();
        fs::write(root.join("source.py"), SOURCE).unwrap();
    }
    project_config(
        &first_root,
        json!(["comparison.less-to-greater"]),
        "from pathlib import Path; import time; Path('ready').touch()\nwhile not Path('release').exists(): time.sleep(0.01)",
    );
    project_config(&second_root, json!(["comparison.less-to-greater"]), "pass");
    let mut first =
        Process(command(&first_root, "results").stdout(Stdio::null()).stderr(Stdio::inherit()).spawn().unwrap());
    await_ready(&mut first, &first_root);
    let second = command(&second_root, "results").output().unwrap();
    assert!(second.status.success(), "{}", String::from_utf8_lossy(&second.stderr));
    assert!(first.0.try_wait().unwrap().is_none());
    assert_ne!(fs::read(first_root.join("source.py")).unwrap(), SOURCE);
    assert_eq!(fs::read(second_root.join("source.py")).unwrap(), SOURCE);
    fs::write(first_root.join("release"), b"").unwrap();
    assert!(first.0.wait().unwrap().success());
    assert_eq!(fs::read(first_root.join("source.py")).unwrap(), SOURCE);
}

#[test]
fn cross_process_lock_precedes_cli_analysis_and_survives_until_restoration() {
    let _guard = TEST_LOCK.lock().unwrap();
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(root.join("source.py"), SOURCE).unwrap();
    project_config(
        root,
        json!(["comparison.less-to-greater"]),
        "from pathlib import Path; import time; Path('ready').touch()\nwhile not Path('release').exists(): time.sleep(0.01)",
    );
    let mut first = Process(command(root, "first").stdout(Stdio::null()).stderr(Stdio::inherit()).spawn().unwrap());
    await_ready(&mut first, root);
    assert_ne!(fs::read(root.join("source.py")).unwrap(), SOURCE);
    let second = command(root, "second").output().unwrap();
    assert!(!second.status.success());
    let error = String::from_utf8_lossy(&second.stderr);
    assert!(error.contains("project is locked"), "{error}");
    assert!(!error.contains("Analyzing source files"), "analysis ran before locking: {error}");
    assert!(!root.join("second").exists());
    let settings = config();
    assert!(MutationRunner::new(&root.join("."), &settings).is_err());
    assert_ne!(fs::read(root.join("source.py")).unwrap(), SOURCE);
    fs::write(root.join("release"), b"").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = first.0.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < deadline, "runner restoration timeout");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read(root.join("source.py")).unwrap(), SOURCE);
    let rerun = command(root, "second").output().unwrap();
    assert!(rerun.status.success(), "{}", String::from_utf8_lossy(&rerun.stderr));
    let result: Value = serde_json::from_slice(&fs::read(root.join("second")).unwrap()).unwrap();
    assert_eq!(result["outcome"], "survived");
    assert_eq!(fs::read(root.join("source.py")).unwrap(), SOURCE);
}
