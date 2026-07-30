//! `trg run <command>` — execute a command under TraceGuard monitoring.
//!
//! The wrapper guards the top-level command, checkpoints git, watches files,
//! tees output, then derives the authoritative file changes from the final git
//! diff and scans for secrets. It is honest about its limits: a GUI tool
//! launched here is observed via file changes and git diffs, but its internal
//! actions cannot be guarded.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use traceguard_core::diagnose;
use traceguard_core::git;
use traceguard_core::guard::{self, Decision};
use traceguard_core::models::*;
use traceguard_core::{paths, secrets};

use crate::client::Client;
use crate::daemon_ctl;
use crate::project;
use crate::watcher::RunWatcher;

/// Options parsed from the CLI for a run.
pub struct RunOptions {
    pub command: String,
    /// Skip configured project checks even when present.
    pub no_checks: bool,
    /// Assume "yes" to approval prompts (non-interactive use).
    pub yes: bool,
    /// Compress the agent prompt with TraceCompress before running.
    pub compress: bool,
    /// Compression mode for --compress.
    pub mode: Option<String>,
}

pub fn run(opts: RunOptions) -> Result<()> {
    let project = project::load_current()?;
    let root = project.root.clone();
    let mut command = opts.command.trim().to_string();
    if command.is_empty() {
        anyhow::bail!("no command provided to run");
    }

    // 0. Optional TraceCompress: compress the agent prompt before running.
    // Never silently compress — the user sees before/after and confirms.
    let mut compression: Option<traceguard_core::prompt::CompressionResult> = None;
    if opts.compress {
        match maybe_compress(&command, opts.mode.as_deref(), opts.yes)? {
            Some((new_command, result)) => {
                command = new_command;
                compression = Some(result);
            }
            None => {
                println!("Compression declined; running the original command.");
            }
        }
    }

    // 1. Guard the command before doing anything else.
    let guard_result = guard::classify(&command);
    let proceed = handle_guard(&guard_result, opts.yes)?;

    // 2. Daemon + client.
    let port = daemon_ctl::ensure_running()?;
    let client = Arc::new(Client::new(port));

    // Look up the registered project id (init registered it).
    let project_id = lookup_project_id(&client, &root)?;

    // 3. Capture git starting state.
    let start_state = git::capture_state(&root);
    if !start_state.is_repo {
        println!("Warning: not a Git repository — no checkpoint or rollback for this run.");
    } else if start_state.dirty {
        println!("Note: working tree is dirty; recording starting state as dirty.");
    }

    // 4. Create the run record.
    let (agent_name, user_prompt) = split_agent_prompt(&command);
    let run: Run = client.post_json(
        "/api/runs",
        &NewRun {
            project_id: project_id.clone(),
            command: command.clone(),
            agent_name,
            user_prompt,
            starting_commit: start_state.commit.clone(),
        },
    )?;
    let run_id = run.id.clone();

    event(
        &client,
        &run_id,
        EventType::RunCreated,
        &format!("Run created for `{command}`"),
    );

    // Persist compression metadata with the run (honoring prompt-history).
    if let Some(ref result) = compression {
        let _ = client.post(
            "/api/prompt-compressor/record",
            &crate::commands::compress::build_record(result, Some(run_id.clone())),
        );
        event(
            &client,
            &run_id,
            EventType::Note,
            &format!(
                "TraceCompress ({} mode): ~{} → ~{} tokens (~{:.0}% reduction)",
                result.mode, result.original_tokens, result.compressed_tokens, result.reduction_pct
            ),
        );
    }

    if start_state.dirty {
        event(
            &client,
            &run_id,
            EventType::Note,
            "Starting working tree was dirty",
        );
    }

    // Record the command decision.
    let _ = client.post(
        &format!("/api/runs/{run_id}/commands"),
        &NewCommand {
            command: command.clone(),
            decision: guard_result.decision.as_str().to_string(),
            exit_code: None,
            stdout_path: None,
            stderr_path: None,
        },
    );
    record_guard_event(&client, &run_id, &guard_result);

    // If the command was blocked or the user declined, finalize and stop.
    if !proceed {
        finish(
            &client,
            &run_id,
            RunStatus::Blocked,
            None,
            start_state.commit.as_deref(),
        );
        println!("Command blocked. Recorded as a blocked run.");
        print_dashboard_hint(port);
        return Ok(());
    }

    // 5. Create a checkpoint.
    if start_state.is_repo {
        match git::create_checkpoint(&root) {
            Ok(Some(git_ref)) => {
                let _ = client.post(
                    &format!("/api/runs/{run_id}/checkpoints"),
                    &NewCheckpoint {
                        project_id: project_id.clone(),
                        git_ref: Some(git_ref.clone()),
                        checkpoint_type: if start_state.dirty { "stash" } else { "commit" }
                            .to_string(),
                    },
                );
                event(
                    &client,
                    &run_id,
                    EventType::CheckpointCreated,
                    &format!("Checkpoint created at {}", short(&git_ref)),
                );
            }
            Ok(None) => {}
            Err(e) => println!("Warning: could not create checkpoint: {e}"),
        }
    }

    // 6. Start the file watcher.
    let watcher = RunWatcher::start(root.clone(), client.clone(), run_id.clone()).ok();

    // 7. Execute the wrapped command, teeing output.
    event(
        &client,
        &run_id,
        EventType::CommandStarted,
        &format!("Started `{command}`"),
    );
    let log_dir = paths::run_log_dir(&root, &run_id);
    std::fs::create_dir_all(&log_dir).ok();
    let (exit_code, output) = exec_tee(&command, &root, &log_dir)?;

    // Stop the watcher before computing the final diff.
    if let Some(w) = watcher {
        w.stop();
    }

    // Update the command record with its exit code + log paths.
    let _ = client.post(
        &format!("/api/runs/{run_id}/commands"),
        &NewCommand {
            command: command.clone(),
            decision: "executed".to_string(),
            exit_code: Some(exit_code as i64),
            stdout_path: Some(log_dir.join("stdout.log").display().to_string()),
            stderr_path: Some(log_dir.join("stderr.log").display().to_string()),
        },
    );

    // 8. Final git diff = source of truth for file changes.
    let mut changes: Vec<git::DiffEntry> = Vec::new();
    if let Some(ref from) = start_state.commit {
        match git::diff_against(&root, from) {
            Ok(c) => changes = c,
            Err(e) => println!("Warning: could not compute git diff: {e}"),
        }
    }
    record_file_changes(&client, &run_id, &changes);

    // Persist the full unified diff next to the run logs so the dashboard can
    // render a Git diff view. Kept on disk (not in SQLite) to avoid bloat.
    if let Some(ref from) = start_state.commit {
        if let Ok(diff_text) = git::full_diff(&root, from) {
            let _ = std::fs::write(log_dir.join("diff.patch"), &diff_text);
        }
    }

    event(
        &client,
        &run_id,
        EventType::FinalDiffCaptured,
        &format!(
            "Captured final diff: {}",
            diagnose::change_summary(&changes)
        ),
    );

    // 9. Secret + protected-file scan over output, diff, and changed files.
    scan_secrets(
        &client,
        &run_id,
        &root,
        &changes,
        start_state.commit.as_deref(),
        &output,
        &project.config,
    );

    // 10. Optional configured project checks.
    let mut combined_output = output.clone();
    let mut checks_failed = false;
    if !opts.no_checks && !project.config.default_checks.is_empty() {
        let checks = project.config.default_checks.clone();
        println!("\nRunning configured checks...");
        for check in &checks {
            let (ok, out) = run_check(&client, &run_id, check, &root);
            combined_output.push_str(&out);
            if !ok {
                checks_failed = true;
            }
        }
    }

    // 11. Deterministic diagnosis when something failed.
    if exit_code != 0 || checks_failed {
        for hyp in diagnose::diagnose(&changes, &combined_output) {
            event(
                &client,
                &run_id,
                EventType::Note,
                &format!("Diagnosis: {}", hyp.summary),
            );
        }
    }

    // 12. Finalize the run.
    let end_state = git::capture_state(&root);
    let status = if exit_code == 0 && !checks_failed {
        RunStatus::Completed
    } else {
        RunStatus::Failed
    };
    finish(
        &client,
        &run_id,
        status,
        Some(exit_code as i64),
        end_state.commit.as_deref(),
    );
    event(
        &client,
        &run_id,
        if status == RunStatus::Completed {
            EventType::RunCompleted
        } else {
            EventType::RunFailed
        },
        &format!("Run {} (exit {exit_code})", status.as_str()),
    );

    // 13. Summary.
    print_summary(&command, exit_code, &changes, status);
    print_dashboard_hint(port);
    Ok(())
}

