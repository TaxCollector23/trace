# TraceGuard — Codex CLI integration

Wrapper-first. Launch Codex through TraceGuard:

```bash
trg run codex
trg run "codex implement the parser, do not touch src/legacy/"
```

This starts a TraceGuard run, creates a checkpoint, watches file changes,
records command output and the Git diff, and finalizes the run state. The Codex
invocation maps to a TraceGuard run id.

## Optional deeper integration

`codex-adapter.sh` is a thin shim you can place on your PATH as `codex` (or alias)
to always route Codex through TraceGuard. If Codex exposes logs/hooks/config in
the future, extend the adapter here. TraceGuard does not assume unsupported
Codex internals — wrapper monitoring is the reliable baseline.
