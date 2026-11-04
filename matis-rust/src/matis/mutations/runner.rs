//! Sequential independent byte edits against trusted commands, without a baseline.
pub use super::matching::matching_targets;
use super::{
    config::MutationConfig,
    formatting::{command_error, error_suffix, io_error, python_json, repr, shell_join, text},
    process::{Interrupt, run_command},
    workspace::{fresh_file, prepare_output, resolve, source, working_directories},
};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Instant,
};

#[cfg(test)]
#[path = "../../../tests/integration/mutations/project_lock.rs"]
mod project_lock_integration_tests;

static RUN_LOCK: Mutex<()> = Mutex::new(());

struct ProjectLock(File);
impl Drop for ProjectLock {
    fn drop(&mut self) {
        // Closing alone can retain the lock in a descriptor inherited by a concurrent fork.
        let _ = self.0.unlock();
    }
}

pub struct MutationRunner<'a> {
    root: PathBuf,
    config: &'a MutationConfig,
    directories: Vec<PathBuf>,
    _project_locks: Vec<ProjectLock>,
}
impl<'a> MutationRunner<'a> {
    /// Validate configuration, resolve working directories, and lock the project.
    ///
    /// Hold directory locks until this runner is dropped. Shared ancestor locks
    /// allow sibling campaigns but exclude overlapping roots. Create the runner
    /// before candidate analysis and keep it alive through source restoration.
    /// No sources are changed and no commands run during construction.
    ///
    /// # Errors
    /// Reject invalid configuration, missing/escaping working directories, or
    /// competing runners immediately. Propagate filesystem and locking failures.
    pub fn new(root: &Path, config: &'a MutationConfig) -> Result<Self> {
        config.validate()?;
        let root = root.canonicalize()?;
        // Lock the directory inode: no lock-file symlinks or stale lock files to manage.
        let mut project_locks = Vec::new();
        for (index, directory) in root.ancestors().enumerate() {
            let lock = File::open(directory)?;
            // Shared ancestors allow siblings; the exclusive root excludes nested campaigns.
            let result = if index == 0 { lock.try_lock() } else { lock.try_lock_shared() };
            result.context("Mutation project is locked or filesystem locking is unavailable")?;
            project_locks.push(ProjectLock(lock));
        }
        let directories = working_directories(&root, config)?;
        Ok(Self { root, config, directories, _project_locks: project_locks })
    }

    /// Execute independent mutations without progress notifications.
    ///
    /// Return outcome counts using [`Self::run_with_progress`]. Replace results
    /// and reused logs, temporarily edit and restore sources, and execute trusted
    /// commands. The same output, interruption, and restoration errors propagate.
    pub fn run(&self, manifests: &[Value], output: &Path) -> Result<Value> {
        self.run_with_progress(manifests, output, &mut |_| {})
    }

    /// Execute independent manifests sequentially and report progress to the callback.
    ///
    /// Validate each mapped source against its hash and byte span before writing.
    /// Restore original bytes after its targets finish. Unmapped manifests skip
    /// execution. Commands are trusted and may change other files; no baseline or
    /// isolated workspace is provided. Serialize campaigns within this process.
    ///
    /// Replace `output` (relative to the process working directory if not absolute)
    /// and flush one JSONL result per manifest. Replace reused command logs in the
    /// adjacent `.logs` directory. Return counts keyed by outcome; source-validation
    /// and ordinary command failures are recorded per mutation rather than aborting.
    ///
    /// # Errors
    /// Reject symlink/protected output paths. Propagate output and setup failures,
    /// embedded-NUL command errors, interruption, and source-restoration failures.
    /// On Unix, SIGINT/SIGTERM stop commands and trigger restoration before return.
    /// SIGKILL, machine failure, or restoration I/O errors can leave sources modified.
    pub fn run_with_progress(
        &self,
        manifests: &[Value],
        output: &Path,
        progress: &mut dyn FnMut(&str),
    ) -> Result<Value> {
        let _lock = RUN_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let started = Instant::now();
        if output.is_symlink() {
            bail!("Mutation results must not be a symlink");
        }
        let output = resolve(output)?;
        let logs = prepare_output(&self.root, manifests, &output)?;
        let mapped =
            manifests.iter().filter(|item| !matching_targets(text(item, "path"), self.config).is_empty()).count();
        progress(&format!(
            "Planned {} mutations: {mapped} mapped, {} unmapped",
            manifests.len(),
            manifests.len() - mapped
        ));
        progress(&format!("Results: {}; command logs: {}", output.display(), logs.display()));
        let interrupt = Interrupt::new()?;
        let mut campaign = Campaign { runner: self, logs, interrupt: &interrupt, progress };
        let mut stream = fresh_file(&output)?;
        let mut counts = json!({});
        for (ordinal, manifest) in manifests.iter().enumerate() {
            interrupt.check()?;
            let label = format!(
                "[{}/{}] {}:{} {}",
                ordinal + 1,
                manifests.len(),
                text(manifest, "path"),
                manifest["span"]["startLine"],
                text(manifest, "ruleId")
            );
            (campaign.progress)(&format!(
                "{label}: {} -> {}",
                repr(text(manifest, "originalText")),
                repr(text(manifest, "replacement"))
            ));
            let result = campaign.execute(manifest, ordinal)?;
            writeln!(stream, "{}", python_json(&result)).map_err(io_error)?;
            stream.flush().map_err(io_error)?;
            let outcome = result["outcome"].as_str().unwrap();
            counts[outcome] = json!(counts[outcome].as_u64().unwrap_or(0) + 1);
            (campaign.progress)(&format!("{label}: {outcome}{}", error_suffix(&result)));
        }
        (campaign.progress)(&format!(
            "Completed {} mutations in {:.2}s: {}",
            manifests.len(),
            started.elapsed().as_secs_f64(),
            python_json(&counts)
        ));
        Ok(counts)
    }
}

