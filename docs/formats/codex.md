# Codex CLI local format

Trace's Codex importer reads local rollout JSONL files below
`$CODEX_HOME/sessions`, falling back to `~/.codex/sessions`. The source is
read-only and the importer stores accounting metadata only. It never stores
prompt text, response text, tool arguments, or tool output.

## Evidence

- Official Codex telemetry/config source: [Codex OTel README](https://github.com/openai/codex/blob/main/codex-rs/otel/README.md)
- Official Codex configuration types: [Codex config types](https://github.com/openai/codex/blob/main/codex-rs/config/src/types.rs)
- Local files inspected on 2026-10-12: `$HOME/.codex/sessions/**/rollout-*.jsonl`

The inspected records use an outer JSON object with `type`, `timestamp`, and
`payload`. `session_meta` identifies a session. `token_usage_record` contains
`response_id`, `turn_id`, optional session/agent linkage, and a `usage` object
with `input_tokens`, `cached_input_tokens`, `cache_write_input_tokens`,
`output_tokens`, `reasoning_output_tokens`, and `total_tokens`. Other
`response_item` records are deliberately ignored by the usage importer.

## Trace behavior

Trace consumes only complete newline-terminated records after the stored byte
cursor. A malformed complete record increments `parse_errors`; an incomplete
final line is retried later. Events are deduplicated by
`(agent, session_id, message_id)`.

The local rollout records inspected did not contain a provider cost field, so
Codex cost is `not_available`, never `$0` or a computed estimate. Subagent
flags and identifiers are retained when present; linkage is `partial` because
the rollout shape varies by version.

Fixture and expected output: `tests/fixtures/agents/codex/basic.jsonl` and
`tests/fixtures/agents/codex/expected.json`.
