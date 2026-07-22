//! Concrete [`trace_core::adapter::Adapter`] implementations.
//!
//! Every supported agent — Claude Code, Codex, Cursor, Gemini, Aider,
//! OpenCode — plugs in through the same trait. Today they all share one
//! real capability underneath (the terminal wrapper observes filesystem,
//! Git, commands, and output regardless of which agent is running), so
//! each adapter is small; that's expected; each one is a real, distinct
//! place for agent-specific behavior to grow into later without touching
//! any other adapter or `trace-core`. `terminal` is the fallback used for
//! any command whose agent isn't recognized.

pub mod aider;
pub mod claude;
pub mod codex;
pub mod cursor;
pub mod gemini;
pub mod opencode;
pub mod terminal;
mod version;

use trace_core::adapter::Adapter;

/// Pick the adapter matching a detected agent name (see
/// `trace_core::guard::detect_agent`), falling back to the generic
/// terminal adapter for anything unrecognized.
pub fn for_agent(agent_name: Option<&str>) -> Box<dyn Adapter> {
    match agent_name {
        Some("claude") => Box::new(claude::ClaudeAdapter),
        Some("codex") => Box::new(codex::CodexAdapter),
        Some("cursor") => Box::new(cursor::CursorAdapter),
        Some("gemini") => Box::new(gemini::GeminiAdapter),
        Some("aider") => Box::new(aider::AiderAdapter),
        Some("opencode") => Box::new(opencode::OpenCodeAdapter),
        other => Box::new(terminal::TerminalAdapter::new(other.map(str::to_string))),
    }
}
