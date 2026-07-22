//! Claude Code adapter — wraps `claude` the same way the generic terminal
//! adapter does, but identifies itself distinctly so metadata and future
//! Claude-specific observation (e.g. parsing its own transcript format) has
//! somewhere to live without touching any other adapter.

use anyhow::Result;
use serde_json::{json, Value};

use trace_core::adapter::{Adapter, SessionContext};

use super::version::tool_version;

pub struct ClaudeAdapter;

impl Adapter for ClaudeAdapter {
    fn id(&self) -> &'static str {
        "claude"
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
        true
    }

    fn capture_metadata(&self) -> Value {
        json!({ "adapter": "claude", "version": tool_version("claude") })
    }
}
