use super::{config::MutationConfig, digest, runner::MutationRunner};
use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{fs, path::Path};

const ORIGINAL: &[u8] = b"# caf\xc3\xa9\r\nvalue = 1 < 2\r\n";
fn manifest() -> Value {
    let start = ORIGINAL.iter().position(|&ch| ch == b'<').unwrap();
    json!({"id": "mutation", "path": "source.py", "ruleId": "comparison.less-to-greater",
        "sourceHash": digest(ORIGINAL), "span": {"startByte": start, "endByte": start + 1, "startLine": 2},
        "originalBytes": STANDARD.encode(b"<"), "replacementBytes": STANDARD.encode(b">"),
        "originalText": "<", "replacement": ">"})
}
fn config(script: &str) -> MutationConfig {
    MutationConfig::from_value(&json!({"targets": {"unit": {"sources": ["source.py"],
        "command": ["python3", "-c", script], "timeoutSeconds": 1}}}))
    .unwrap()
}
fn records(root: &Path) -> Vec<Value> {
    fs::read_to_string(root.join("results")).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect()
}
fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source.py"), ORIGINAL).unwrap();
    root
}

#[test]
fn independent_edits_logs_progress_flush_and_rerun() -> Result<()> {
    let project = project();
    let root = project.path();
    let output = root.join("results");
    let script = "from pathlib import Path; import sys; p=Path('calls'); assert not p.exists() or len(Path('results').read_text().splitlines()) == 1; p.touch(); sys.stdout.buffer.write(Path('source.py').read_bytes()); sys.exit(5)";
    let mut messages = vec![];
    assert_eq!(
        MutationRunner::new(root, &config(script))?.run_with_progress(
            &[manifest(), manifest()],
            &output,
            &mut |message| messages.push(message.to_owned())
        )?,
        json!({"killed": 2})
    );
    assert_eq!(fs::read(root.join("source.py"))?, ORIGINAL);
    let results = records(root);
    assert_eq!(results.len(), 2);
    for (index, record) in results.iter().enumerate() {
        assert_eq!(record["targets"][0]["exitCode"], 5);
        let expected = String::from_utf8(ORIGINAL.to_vec())?.replace('<', ">");
        assert_eq!(fs::read(root.join(format!("results.logs/{index}-0.log")))?, expected.as_bytes());
    }
    assert!(fs::read_to_string(&output)?.starts_with("{\"manifestId\": \"mutation\", \"path\": "));
    assert_eq!(messages.iter().filter(|line| *line == "Source restored: source.py").count(), 2);
    assert!(messages.iter().any(|line| line.starts_with("Running target unit:")));
    assert!(messages.last().unwrap().starts_with("Completed 2 mutations"));
    assert_eq!(
        MutationRunner::new(root, &config("print('new')"))?.run(&[manifest()], &output)?,
        json!({"survived": 1})
    );
    assert_eq!(records(root).len(), 1);
    assert_eq!(fs::read(root.join("results.logs/0-0.log"))?, b"new\n");
    Ok(())
}

#[test]
fn outcomes_working_directory_and_stale_guards() -> Result<()> {
    let project = project();
    let root = project.path();
    let output = root.join("results");
    fs::create_dir(root.join("tests"))?;
    let mut settings = config("from pathlib import Path; assert Path('../source.py').is_file()");
    settings.targets["unit"].cwd = "tests".into();
    assert_eq!(MutationRunner::new(root, &settings)?.run(&[manifest()], &output)?, json!({"survived": 1}));
    settings.targets["unit"].command = vec!["/matis-missing-command".into()];
    assert_eq!(MutationRunner::new(root, &settings)?.run(&[manifest()], &output)?, json!({"error": 1}));
    assert_eq!(
        records(root)[0]["targets"][0]["error"],
        "[Errno 2] No such file or directory: '/matis-missing-command'"
    );
    for change in [
        json!({"sourceHash": "wrong"}),
        json!({"originalBytes": "Pw=="}),
        json!({"span": {"startByte": -1, "endByte": 1}}),
        json!({"span": {"startByte": 0, "endByte": 0}}),
    ] {
        let mut item = manifest();
        item.as_object_mut().unwrap().extend(change.as_object().unwrap().clone());
        assert_eq!(MutationRunner::new(root, &config("pass"))?.run(&[item], &output)?, json!({"error": 1}));
        assert!(records(root)[0]["error"].as_str().unwrap().starts_with("Stale mutation source:"));
    }
    assert_eq!(fs::read(root.join("source.py"))?, ORIGINAL);
    assert_eq!(
        MutationRunner::new(root, &MutationConfig::default())?.run(&[manifest()], &output)?,
        json!({"unmapped": 1})
    );
    Ok(())
}

