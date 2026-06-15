//! `trg` — the TraceGuard CLI.
//!
//! A local black box recorder, safety layer, cost tracker, and patch-review
//! launcher for AI coding agents. The long alias `traceguard` is provided by the
//! install scripts as a symlink to this same binary.

mod client;
mod commands;
mod daemon_ctl;
mod project;
mod watcher;

use anyhow::Result;
use clap::{Parser, Subcommand};

use commands::run::RunOptions;

#[derive(Parser)]
#[command(
    name = "trg",
    bin_name = "trg",
    version,
    about = "TraceGuard — local black box recorder for AI coding agents.",
    long_about = "TraceGuard records what AI coding agents change, run, cost, and break, \
                  and helps you roll back. It is local-first: all data stays on your machine."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize TraceGuard in the current project.
    Init,

    /// Run a command under TraceGuard monitoring (e.g. `trg run "claude fix the bug"`).
    Run {
        /// The command to wrap. Quote multi-word commands.
        #[arg(required = true, trailing_var_arg = true, num_args = 1..)]
        command: Vec<String>,
        /// Skip configured project checks for this run.
        #[arg(long)]
        no_checks: bool,
        /// Auto-approve commands that would otherwise prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Open the local dashboard in your browser (starts the daemon if needed).
    Dashboard,

    /// Roll back to the most recent checkpoint (Git-based, with confirmation).
    Rollback {
        /// Skip the confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Compress a prompt locally before sending it to an agent.
    #[command(name = "compress-prompt")]
    CompressPrompt {
        /// The prompt text. Omit to paste interactively from stdin.
        #[arg(trailing_var_arg = true, num_args = 0..)]
        prompt: Vec<String>,
        /// Append an output-budget guidance block to the compressed prompt.
        #[arg(long)]
        budget: bool,
        /// Soft output token target to include in the budget block.
        #[arg(long)]
        target: Option<usize>,
        /// Accept the compressed prompt without prompting.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Manage the local daemon.
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },

    /// Internal: run the daemon server in the foreground (used by `daemon start`).
    #[command(name = "__serve", hide = true)]
    Serve,
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Start the local daemon.
    Start,
    /// Stop the local daemon.
    Stop,
    /// Show daemon status (running, port, database path, active projects).
    Status,
}

fn main() {
    if let Err(e) = real_main() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Run {
            command,
            no_checks,
            yes,
        } => commands::run::run(RunOptions {
            command: command.join(" "),
            no_checks,
            yes,
        }),
        Commands::Dashboard => commands::dashboard::run(),
        Commands::Rollback { yes } => commands::rollback::run(yes),
        Commands::CompressPrompt {
            prompt,
            budget,
            target,
            yes,
        } => commands::compress::run(commands::compress::CompressOptions {
            prompt: prompt.join(" "),
            budget,
            target,
            yes,
        }),
        Commands::Daemon { action } => daemon_action(action),
        Commands::Serve => {
            // Foreground server inside the detached child process.
            traceguard_daemon::run_blocking(traceguard_daemon::PREFERRED_PORT)
        }
    }
}

fn daemon_action(action: DaemonAction) -> Result<()> {
    match action {
        DaemonAction::Start => {
            let port = daemon_ctl::ensure_running()?;
            println!("Daemon running on http://127.0.0.1:{port}");
            Ok(())
        }
        DaemonAction::Stop => {
            if daemon_ctl::stop()? {
                println!("Daemon stopped.");
            } else {
                println!("Daemon was not running.");
            }
            Ok(())
        }
        DaemonAction::Status => {
            let status = daemon_ctl::status()?;
            if status.running {
                println!("Daemon: running");
            } else {
                println!("Daemon: not running");
            }
            if let Some(port) = status.port {
                println!("  port:    {port}");
                println!("  url:     http://127.0.0.1:{port}");
            }
            if let Some(pid) = status.pid {
                println!("  pid:     {pid}");
            }
            if let Some(started) = status.started_at {
                println!("  started: {started}");
            }
            println!("  db:      {}", status.db_path.display());

            // Active projects (best-effort).
            if let Some(port) = status.port {
                if let Ok(v) = client::Client::new(port).get_json::<serde_json::Value>("/api/state")
                {
                    if let Some(n) = v.get("active_projects").and_then(|n| n.as_u64()) {
                        println!("  projects: {n}");
                    }
                }
            }
            Ok(())
        }
    }
}
