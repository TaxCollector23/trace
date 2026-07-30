//! OpenAI Codex CLI adapter. Same wrapper mechanism as every other
//! terminal-launched agent; see `integrations/codex` for the standalone
//! shell shim that routes bare `codex` invocations through this same path.

use anyhow::Result;
use serde_json::{json, Value};

use trace_core::adapter::{Adapter, SessionContext};

use super::version::tool_version;

pub struct CodexAdapter;

impl Adapter for CodexAdapter {
    fn id(&self) -> &'static str {
        "codex"
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
        json!({ "adapter": "codex", "version": tool_version("codex") })
    }
}