// --- TraceCompress integration -------------------------------------------

/// Compress the agent-prompt portion of a command. Shows before/after and asks
/// for confirmation (unless `assume_yes`). Returns the rebuilt command and the
/// compression result, or `None` if there is nothing to compress / declined.
fn maybe_compress(
    command: &str,
    mode_flag: Option<&str>,
    assume_yes: bool,
) -> Result<Option<(String, traceguard_core::prompt::CompressionResult)>> {
    let (agent, prompt) = split_agent_prompt(command);
    let Some(prompt_text) = prompt else {
        println!("Nothing to compress (no agent prompt detected in the command).");
        return Ok(None);
    };

    let mode = crate::commands::compress::resolve_mode(mode_flag);
    let result = traceguard_core::prompt::compress_with_mode(&prompt_text, mode);
    crate::commands::compress::print_result(&result, None);

    if !result.conflicts.is_empty()
        && !assume_yes
        && !prompt_yes_no("Proceed despite the conflict(s) above?")?
    {
        return Ok(None);
    }
    if !assume_yes && !prompt_yes_no("Run the agent with this compressed prompt?")? {
        return Ok(None);
    }

    // Rebuild the command: keep the agent, swap in the compressed prompt.
    let rebuilt = match agent {
        Some(a) => format!("{a} {}", result.compressed),
        None => result.compressed.clone(),
    };
    Ok(Some((rebuilt, result)))
}

