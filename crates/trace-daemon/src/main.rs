//! Development entry point for the daemon: `cargo run -p trace-daemon`.
//!
//! In production the daemon is hosted inside the `trace` binary; this thin wrapper
//! exists so the server can be run directly during development.

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    trace_daemon::run_blocking(trace_daemon::PREFERRED_PORT)
}
