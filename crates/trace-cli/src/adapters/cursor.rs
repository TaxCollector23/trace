//! Cursor adapter for the terminal wrapper path (`trace run "cursor ..."`).
//!
//! This is distinct from `integrations/cursor`, which is Cursor's *primary*
//! integration surface: an MCP server Cursor talks to directly. Cursor is a
//! GUI editor, so a terminal-wrapped session can observe filesystem and Git
//! changes but not meaningful piped terminal output — `observe_terminal` is
//! honestly `false` here, unlike the CLI-first adapters.

use anyhow::Result;
use serde_json::{json, Value};

use trace_core::adapter::{Adapter, SessionContext};

pub struct CursorAdapter;

impl Adapter for CursorAdapter {
    fn id(&self) -> &'static str {
        "cursor"
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
        json!({ "adapter": "cursor", "surface": "terminal-wrapper" })
    }
}
