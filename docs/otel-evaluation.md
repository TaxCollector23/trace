# OTel evaluation: Claude Code and Codex

Evaluated 2026-10-12 against the current official documentation and local
files. This is an evaluation, not a claim that Trace has a hosted telemetry
collector.

## Claude Code

[Claude Code usage monitoring](https://code.claude.com/docs/en/monitoring-usage)
documents OpenTelemetry metrics, logs, and traces. Telemetry is opt-in through
`CLAUDE_CODE_ENABLE_TELEMETRY=1`; exporters, OTLP protocol, and endpoint are
configured with the documented `OTEL_*` variables. Prompt, assistant response,
tool-detail, and raw API-body logging are separately gated because they can
expose sensitive source and conversation content. The trace hierarchy can
represent subagent work under a tool span.

Trace does not enable or collect this automatically. The existing local
Claude importer is deterministic and content-free; OTel would require an
endpoint lifecycle, resource/tenant policy, batching, shutdown behavior, and
an explicit privacy contract. That is larger than a small, testable local
change for this workstream.

## Codex

[Codex's official OTel README](https://github.com/openai/codex/blob/main/codex-rs/otel/README.md)
documents `codex-otel` exporter configuration for logs, traces, and metrics.
`otel.log_agent_responses` is opt-in and can expose response text, including
spawned subagents. The local Codex rollout JSONL inspected on this machine is
already sufficient for a content-free token importer.

Trace therefore does not implement OTel ingestion for Codex in this release.
The local JSONL path is bounded, resumable, fixture-tested, and keeps raw
content out of the canonical event table. A future OTel adapter should only be
added with an in-process collector contract, redaction tests, exporter failure
tests, and explicit opt-in configuration.

## Decision

No OTel implementation is added here. The small, testable boundary is the
read-only local importer plus explicit source/capability status. Official OTel
protocol behavior is documented by the
[OpenTelemetry OTLP exporter specification](https://opentelemetry.io/docs/specs/otel/protocol/exporter/),
but a protocol reference alone is not evidence that a user's agent is
configured to export or that Trace can safely receive it.
