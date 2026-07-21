//! Daemon lifecycle control from the CLI: start, stop, status, and "ensure
//! running". The daemon runs inside a detached `trace` child process so releases
//! ship a single binary.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use trace_core::paths;
use trace_daemon::DaemonState;

use crate::client::Client;

/// Hidden subcommand used to run the server in the foreground inside the child.
pub const SERVE_ARG: &str = "__serve";

/// Returns the port of a healthy running daemon, if any.
pub fn running_port() -> Option<u16> {
    let state = DaemonState::read().ok().flatten()?;
    let client = Client::new(state.port);
    client.health().ok().map(|_| state.port)
}

/// Ensure a daemon is running and return its port, starting one if needed.
pub fn ensure_running() -> Result<u16> {
    if let Some(port) = running_port() {
        return Ok(port);
    }
    start()?;
    // Wait for the child to write daemon.json and answer health checks.
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Some(port) = running_port() {
            return Ok(port);
        }
        sleep(Duration::from_millis(150));
    }
    Err(anyhow!("daemon did not become ready within 10s"))
}

/// Spawn a detached daemon child process. No-op if one is already running.
pub fn start() -> Result<()> {
    if running_port().is_some() {
        return Ok(());
    }
    paths::ensure_global_dir()?;
    let log_path = paths::global_dir()?.join("daemon.log");
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("opening {}", log_path.display()))?;
    let log_err = log.try_clone()?;

    let exe = std::env::current_exe().context("locating current executable")?;
    let mut cmd = Command::new(exe);
    cmd.arg(SERVE_ARG)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));

    detach(&mut cmd);

    cmd.spawn().context("spawning daemon process")?;
    Ok(())
}

/// Detach the child from the controlling terminal so it survives the CLI exit.
#[cfg(unix)]
fn detach(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    // Start a new session; the daemon becomes its own process-group leader.
    unsafe {
        cmd.pre_exec(|| {
            // setsid detaches from the controlling terminal.
            if libc_setsid() == -1 {
                // Non-fatal: still works while the terminal stays open.
            }
            Ok(())
        });
    }
}

#[cfg(unix)]
fn libc_setsid() -> i64 {
    // Avoid pulling in the libc crate for one symbol.
    extern "C" {
        fn setsid() -> i32;
    }
    unsafe { setsid() as i64 }
}

#[cfg(not(unix))]
fn detach(_cmd: &mut Command) {}

/// Stop the running daemon by sending it a signal / terminating the process.
pub fn stop() -> Result<bool> {
    let state = match DaemonState::read()? {
        Some(s) => s,
        None => return Ok(false),
    };
    let killed = terminate_pid(state.pid);
    let _ = DaemonState::clear();
    Ok(killed)
}

#[cfg(unix)]
fn terminate_pid(pid: u32) -> bool {
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    // SIGTERM = 15; triggers graceful shutdown in the daemon.
    unsafe { kill(pid as i32, 15) == 0 }
}

#[cfg(windows)]
fn terminate_pid(pid: u32) -> bool {
    Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Human-readable status report.
pub struct Status {
    pub running: bool,
    pub port: Option<u16>,
    pub pid: Option<u32>,
    pub started_at: Option<String>,
    pub db_path: PathBuf,
}

pub fn status() -> Result<Status> {
    let db_path = paths::database_path()?;
    let state = DaemonState::read()?;
    let running = running_port().is_some();
    Ok(Status {
        running,
        port: state.as_ref().map(|s| s.port),
        pid: state.as_ref().map(|s| s.pid),
        started_at: state.map(|s| s.started_at),
        db_path,
    })
}