// --- Guard prompting ------------------------------------------------------

/// Returns Ok(true) to proceed, Ok(false) to block.
fn handle_guard(result: &guard::GuardResult, assume_yes: bool) -> Result<bool> {
    match result.decision {
        Decision::Allow => Ok(true),
        Decision::Warn => {
            println!("TraceGuard warning: {}", result.reason);
            Ok(true)
        }
        Decision::RequireApproval => {
            println!("\nTraceGuard warning:");
            println!("  {}", result.reason);
            if assume_yes {
                println!("  Auto-approved (--yes).");
                return Ok(true);
            }
            Ok(prompt_yes_no("Allow this command to run?")?)
        }
        Decision::Block => {
            println!("\nTraceGuard blocked this command:");
            println!("  {}", result.reason);
            println!("This command is not allowed to run through TraceGuard.");
            Ok(false)
        }
    }
}

fn prompt_yes_no(question: &str) -> Result<bool> {
    print!("{question} [y] allow once  [n] block: ");
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}

fn record_guard_event(client: &Client, run_id: &str, result: &guard::GuardResult) {
    let (etype, msg) = match result.decision {
        Decision::Block => (
            EventType::RiskyCommandBlocked,
            format!("Blocked: {}", result.reason),
        ),
        Decision::RequireApproval => (
            EventType::CommandApproved,
            format!("Approved (was risky): {}", result.reason),
        ),
        Decision::Warn => (
            EventType::RiskyCommandWarned,
            format!("Warned: {}", result.reason),
        ),
        Decision::Allow => return,
    };
    event(client, run_id, etype, &msg);
}

// --- Execution with output teeing ----------------------------------------

/// Run `command` through the platform shell, streaming output to the console and
/// to `stdout.log` / `stderr.log`, while collecting combined text for scanning.
fn exec_tee(
    command: &str,
    cwd: &std::path::Path,
    log_dir: &std::path::Path,
) -> Result<(i32, String)> {
    let mut child = shell_command(command)
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawning `{command}`"))?;

    let collected = Arc::new(Mutex::new(String::new()));
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let out_handle = stdout.map(|s| {
        let collected = collected.clone();
        let path = log_dir.join("stdout.log");
        std::thread::spawn(move || tee_stream(s, path, collected, false))
    });
    let err_handle = stderr.map(|s| {
        let collected = collected.clone();
        let path = log_dir.join("stderr.log");
        std::thread::spawn(move || tee_stream(s, path, collected, true))
    });

    let status = child.wait().context("waiting for command")?;
    if let Some(h) = out_handle {
        let _ = h.join();
    }
    if let Some(h) = err_handle {
        let _ = h.join();
    }

    let output = collected.lock().unwrap().clone();
    Ok((status.code().unwrap_or(-1), output))
}