#[test]
fn multiple_targets_execute_and_obey_outcome_priority() -> Result<()> {
    let project = project();
    let root = project.path();
    for (first, second, expected) in [("survived", "error", "error"), ("killed", "error", "killed")] {
        let command = |outcome| match outcome {
            "survived" => vec!["python3", "-c", "pass"],
            "killed" => vec!["python3", "-c", "raise SystemExit(1)"],
            _ => vec!["/matis-missing-command"],
        };
        let target = |outcome| json!({"sources": ["source.py"], "command": command(outcome), "timeoutSeconds": 1});
        let settings =
            MutationConfig::from_value(&json!({"targets": {"first": target(first), "second": target(second)}}))?;
        assert_eq!(
            MutationRunner::new(root, &settings)?.run(&[manifest()], &root.join("results"))?,
            json!({expected: 1})
        );
        let records = records(root);
        let executions = records[0]["targets"].as_array().unwrap();
        assert_eq!(executions[0]["outcome"], first);
        assert_eq!(executions[1]["outcome"], second);
        assert_eq!(fs::read(root.join("source.py"))?, ORIGINAL);
    }
    Ok(())
}

#[test]
fn output_and_directory_safety_preserves_files() -> Result<()> {
    let project = project();
    let root = project.path();
    fs::write(root.join("matis.json"), b"{}")?;
    for name in ["source.py", "matis.json"] {
        assert!(MutationRunner::new(root, &config("pass"))?.run(&[manifest()], &root.join(name)).is_err());
    }
    let output = root.join("results");
    fs::write(&output, b"previous")?;
    fs::write(root.join("results.logs"), b"not a directory")?;
    assert!(MutationRunner::new(root, &config("pass"))?.run(&[manifest()], &output).is_err());
    assert_eq!(fs::read(&output)?, b"previous");
    for cwd in ["missing", ".."] {
        let mut settings = config("pass");
        settings.targets["unit"].cwd = cwd.into();
        assert!(MutationRunner::new(root, &settings).is_err());
        assert_eq!(fs::read(&output)?, b"previous");
    }
    assert_eq!(fs::read(root.join("source.py"))?, ORIGINAL);
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_safety_and_path_traversal() -> Result<()> {
    use std::os::unix::fs::symlink;
    let project = project();
    let root = project.path();
    let output = root.join("results");
    let external = tempfile::tempdir()?;
    fs::write(external.path().join("source.py"), ORIGINAL)?;
    fs::write(external.path().join("0-0.log"), b"keep")?;
    symlink(external.path(), root.join("results.logs"))?;
    assert!(MutationRunner::new(root, &config("pass"))?.run(&[manifest()], &output).is_err());
    fs::remove_file(root.join("results.logs"))?;
    fs::create_dir(root.join("results.logs"))?;
    symlink(external.path().join("0-0.log"), root.join("results.logs/0-0.log"))?;
    assert_eq!(MutationRunner::new(root, &config("pass"))?.run(&[manifest()], &output)?, json!({"error": 1}));
    assert_eq!(fs::read(external.path().join("0-0.log"))?, b"keep");
    symlink(external.path().join("source.py"), root.join("link.py"))?;
    fs::create_dir(root.join("nested"))?;
    for path in [root.join("source.py").display().to_string(), "nested/../source.py".into(), "link.py".into()] {
        let mut item = manifest();
        item["path"] = json!(path);
        let mut settings = config("pass");
        settings.targets["unit"].sources = vec!["**".into()];
        assert_eq!(MutationRunner::new(root, &settings)?.run(&[item], &output)?, json!({"error": 1}));
        assert!(records(root)[0]["error"].as_str().unwrap().starts_with("Unsafe mutation path:"));
    }
    assert_eq!(fs::read(root.join("source.py"))?, ORIGINAL);
    Ok(())
}

#[test]
fn panic_restores_source_and_next_run_can_proceed() -> Result<()> {
    let project = project();
    let root = project.path();
    let settings = config("pass");
    let runner = MutationRunner::new(root, &settings)?;
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runner.run_with_progress(&[manifest()], &root.join("results"), &mut |message| {
            if message.starts_with("Running target") {
                panic!("interrupted progress");
            }
        })
    }));
    assert!(interrupted.is_err());
    assert_eq!(fs::read(root.join("source.py"))?, ORIGINAL);
    assert!(fs::read(root.join("results"))?.is_empty());
    assert_eq!(runner.run(&[manifest()], &root.join("results"))?, json!({"survived": 1}));
    Ok(())
}

#[test]
fn restoration_failure_aborts_before_next_mutation() -> Result<()> {
    let project = project();
    let root = project.path();
    let settings = config(
        "from pathlib import Path; p=Path('source.py'); p.unlink(); p.mkdir(); Path('executed').write_text('once')",
    );
    let error = MutationRunner::new(root, &settings)?.run(&[manifest(), manifest()], &root.join("results"));
    assert!(error.is_err());
    assert_eq!(fs::read_to_string(root.join("executed"))?, "once");
    assert!(fs::read(root.join("results"))?.is_empty());
    Ok(())
}

#[test]
fn python_commands_do_not_reuse_baseline_bytecode() -> Result<()> {
    let project = project();
    let root = project.path();
    assert!(
        std::process::Command::new("python3")
            .args(["-m", "py_compile", "source.py"])
            .current_dir(root)
            .status()?
            .success()
    );
    assert_eq!(
        MutationRunner::new(root, &config("import source; assert source.value is False"))?
            .run(&[manifest()], &root.join("results"))?,
        json!({"survived": 1})
    );
    Ok(())
}
