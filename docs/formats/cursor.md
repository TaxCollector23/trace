# Cursor local format

Trace treats Cursor's native chat store as opaque. Cursor documents chat
history as local SQLite and supports exporting history, but does not publish a
stable schema for the native conversation blobs. The inspectable local agent
surface on this machine is the JSONL transcript tree under
`~/.cursor/projects/**/agent-transcripts/`.

## Evidence

- Official history documentation: [Cursor agent chat history](https://docs.cursor.com/en/agent/chat/history)
- Official SDK/local-store documentation: [Cursor TypeScript SDK](https://cursor.com/docs/sdk/typescript)
- Local files inspected on 2026-10-12: `$HOME/.cursor/**/agent-transcripts/*.jsonl`
  and `$HOME/.cursor/chats/**/store.db`

Observed transcript lines contain a `message` object with a `role`, usually
`user` or `assistant`; the native SQLite `store.db` blobs did not expose a
documented usage schema during inspection. The importer accepts optional
structured `usage`, `tokens`, `model`, and timestamp fields if a transcript
version provides them, but does not promote message content into Trace.

## Trace behavior and limitation

Assistant transcript records are imported when they have a stable id, or a
byte-offset locator is used. Missing token and cost fields stay unavailable.
Native `store.db` files are reported as discovered source evidence but are not
decoded speculatively. Cursor token and cost capabilities therefore remain
`not_available` until Cursor publishes a stable local accounting format or a
future Trace importer is backed by a real fixture.

Fixture and expected output: `tests/fixtures/agents/cursor/basic.jsonl` and
`tests/fixtures/agents/cursor/expected.json`.