fn tee_stream<R: std::io::Read>(
    reader: R,
    log_path: std::path::PathBuf,
    collected: Arc<Mutex<String>>,
    is_err: bool,
) {
    let mut file = std::fs::File::create(&log_path).ok();
    let buf = BufReader::new(reader);
    for line in buf.lines().map_while(Result::ok) {
        if is_err {
            eprintln!("{line}");
        } else {
            println!("{line}");
        }
        if let Some(f) = file.as_mut() {
            let _ = writeln!(f, "{line}");
        }
        if let Ok(mut c) = collected.lock() {
            c.push_str(&line);
            c.push('\n');
        }
    }
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut c = Command::new("cmd");
    c.arg("/C").arg(command);
    c
}

#[cfg(not(windows))]
fn shell_command(command: &str) -> Command {
    let mut c = Command::new("sh");
    c.arg("-c").arg(command);
    c
}

// --- Checks ---------------------------------------------------------------

/// Run a configured check, recording a test_result and timeline events.
/// Returns (passed, output).
fn run_check(client: &Client, run_id: &str, check: &str, cwd: &std::path::Path) -> (bool, String) {
    let is_test = check.to_lowercase().contains("test");
    event(
        client,
        run_id,
        EventType::BuildStarted,
        &format!("Check started: {check}"),
    );
    println!("  $ {check}");

    let output = shell_command(check).current_dir(cwd).output();

    let (code, combined) = match output {
        Ok(o) => {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            (o.status.code().unwrap_or(-1), text)
        }
        Err(e) => (-1, format!("failed to start check: {e}")),
    };

    let passed = code == 0;
    let summary = summarize_output(&combined);
    let _ = client.post(
        &format!("/api/runs/{run_id}/test-results"),
        &NewTestResult {
            command: check.to_string(),
            status: if passed {
                "passed".into()
            } else {
                "failed".into()
            },
            output_summary: Some(summary),
        },
    );

    let etype = match (is_test, passed) {
        (true, true) => EventType::TestsPassed,
        (true, false) => EventType::TestsFailed,
        (false, true) => EventType::BuildPassed,
        (false, false) => EventType::BuildFailed,
    };
    event(
        client,
        run_id,
        etype,
        &format!(
            "Check {}: {check}",
            if passed { "passed" } else { "failed" }
        ),
    );
    println!("  -> {}", if passed { "passed" } else { "FAILED" });
    (passed, combined)
}

fn summarize_output(output: &str) -> String {
    // Keep the last ~30 lines so failure tails are preserved.
    let lines: Vec<&str> = output.lines().collect();
    let start = lines.len().saturating_sub(30);
    lines[start..].join("\n")
}

// --- Recording helpers ----------------------------------------------------

fn record_file_changes(client: &Client, run_id: &str, changes: &[git::DiffEntry]) {
    let payload: Vec<NewFileChange> = changes
        .iter()
        .map(|c| NewFileChange {
            path: c.path.clone(),
            change_type: c.change_type.as_str().to_string(),
            diff_summary: c.diff_summary.clone(),
        })
        .collect();
    let _ = client.post(&format!("/api/runs/{run_id}/file-changes"), &payload);
}

