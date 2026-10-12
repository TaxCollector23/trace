# Claude Code golden fixtures

These transcripts are deliberately synthetic and sanitized. They contain no
user prompts, response text, real filesystem paths, credentials, or real
session identifiers. Every `content` array is empty and every path is the
literal placeholder `<path>`.

The shape follows the Claude Code JSONL records observed locally on 2026-10-12:
usage is attached to `assistant.message.usage`, message identity is in
`assistant.message.id`, and sidechain records carry `isSidechain` plus an
`agentId`. Claude Code stores local transcripts below `~/.claude/projects/` and
documents monitoring through OpenTelemetry; the fixture harness deliberately
tests the file importer only. The format is not treated as a stable public
schema, so unknown fields and malformed/truncated input are part of the
harness cases.

The known-model expected costs use the Anthropic global standard rates recorded
in `pricing_source` below. The arithmetic is intentionally visible in each
`expected.json`; the pricing implementation remains owned by the pricing
workstream. Synthetic timestamps are 2026-10-12 so those cases exercise the
catalog's `official-2026-10-12` effective window rather than silently testing
an unavailable historical rate.

Fixtures:

- `cache-tokens`: input, output, cache-read, and both 5-minute and 1-hour
  cache-write token fields.
- `subagents`: a parent transcript plus a sidechain transcript linked to the
  parent message.
- `duplicate-streaming`: two records with the same agent/session/message key;
  they must count once.
- `unknown-model`: valid usage with an unseen model; cost must be unavailable,
  never zero.

Run structural validation without an importer:

```bash
python3 scripts/usage_fixture_harness.py --validate-only
```

Run the importer-backed golden and property suite after Workstreams 2–3 land:

```bash
python3 scripts/usage_fixture_harness.py --binary target/release/trc --all --properties
```

The harness creates a temporary `HOME`, `TRACE_HOME`, and `TRACE_DB` for every
case. It never reads or writes the user's real Claude directory during tests.

`pricing_source`: Anthropic, “Pricing - Claude Platform Docs”, retrieved
2026-10-12, https://platform.claude.com/docs/en/about-claude/pricing
