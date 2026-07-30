//! The reference [`Adapter`] implementation: any terminal-launched coding
//! agent, wrapped rather than modified. This is what every
//! `trace run "<agent> ..."` session uses today, regardless of which agent
//! name (if any) was recognized in the command.

use anyhow::Result;
use serde_json::{json, Value};

use trace_core::adapter::{Adapter, SessionContext};

pub struct TerminalAdapter {
    agent_name: Option<String>,
}

impl TerminalAdapter {
    pub fn new(agent_name: Option<String>) -> Self {
        TerminalAdapter { agent_name }
    }
}

impl Adapter for TerminalAdapter {
    fn id(&self) -> &'static str {
        "terminal"
    }

    fn start_session(&mut self, _ctx: &SessionContext) -> Result<()> {
        // Nothing to set up: the wrapper doesn't launch the agent itself,
        // the shell does. Filesystem/git/command observation is handled by
        // trace-cli's existing watcher + guard + git modules, which this
        // adapter's `observe_*` flags below advertise as active.
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
        json!({ "adapter": "terminal", "agent": self.agent_name })
    }
}
