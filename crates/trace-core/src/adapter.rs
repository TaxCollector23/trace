//! The Trace Adapter System.
//!
//! Trace does not have special knowledge of any one coding assistant. Every
//! supported tool — Claude Code today, others later — plugs in through the
//! same [`Adapter`] trait and emits the same [`crate::models::EventType`]
//! vocabulary. The replay engine, the dashboard, and the rest of core only
//! ever consume that shared event stream; they never branch on which
//! assistant produced it. Adding a new assistant means writing a new
//! `Adapter` implementation, not touching core.
//!
//! Claude is implemented today as the reference adapter (see
//! `trace-cli/src/adapters/claude.rs`, which wraps the existing terminal
//! wrapper: guard, checkpoint, file watcher, diff capture, secret scan).
//! Cursor, Codex, Gemini, Aider, and OpenCode are designed for by this
//! trait but not yet implemented — adding them is future work, not
//! something this pass claims to have finished.

use anyhow::Result;
use serde_json::Value;

/// What a monitored session is running against: which project, which run
/// record it's tied to, and (when known) which agent is driving it.
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub project_root: std::path::PathBuf,
    pub run_id: String,
    pub agent_name: Option<String>,
}

/// The lifecycle every adapter implements. Capability methods
/// (`observe_*`) default to `false` so a minimal adapter — one that only
/// wraps a command and watches nothing else — compiles with almost no code;
/// override only the capabilities the underlying tool actually supports.
pub trait Adapter {
    /// Stable id used in CLI commands and metadata, e.g. "claude".
    fn id(&self) -> &'static str;

    /// One-time setup before any session starts (e.g. checking the
    /// underlying tool is installed).
    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    /// Called once when a monitored session begins.
    fn start_session(&mut self, ctx: &SessionContext) -> Result<()>;

    /// Called once when the session ends, successfully or not.
    fn stop_session(&mut self) -> Result<()> {
        Ok(())
    }

    /// Whether this adapter watches the project's filesystem for changes.
    fn observe_filesystem(&self) -> bool {
        false
    }
    /// Whether this adapter records the shell commands the session runs.
    fn observe_commands(&self) -> bool {
        false
    }
    /// Whether this adapter captures Git state (checkpoints, diffs, branch).
    fn observe_git(&self) -> bool {
        false
    }
    /// Whether this adapter captures raw terminal output.
    fn observe_terminal(&self) -> bool {
        false
    }

    /// Adapter-specific metadata to attach to the run (tool version, model,
    /// anything worth recording that isn't a first-class event field).
    fn capture_metadata(&self) -> Value {
        Value::Null
    }

    /// Release any resources (file handles, child processes) the adapter
    /// holds. Called after `stop_session`, even on error paths.
    fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }
}
