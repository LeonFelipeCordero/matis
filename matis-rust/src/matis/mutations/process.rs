//! Interruptible commands in isolated process groups. Always reap the direct child.
use super::config::MutationTarget;
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);
#[cfg(unix)]
extern "C" fn interrupt_handler(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::Relaxed);
}

pub struct Interrupt {
    #[cfg(unix)]
    previous: Vec<(libc::c_int, libc::sigaction)>,
}
impl Interrupt {
    /// Reset interruption state and install Unix SIGINT/SIGTERM handlers.
    ///
    /// The guard restores previous signal dispositions on drop. Installation
    /// errors propagate; callers must serialize guards because state is global.
    /// On non-Unix platforms, this only resets the shared state.
    pub fn new() -> Result<Self> {
        INTERRUPTED.store(false, Ordering::Relaxed);
        let mut interrupt = Self {
            #[cfg(unix)]
            previous: vec![],
        };
        #[cfg(unix)]
        for signal in [libc::SIGINT, libc::SIGTERM] {
            // Only an atomic store runs inside the handler. Restore exact prior dispositions.
            unsafe {
                let mut action: libc::sigaction = std::mem::zeroed();
                let mut previous: libc::sigaction = std::mem::zeroed();
                action.sa_sigaction = interrupt_handler as *const () as usize;
                libc::sigemptyset(&mut action.sa_mask);
                if libc::sigaction(signal, &action, &mut previous) == -1 {
                    return Err(std::io::Error::last_os_error().into());
                }
                interrupt.previous.push((signal, previous));
            }
        }
        Ok(interrupt)
    }
    /// Return an error if a handled signal requested cancellation; otherwise succeed.
    pub fn check(&self) -> Result<()> {
        if INTERRUPTED.load(Ordering::Relaxed) {
            bail!("Mutation run interrupted");
        }
        Ok(())
    }
}
impl Drop for Interrupt {
    /// Restore saved Unix signal dispositions, ignoring restoration failures.
    fn drop(&mut self) {
        #[cfg(unix)]
        for (signal, action) in self.previous.drain(..).rev() {
            unsafe {
                libc::sigaction(signal, &action, std::ptr::null_mut());
            }
        }
    }
}

#[derive(Debug)]
pub struct InvalidCommand;
impl std::fmt::Display for InvalidCommand {
    /// Write the embedded-NUL diagnostic; propagate formatter errors.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("embedded null byte")
    }
}
impl std::error::Error for InvalidCommand {}

struct Process(Child);
impl Drop for Process {
    /// Kill and reap the child, also killing its Unix process group; ignore errors.
    fn drop(&mut self) {
        #[cfg(unix)]
        unsafe {
            libc::kill(-(self.0.id() as i32), libc::SIGKILL);
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Run a trusted, validated target and return its outcome, exit code, log, and duration.
///
/// `paths` contains the working directory and log destination. Replace the log,
/// combine stdout/stderr there, and use a temporary Python bytecode cache. Execute
/// the argument vector directly, without a shell or workspace isolation. Reap the
/// child on exit; on Unix also kill its process group, including remaining children.
/// Timeouts kill the process and return `timeout`; zero/nonzero exits return
/// `survived`/`killed`. Unix signal exits use a negative signal number.
///
/// # Errors
/// Propagate interruption and embedded NUL arguments. Record other execution or
/// I/O failures as an `error` outcome. The caller must supply a nonempty command
/// and a positive integer timeout; values above `u64::MAX` use that maximum.
pub fn run_command(target: &MutationTarget, paths: (&Path, &Path), interrupt: &Interrupt) -> Result<Value> {
    let (cwd, log) = paths;
    let started = Instant::now();
    let mut result = json!({"command": target.command, "log": log, "exitCode": null});
    match execute(target, (cwd, log), interrupt) {
        Ok((outcome, code)) => {
            result["outcome"] = json!(outcome);
            result["exitCode"] = json!(code);
        }
        Err(error) => {
            if error.is::<InvalidCommand>() {
                return Err(error);
            }
            interrupt.check()?;
            result["outcome"] = json!("error");
            result["error"] = json!(error.to_string());
        }
    }
    result["durationSeconds"] = json!(started.elapsed().as_secs_f64());
    Ok(result)
}

fn execute(
    target: &MutationTarget,
    paths: (&Path, &Path),
    interrupt: &Interrupt,
) -> Result<(&'static str, Option<i32>)> {
    let (cwd, log) = paths;
    interrupt.check()?;
    let stream = super::workspace::fresh_file(log)?;
    let cache = tempfile::Builder::new().prefix("matis-pycache-").tempdir()?;
    if target.command.iter().any(|argument| argument.contains('\0')) {
        return Err(InvalidCommand.into());
    }
    let mut command = Command::new(&target.command[0]);
    command
        .args(&target.command[1..])
        .current_dir(cwd)
        .stdout(Stdio::from(stream.try_clone()?))
        .stderr(Stdio::from(stream))
        .env("PYTHONPYCACHEPREFIX", cache.path())
        .env("PYTHONDONTWRITEBYTECODE", "1");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    let mut process =
        Process(command.spawn().map_err(|error| super::formatting::command_error(error, &target.command[0]))?);
    let started = Instant::now();
    loop {
        interrupt.check()?;
        if let Some(status) = process.0.try_wait()? {
            #[cfg(unix)]
            let code = {
                use std::os::unix::process::ExitStatusExt;
                status.code().or_else(|| status.signal().map(|s| -s))
            };
            #[cfg(not(unix))]
            let code = status.code();
            return Ok((if status.success() { "survived" } else { "killed" }, code));
        }
        if started.elapsed() >= Duration::from_secs(target.timeout_seconds.as_u64().unwrap_or(u64::MAX)) {
            return Ok(("timeout", None));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
