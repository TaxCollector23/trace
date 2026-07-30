//! traceguard-daemon: the local API server and dashboard host.
//!
//! Exposed as a library so the single `trg` binary can host the daemon in a
//! child process — release artifacts ship only `trg`. A thin `main.rs` is kept
//! for `cargo run -p traceguard-daemon` during development.

pub mod api;
pub mod assets;
pub mod server;
pub mod state;

pub use server::{serve, PREFERRED_PORT};
pub use state::DaemonState;

/// Run the daemon to completion on a fresh multi-threaded Tokio runtime.
///
/// Convenience for callers (the CLI, the dev binary) that are not already async.
pub fn run_blocking(preferred_port: u16) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(serve(preferred_port))
}
