//! `trg` — the TraceGuard CLI.
//!
//! A local black box recorder, safety layer, cost tracker, and patch-review
//! launcher for AI coding agents. The long alias `traceguard` is provided by the
//! install scripts as a symlink to this same binary.

mod client;
mod commands;
mod daemon_ctl;
mod project;
mod settings;
mod watcher;

use anyhow::Result;
use clap::{Parser, Subcommand};

use commands::run::RunOptions;

#[derive(Parser)]
#[command(
    name = "trg",
    bin_name = "trg",
    // Version is handled manually (see `main`) so `--version` prints exactly
    // "TraceGuard 1.1" for both the `trg` and `traceguard` binary names.
    disable_version_flag = true,
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
        /// Compress the agent prompt with TraceCompress before running.
        #[arg(long)]
        compress: bool,
        /// Compression mode when --compress is set: normal | concise | bare.
        #[arg(long)]
        mode: Option<String>,
    },

    /// Open the local dashboard in your browser (starts the daemon if needed).
    Dashboard,

    /// Harden a prompt: clean it, attach strict rules, score and lint it.
    Guard {
        /// The prompt text (quote multi-word prompts).
        #[arg(required = true, trailing_var_arg = true, num_args = 1..)]
        prompt: Vec<String>,
        /// Hardening mode (minimal, balanced, strict, coding, research, …, no-bullshit).
        #[arg(long)]
        mode: Option<String>,
        /// Apply the strict coding-agent rule set.
        #[arg(long)]
        coding: bool,
        /// Detect the project and include its context.
        #[arg(long)]
        project: bool,
        /// Copy the hardened prompt to the clipboard.
        #[arg(long)]
        copy: bool,
        /// Launch the hardened prompt into a tool (e.g. claude, chatgpt, auto).
        #[arg(long)]
        launch: Option<String>,
        /// Skip secret-scan confirmation.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// List supported AI tools and whether each is installed.
    Agents,

    /// Set the default launch target (e.g. `trg use claude`).
    Use { target: String },

    /// Launch a prompt into an AI tool (CLI or web). Use `auto` to auto-route.
    Launch {
        /// Target tool id, or `auto` (e.g. claude, chatgpt, cursor, auto).
        target: String,
        /// The prompt text.
        #[arg(required = true, trailing_var_arg = true, num_args = 1..)]
        prompt: Vec<String>,
        /// Continue even if the prompt may contain secrets.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Run system checks (toolchain, clipboard, daemon, agents, paths).
    Doctor,

    /// Scan the current project and print its detected stack.
    Scan,

    /// Manage custom hardening rules.
    Rules {
        #[command(subcommand)]
        action: RulesAction,
    },

    /// List recent runs.
    Runs,

    /// Show a run's summary and timeline.
    Show { run_id: String },

    /// Show the changed files for a run.
    Patch { run_id: String },

    /// Show guarded commands and secret warnings for a run.
    Risks { run_id: String },

    /// Show API usage and estimated cost for a run.
    Costs { run_id: String },

    /// List checkpoints across recent runs.
    Checkpoints,

    /// Show or change project configuration.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// List integrations or check what is live.
    Integrations {
        #[command(subcommand)]
        action: Option<IntegrationsAction>,
    },

    /// Roll back to the most recent checkpoint (Git-based, with confirmation).
    Rollback {
        /// Skip the confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Compress a prompt locally before sending it to an agent (TraceCompress).
    #[command(name = "compress-prompt")]
    CompressPrompt {
        /// The prompt text. Omit to paste interactively from stdin.
        #[arg(trailing_var_arg = true, num_args = 0..)]
        prompt: Vec<String>,
        /// Compression mode: normal | concise | bare. Defaults to project config.
        #[arg(long)]
        mode: Option<String>,
        /// Append an output-budget preset block: tiny | short | normal | detailed.
        #[arg(long)]
        output_budget: Option<String>,
        /// Accept the compressed prompt without prompting.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Update the trg binary to the latest GitHub release.
    Update,

    /// Read directly from the project's GitHub repo (supports private repos).
    Github {
        #[command(subcommand)]
        action: GithubAction,
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
enum ConfigAction {
    /// Print the current project config.
    Show,
    /// Set a config key (e.g. prompt_compression.default_mode bare).
    Set { key: String, value: String },
}

#[derive(Subcommand)]
enum IntegrationsAction {
    /// Check what is live right now (daemon, GitHub token, …).
    Status,
}

#[derive(Subcommand)]
enum GithubAction {
    /// Show auth + repo connection status.
    Status,
    /// List recent commits from the remote repo.
    Commits {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// List open pull requests.
    Pulls,
    /// Print a file's contents from the remote repo (works for private repos).
    Cat {
        /// Path within the repo, e.g. src/main.rs.
        path: String,
        /// Git ref (branch, tag, or SHA). Defaults to the default branch.
        #[arg(long)]
        r#ref: Option<String>,
    },
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

#[derive(Subcommand)]
enum RulesAction {
    /// List custom hardening rules.
    List,
    /// Add a custom hardening rule.
    Add { rule: String },
    /// Remove all custom rules.
    Clear,
}

fn main() {
    // Handle `--version` / `-V` manually so the output is exactly
    // "TraceGuard 1.1" regardless of which binary name (trg or traceguard) was
    // invoked. clap's built-in flag is disabled for this reason.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("{}", traceguard_core::version_string());
        return;
    }

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
            compress,
            mode,
        } => commands::run::run(RunOptions {
            command: command.join(" "),
            no_checks,
            yes,
            compress,
            mode,
        }),
        Commands::Dashboard => commands::dashboard::run(),
        Commands::Guard {
            prompt,
            mode,
            coding,
            project,
            copy,
            launch,
            yes,
        } => commands::harden_cmd::run(commands::harden_cmd::GuardOptions {
            prompt: prompt.join(" "),
            mode,
            coding,
            project,
            copy,
            launch,
            yes,
        }),
        Commands::Agents => commands::launch::agents(),
        Commands::Use { target } => commands::launch::use_target(&target),
        Commands::Launch {
            target,
            prompt,
            yes,
        } => commands::launch::launch(commands::launch::LaunchOptions {
            target: Some(target),
            prompt: prompt.join(" "),
            yes,
        }),
        Commands::Doctor => commands::doctor::run(),
        Commands::Scan => commands::scan_cmd::run(),
        Commands::Rules { action } => commands::harden_cmd::rules(match action {
            RulesAction::List => commands::harden_cmd::RulesAction::List,
            RulesAction::Add { rule } => commands::harden_cmd::RulesAction::Add(rule),
            RulesAction::Clear => commands::harden_cmd::RulesAction::Clear,
        }),
        Commands::Runs => commands::query::runs(),
        Commands::Show { run_id } => commands::query::show(&run_id),
        Commands::Patch { run_id } => commands::query::patch(&run_id),
        Commands::Risks { run_id } => commands::query::risks(&run_id),
        Commands::Costs { run_id } => commands::query::costs(&run_id),
        Commands::Checkpoints => commands::query::checkpoints(),
        Commands::Config { action } => match action {
            ConfigAction::Show => commands::config_cmd::show(),
            ConfigAction::Set { key, value } => commands::config_cmd::set(&key, &value),
        },
        Commands::Integrations { action } => match action {
            Some(IntegrationsAction::Status) => commands::integrations::status(),
            None => commands::integrations::list(),
        },
        Commands::Rollback { yes } => commands::rollback::run(yes),
        Commands::CompressPrompt {
            prompt,
            mode,
            output_budget,
            yes,
        } => commands::compress::run(commands::compress::CompressOptions {
            prompt: prompt.join(" "),
            mode,
            output_budget,
            yes,
        }),
        Commands::Update => commands::update::run(),
        Commands::Github { action } => commands::github::run(match action {
            GithubAction::Status => commands::github::GithubCmd::Status,
            GithubAction::Commits { limit } => commands::github::GithubCmd::Commits { limit },
            GithubAction::Pulls => commands::github::GithubCmd::Pulls,
            GithubAction::Cat { path, r#ref } => commands::github::GithubCmd::Cat { path, r#ref },
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
