# Aider local format

Trace reads Aider's documented Markdown history file, normally
`.aider.chat.history.md`. Aider also supports a configurable LLM history file;
Trace only imports the default chat history path currently.

## Evidence

- Official history configuration: [Aider `.aider.conf.yml`](https://aider.chat/docs/config/aider_conf.html)
- Official caching/cost limitation: [Aider caching](https://aider.chat/docs/usage/caching.html)
- Local file inspected on 2026-10-12: `$HOME/.aider.chat.history.md`
- Parser source reference: [Aider IO history writer](https://github.com/Aider-AI/aider/blob/main/aider/io.py)

The documented history is Markdown with chat headings. When Aider writes
optional summaries such as `> Model:`, `> Tokens: ... sent, ... received`, or
`> Cost: $...`, Trace imports those values. Timestamps without a timezone are
not converted to an invented UTC time; the event timestamp remains unavailable.

## Trace behavior and limitation

The importer stores one metadata event per chat heading and never stores the
heading or message content. Tokens and cost are `partial`: they are available
only for turns where Aider wrote the explicit summary. Aider's Markdown does
not expose a stable subagent or tool-call schema. Command guarding is only
available when the command is run through `trc run`; the history importer is
read-only.

Fixture and expected output: `tests/fixtures/agents/aider/basic.chat.history.md`
and `tests/fixtures/agents/aider/expected.json`.
