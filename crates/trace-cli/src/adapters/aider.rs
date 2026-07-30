//! Aider adapter.

use anyhow::Result;
use serde_json::{json, Value};

use trace_core::adapter::{Adapter, SessionContext};

use super::version::tool_version;

pub struct AiderAdapter;

impl Adapter for AiderAdapter {
    fn id(&self) -> &'static str {
        "aider"
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
        json!({ "adapter": "aider", "version": tool_version("aider") })
    }
}