fn scan_secrets(
    client: &Client,
    run_id: &str,
    root: &std::path::Path,
    changes: &[git::DiffEntry],
    start_commit: Option<&str>,
    output: &str,
    config: &traceguard_core::ProjectConfig,
) {
    let mut posted = 0;

    // Scan combined command output.
    for finding in secrets::scan_text(output) {
        post_secret(
            client,
            run_id,
            None,
            &finding.secret_type,
            &finding.redacted_value,
        );
        posted += 1;
    }

    // Scan the full diff text (added/removed lines), if we have a base commit.
    if let Some(from) = start_commit {
        if let Ok(diff_text) = git::full_diff(root, from) {
            for finding in secrets::scan_text(&diff_text) {
                post_secret(
                    client,
                    run_id,
                    None,
                    &finding.secret_type,
                    &finding.redacted_value,
                );
                posted += 1;
            }
        }
    }

    // Scan changed files on disk + flag protected / env-like files.
    for change in changes {
        if config.is_protected(&change.path) {
            let _ = client.post(
                &format!("/api/runs/{run_id}/secrets"),
                &NewSecret {
                    file_path: Some(change.path.clone()),
                    secret_type: "protected_file".to_string(),
                    redacted_value: "(protected file changed)".to_string(),
                    action_taken: "warned".to_string(),
                },
            );
            event(
                client,
                run_id,
                EventType::SecretWarning,
                &format!("Protected file changed: {}", change.path),
            );
            posted += 1;
        }

        let abs = root.join(&change.path);
        if let Ok(content) = std::fs::read_to_string(&abs) {
            for finding in secrets::scan_text(&content) {
                post_secret(
                    client,
                    run_id,
                    Some(&change.path),
                    &finding.secret_type,
                    &finding.redacted_value,
                );
                posted += 1;
            }
        }
    }

    if posted > 0 {
        println!(
            "TraceGuard recorded {posted} secret/protected-file warning(s) (values redacted)."
        );
    }
}

fn post_secret(
    client: &Client,
    run_id: &str,
    file: Option<&str>,
    secret_type: &str,
    redacted: &str,
) {
    let _ = client.post(
        &format!("/api/runs/{run_id}/secrets"),
        &NewSecret {
            file_path: file.map(|s| s.to_string()),
            secret_type: secret_type.to_string(),
            redacted_value: redacted.to_string(),
            action_taken: "warned".to_string(),
        },
    );
    event(
        client,
        run_id,
        EventType::SecretWarning,
        &format!("Possible {secret_type} detected"),
    );
}

fn event(client: &Client, run_id: &str, event_type: EventType, message: &str) {
    let _ = client.post(
        &format!("/api/runs/{run_id}/events"),
        &NewEvent {
            event_type: event_type.as_str().to_string(),
            message: message.to_string(),
            metadata_json: None,
        },
    );
}

fn finish(
    client: &Client,
    run_id: &str,
    status: RunStatus,
    exit_code: Option<i64>,
    ending_commit: Option<&str>,
) {
    let _ = client.post(
        &format!("/api/runs/{run_id}/finish"),
        &serde_json::json!({
            "status": status.as_str(),
            "exit_code": exit_code,
            "ending_commit": ending_commit,
        }),
    );
}

fn lookup_project_id(client: &Client, root: &std::path::Path) -> Result<String> {
    let projects: Vec<Project> = client.get_json("/api/projects")?;
    let target = root.display().to_string();
    projects
        .into_iter()
        .find(|p| p.path == target)
        .map(|p| p.id)
        .context("project not registered; run `trg init`")
}

// --- Small utilities ------------------------------------------------------

/// Split a wrapped command into (agent_name, user_prompt).
fn split_agent_prompt(command: &str) -> (Option<String>, Option<String>) {
    let agent = guard::detect_agent(command);
    let prompt = agent.as_ref().and_then(|a| {
        let rest = command[a.len()..].trim();
        if rest.is_empty() {
            None
        } else {
            Some(rest.to_string())
        }
    });
    (agent, prompt)
}

fn short(git_ref: &str) -> String {
    git_ref.chars().take(10).collect()
}

fn print_summary(command: &str, exit_code: i32, changes: &[git::DiffEntry], status: RunStatus) {
    println!("\n── TraceGuard run summary ──");
    println!("  command: {command}");
    println!("  status:  {} (exit {exit_code})", status.as_str());
    println!("  changes: {}", diagnose::change_summary(changes));
}

fn print_dashboard_hint(port: u16) {
    println!("  dashboard: http://127.0.0.1:{port}  (run `trg dashboard`)");
}
