//! Concrete [`trace_core::adapter::Adapter`] implementations.
//!
//! `terminal` is the reference adapter and the only one implemented today —
//! it's what `trace run "<agent> ..."` uses regardless of which CLI agent
//! is named (Claude Code, Codex, Aider, OpenCode, or nothing recognized at
//! all). Cursor and Gemini are not terminal-wrapped tools in the same way
//! (Cursor already has its own MCP-based integration under
//! `integrations/cursor`) and would need their own adapters — not built
//! here, but nothing in `trace-core::adapter` assumes they can't exist.

pub mod terminal;