struct Campaign<'a, 'b> {
    runner: &'a MutationRunner<'a>,
    logs: PathBuf,
    interrupt: &'a Interrupt,
    progress: &'b mut dyn FnMut(&str),
}
impl Campaign<'_, '_> {
    fn execute(&mut self, manifest: &Value, ordinal: usize) -> Result<Value> {
        let mut result = json!({"manifestId": manifest["id"], "path": manifest["path"], "ruleId": manifest["ruleId"],
            "outcome": "unmapped", "targets": []});
        if matching_targets(text(manifest, "path"), self.runner.config).is_empty() {
            return Ok(result);
        }
        let mut source = match source(&self.runner.root, manifest) {
            Ok(source) => source,
            Err(error) => {
                set_error(&mut result, error);
                return Ok(result);
            }
        };
        let execution = (|| -> Result<()> {
            self.interrupt.check()?;
            fs::write(&source.path, &source.modified)
                .map_err(|error| command_error(error, &source.path.to_string_lossy()))?;
            self.execute_targets(manifest, ordinal, &mut result)
        })();
        source.restore()?;
        (self.progress)(&format!("Source restored: {}", text(manifest, "path")));
        self.interrupt.check()?;
        if let Err(error) = execution {
            if error.is::<super::process::InvalidCommand>() {
                return Err(error);
            }
            set_error(&mut result, error);
            return Ok(result);
        }
        result["outcome"] = json!(combined_outcome(result["targets"].as_array().unwrap()));
        Ok(result)
    }

    fn execute_targets(&mut self, manifest: &Value, ordinal: usize, result: &mut Value) -> Result<()> {
        let targets = matching_targets(text(manifest, "path"), self.runner.config);
        for (index, (name, _)) in targets.iter().enumerate() {
            let log = self.logs.join(format!("{ordinal}-{index}.log"));
            let execution = self.run_target(name, &log)?;
            result["targets"].as_array_mut().unwrap().push(execution);
        }
        Ok(())
    }

    fn run_target(&mut self, name: &str, log: &Path) -> Result<Value> {
        let position = self.runner.config.targets.get_index_of(name).unwrap();
        let target = &self.runner.config.targets[name];
        let cwd = &self.runner.directories[position];
        (self.progress)(&format!(
            "Running target {name}: {} (cwd={}, timeout={}s, log={})",
            shell_join(&target.command),
            cwd.display(),
            target.timeout_seconds,
            log.display()
        ));
        let execution = run_command(target, (cwd, log), self.interrupt)?;
        let exit = execution["exitCode"].as_i64().map(|n| n.to_string()).unwrap_or_else(|| "None".into());
        (self.progress)(&format!(
            "Target {name}: {} (exit={exit}, {:.2}s){}",
            text(&execution, "outcome"),
            execution["durationSeconds"].as_f64().unwrap(),
            error_suffix(&execution)
        ));
        let mut named = json!({"name": name});
        named.as_object_mut().unwrap().extend(execution.as_object().unwrap().clone());
        Ok(named)
    }
}
pub(super) fn combined_outcome(targets: &[Value]) -> &'static str {
    ["killed", "error", "timeout", "survived"]
        .into_iter()
        .find(|outcome| targets.iter().any(|item| item["outcome"] == *outcome))
        .unwrap()
}
fn set_error(result: &mut Value, error: anyhow::Error) {
    result["outcome"] = json!("error");
    result["error"] = json!(error.to_string());
}
