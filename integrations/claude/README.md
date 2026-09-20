# Trace + Claude Code

The supported setup is automatic:

```bash
npm install -g trace-agent-cli
trc integrations install claude
```

The installer writes `~/.trace/integrations/shared/agent-hook.js` and patches
`~/.claude/settings.json` with SessionStart, SessionEnd, PreToolUse, and
PostToolUse hooks. It keeps your existing hooks and backs up the settings file.

Restart Claude Code, then run `trc integrations status` and open `trc dashboard`.
Each new Claude session becomes a Trace run. Bash commands are checked before
they run; file edits are recorded and reviewed after they land.

The old `trace-hook.sh` remains in this folder as a manual fallback for older
Claude Code installations. It is not needed by the normal installer.
