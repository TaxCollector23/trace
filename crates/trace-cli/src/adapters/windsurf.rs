//! Windsurf adapter for the terminal wrapper path (`trace run "windsurf ..."`).
//!
//! Same shape as the Cursor adapter: Windsurf is a GUI editor, so a
//! terminal-wrapped session can observe filesystem and Git changes but
//! not meaningful piped terminal output. The primary Windsurf integration
//! surface is expected to be an MCP server (see `integrations/windsurf`);
//! this adapter exists so `trace run "windsurf ..."` produces a
//! Windsurf-labeled run instead of the generic terminal fallback.

use anyhow::Result;
use serde_json::{json, Value};

use trace_core::adapter::{Adapter, SessionContext};

pub struct WindsurfAdapter;

impl Adapter for WindsurfAdapter {
    fn id(&self) -> &'static str {
        "windsurf"
    }

    fn start_session(&mut self, _ctx: &SessionContext) -> Result<()> {
        Ok(())
    }

    fn observe_filesystem(&self) -> bool {
        true
    }
    fn observe_commands(&self) -> bool {
        true
    }
    fn observe_git(&self) -> bool {
        true
    }
    fn observe_terminal(&self) -> bool {
        false
    }

    fn capture_metadata(&self) -> Value {
        json!({ "adapter": "windsurf", "surface": "terminal-wrapper" })
    }
}
