# Claude Code local JSONL usage format

Status: observed importer contract, not a promise that Claude Code's private
transcript schema is stable.

## Evidence used

On 2026-10-12, Trace inspected the real files matching
~/.claude/projects/**/*.jsonl on the development machine. The observations
included both project-root session files and nested sidechain files:

The inspection found 297 JSONL files in total, including 243 files below a
subagents directory. These counts are a point-in-time observation, not a
repository fixture or a promise about another machine.

~~~text
~/.claude/projects/<project-slug>/<session-id>.jsonl
~/.claude/projects/<project-slug>/<session-id>/subagents/agent-<id>.jsonl
~/.claude/projects/<project-slug>/<session-id>/subagents/agent-<id>.meta.json
~~~

Observed top-level record types included assistant, user, system, attachment,
mode, ai-title, custom-title, last-prompt, and queue-operation. Only assistant
records with a nested message.usage object are usage events.

For those records, the observed accounting fields were:

| Location | Observed fields |
| --- | --- |
| record | sessionId, timestamp, parentUuid, isSidechain, agentId |
| message | id, model, usage |
| message.usage | input_tokens, output_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens_details (including observed thinking_tokens), and other provider fields |

Observed model labels included claude-opus-4-6,
claude-sonnet-5, claude-haiku-4-5-20251001, and <synthetic>. The
importer preserves a reported model label as metadata; an absent or invalid
model is unavailable (null). It does not invent a model from a path or
perform pricing lookup.

The real files also showed repeated assistant records with the same
message.id and different stream snapshots. This is why the canonical key is
(agent, session_id, message_id), and why importing a later snapshot updates
the existing row instead of creating another billable event.

## What Trace stores

The Claude Code importer stores only:

- session/message identity and parent identity;
- sidechain and subagent identifiers;
- provider-reported model and token counters;
- timestamp, source byte reference, and cost provenance.

It never persists message.content, user prompt text, assistant response text,
tool parameters, or tool output. raw_ref is a source path plus byte offset,
not a copy of the source line. The importer itself does not apply pricing math:
it preserves provider-reported cost only. The pricing consumer can map the row
to the versioned catalog through `UsageEventCostInput`; unknown model/rate/tier
data remains `cost_source = "unknown"` rather than zero.

The first consumer is:

~~~text
GET /api/usage?by=model|session|day|agent|project&since=<RFC3339>
trc usage [--by model|session|day|agent|project] [--since <RFC3339>] [--json]
~~~

Unknown totals remain JSON null and are rendered as unavailable; they are not
converted to zero.

## Incremental and attribution rules

- Each discovered JSONL path has a persistent byte cursor in import_sources.
- Only complete newline-terminated records advance the cursor. A partial final
  record is retried on the next import.
- Invalid complete JSONL records increment parse_errors/malformed_lines and do
  not create usage rows.
- Streaming duplicates are upserted by the canonical key and counted as
  updates in the import report.
- Main-session events use parentUuid when present.
- A sidechain event uses agentId/the subagents/agent-*.jsonl path for its
  subagent identity. When the adjacent .meta.json contains toolUseId, that
  value is the preferred parent message identifier; parentUuid is the
  fallback.

## Caveats and official references

Claude Code documents sessions as continuously written JSONL under
~/.claude/projects/<project>/<session-id>.jsonl, but explicitly describes
the transcript as an internal format that can change. This importer therefore
uses only fields observed in local files and treats unknown record types and
unknown fields as ignorable:

- [Claude Code sessions](https://code.claude.com/docs/en/sessions)
- [Claude Code usage monitoring and OpenTelemetry fields](https://code.claude.com/docs/en/monitoring-usage)

Anthropic's monitoring documentation is the authoritative source for
telemetry-oriented usage metrics. Local transcript import is a separate,
best-effort accounting view: it is not a billing statement, and provider
reported cost may be absent or approximate. If a future Claude Code version
changes the JSONL shape, the importer should fail closed for malformed
accounting records and add a fixture before accepting the new shape.
