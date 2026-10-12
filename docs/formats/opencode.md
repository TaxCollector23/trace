# OpenCode local format

Trace's OpenCode importer opens the local SQLite database read-only. It checks
`$OPENCODE_DATA_DIR`, then `$XDG_DATA_HOME/opencode`, then
`~/.local/share/opencode`, and accepts the current `opencode.db`/storage
layout. It reads `session` and assistant `message` metadata only. It does not
read the text-bearing `part.data` payloads.

## Evidence

- Official config locations: [OpenCode configuration](https://dev.opencode.ai/docs/config)
- Official session message shape: [OpenCode message API](https://dev.opencode.ai/v2/docs/api/session/v2-message-list/)
- Local database inspected on 2026-10-12: `$HOME/.local/share/opencode/opencode.db`

The inspected database has `session`, `message`, and `part` tables. Assistant
`message.data` contains `role`, `modelID`, `cost`, `tokens`, and timing fields.
The `tokens` object contains input/output/reasoning and nested cache read/write
counts. Session `parent_id` identifies child sessions in the inspected data.

## Trace behavior

Trace imports assistant messages, preserving the reported cost field exactly
when present, including a reported numeric zero. Missing cost is `unknown`;
Trace does not infer a price from a model name. Parent sessions are exposed as
subagent linkage where the local database provides it. A rowid is retained as
the source locator, and the database file is never modified.

The SQLite schema is an OpenCode implementation detail rather than a promised
stable interchange format. If a future release changes the table or JSON
shape, Trace reports parse/import errors instead of guessing.

Fixture and expected output: `tests/fixtures/agents/opencode/messages.json` and
`tests/fixtures/agents/opencode/expected.json`.
