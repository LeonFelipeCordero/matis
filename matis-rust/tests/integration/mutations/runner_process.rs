//! Real subprocess tests keep signals isolated from the Rust test harness.
use super::{config::MutationConfig, digest, runner::MutationRunner};
use anyhow::Result;
use serde_json::json;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn settings(script: &str) -> MutationConfig {
    MutationConfig::from_value(&json!({"targets": {"unit": {"sources": ["source.py"],
        "command": ["python3", "-c", script], "timeoutSeconds": 1}}}))
    .unwrap()
}
fn item() -> serde_json::Value {
    json!({"id": "m", "path": "source.py", "ruleId": "comparison.less-to-greater",
        "sourceHash": digest(b"1 < 2\n"), "span": {"startByte": 2, "endByte": 3, "startLine": 1},
        "originalBytes": "PA==", "replacementBytes": "Pg==", "originalText": "<", "replacement": ">"})
}
#[cfg(unix)]
fn stopped(pid: i32) -> bool {
    let state = Command::new("ps").args(["-o", "stat=", "-p", &pid.to_string()]).output().unwrap();
    let state = String::from_utf8_lossy(&state.stdout);
    state.trim().is_empty() || state.trim().starts_with('Z')
}
fn await_file(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(Instant::now() < deadline, "Readiness timeout: {}", path.display());
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[cfg(unix)]
fn await_stopped(pid: i32) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !stopped(pid) {
        if Instant::now() > deadline {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
            panic!("Child {pid} still executing");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn await_exit(child: &mut std::process::Child) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait()? {
            assert!(status.success());
            return Ok(());
        }
        if Instant::now() > deadline {
            child.kill()?;
            panic!("Signal restoration timed out");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[cfg(unix)]
#[test]
fn process_groups_kill_children_after_timeout_and_parent_exit() -> Result<()> {
    for parent in ["import time; time.sleep(60)", "pass"] {
        let project = tempfile::tempdir()?;
        let root = project.path();
        fs::write(root.join("source.py"), b"1 < 2\n")?;
        let child =
            "from pathlib import Path; import os, time; Path('child.pid').write_text(str(os.getpid())); time.sleep(60)";
        let script = format!(
            "import subprocess, sys, time; from pathlib import Path; subprocess.Popen([sys.executable, '-c', {child:?}]);\nwhile not Path('child.pid').exists(): time.sleep(0.01)\n{parent}"
        );
        let result = MutationRunner::new(root, &settings(&script))?.run(&[item()], &root.join("results"))?;
        assert_eq!(result, if parent == "pass" { json!({"survived": 1}) } else { json!({"timeout": 1}) });
        let pid: i32 = fs::read_to_string(root.join("child.pid"))?.parse()?;
        await_stopped(pid);
        assert_eq!(fs::read(root.join("source.py"))?, b"1 < 2\n");
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn signals_restore_source_and_previous_handlers() -> Result<()> {
    if let Some(root) = std::env::var_os("MATIS_RUNNER_SIGNAL_CHILD") {
        let root = Path::new(&root);
        let mut before: libc::sigaction = unsafe { std::mem::zeroed() };
        let signal: i32 = std::env::var("MATIS_RUNNER_SIGNAL")?.parse()?;
        unsafe {
            libc::sigaction(signal, std::ptr::null(), &mut before);
        }
        let mut config = settings(
            "from pathlib import Path; import os, time; Path('ready').write_text(str(os.getpid())); time.sleep(60)",
        );
        config.targets["unit"].timeout_seconds = 60.into();
        let error = MutationRunner::new(root, &config)?.run(&[item()], &root.join("results")).unwrap_err();
        assert!(error.to_string().contains("interrupted"));
        let mut after: libc::sigaction = unsafe { std::mem::zeroed() };
        unsafe {
            libc::sigaction(signal, std::ptr::null(), &mut after);
        }
        assert_eq!(before.sa_sigaction, after.sa_sigaction);
        assert_eq!(fs::read(root.join("source.py"))?, b"1 < 2\n");
        return Ok(());
    }
    for signal in [libc::SIGINT, libc::SIGTERM] {
        let project = tempfile::tempdir()?;
        let root = project.path();
        fs::write(root.join("source.py"), b"1 < 2\n")?;
        let mut child = Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "mutations::runner_process_integration_tests::signals_restore_source_and_previous_handlers",
                "--nocapture",
            ])
            .env("MATIS_RUNNER_SIGNAL_CHILD", root)
            .env("MATIS_RUNNER_SIGNAL", signal.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()?;
        await_file(&root.join("ready"));
        assert_ne!(fs::read(root.join("source.py"))?, b"1 < 2\n");
        unsafe {
            libc::kill(child.id() as i32, signal);
        }
        await_exit(&mut child)?;
        let pid: i32 = fs::read_to_string(root.join("ready"))?.parse()?;
        assert!(stopped(pid));
        assert_eq!(fs::read(root.join("source.py"))?, b"1 < 2\n");
        assert!(fs::read(root.join("results"))?.is_empty());
    }
    Ok(())
}
