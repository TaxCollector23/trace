//! `trace` — the Trace CLI.
//!
//! A local black box recorder, safety layer, cost tracker, and patch-review
//! tool for AI coding agents.

mod client;
mod colors;
mod commands;
mod daemon_ctl;
mod project;
mod watcher;

use anyhow::Result;
use clap::builder::styling::{AnsiColor, Color, RgbColor, Style, Styles};
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

use commands::run::RunOptions;

/// Bold brand-indigo headers/usage, plain-bold flag names, dim placeholders —
/// the same accent used across the landing page, dashboard, and CLI banner.
fn help_styles() -> Styles {
    let brand = Color::Rgb(RgbColor(124, 123, 251));
    Styles::styled()
        .header(Style::new().bold().fg_color(Some(brand)))
        .usage(Style::new().bold().fg_color(Some(brand)))
        .literal(Style::new().bold())
        .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack))))
        .valid(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))))
        .invalid(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red))))
        .error(
            Style::new()
                .bold()
                .fg_color(Some(Color::Ansi(AnsiColor::Red))),
        )
}

#[derive(Parser)]
#[command(
    name = "trace",
    bin_name = "trace",
    // Version is handled manually (see `main`) so `--version` prints exactly
    // "Trace 1.1".
    disable_version_flag = true,
    about = "Trace — the trust layer for AI software engineering.",
    long_about = "Trace records what AI coding agents change, run, cost, and break, \
                  and helps you roll back. It is local-first: all data stays on your machine."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Trace in the current project.
    Init,

    /// Run a command under Trace monitoring (e.g. `trace run "claude fix the bug"`).
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

    /// Run system checks (toolchain, clipboard, daemon, agents, paths).
    Doctor,

    /// Scan the current project and print its detected stack.
    Scan,

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

    /// Update the trace binary to the latest GitHub release.
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
    /// Set a config key.
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

/// "Trace" in the figlet "ANSI Shadow" font — the same block-with-shadow
/// geometry as the dashboard's brand mark, rendered in pure box-drawing
/// characters (U+2588 full block, U+2551/2550 shadow lines, U+2554 etc.).
const BANNER: &str = "\
████████╗██████╗  █████╗  ██████╗███████╗
╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██╔════╝
   ██║   ██████╔╝███████║██║     █████╗
   ██║   ██╔══██╗██╔══██║██║     ██╔══╝
   ██║   ██║  ██║██║  ██║╚██████╗███████╗
   ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚══════╝";

/// Print the startup banner. Skipped when stdout is not a terminal (piped,
/// redirected to a log file, or CI) so it never pollutes scripted output.
fn print_banner() {
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() {
        return;
    }
    println!("{}\n", colors::brand(BANNER));
}

fn main() {
    print_banner();

    // Handle `--version` / `-V` manually so the output is exactly "Trace 1.1".
    // clap's built-in flag is disabled for this reason.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("{}", trace_core::version_string());
        return;
    }

    if let Err(e) = real_main() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<()> {
    let matches = Cli::command().styles(help_styles()).get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(e) => e.exit(),
    };
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
        Commands::Doctor => commands::doctor::run(),
        Commands::Scan => commands::scan_cmd::run(),
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
            trace_daemon::run_blocking(trace_daemon::PREFERRED_PORT)
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
