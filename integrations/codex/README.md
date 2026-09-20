# Trace — Codex integration

The preferred integration uses Codex lifecycle hooks, shared by Codex CLI, the
desktop app, and the IDE extension. Install it with:

```bash
trc integrations install codex
```

Restart Codex, run `/hooks`, and trust Trace. After that, Trace sees each Bash
tool call and can deny a dangerous command before Codex executes it. File edits
are recorded and reviewed after the call.

For older Codex versions, launch through the wrapper:

```bash
trc run codex
trc run "codex implement the parser, do not touch src/legacy/"
```

This starts a Trace run, creates a checkpoint, watches file changes, records
command output and the Git diff, and finalizes the run state. The Codex
invocation maps to a Trace run id.
