import type {
  CostSource,
  UsageBreakdown,
  UsageEvent,
  UsageResponse,
  UsageSession,
  UsageSeriesPoint,
  UsageSubagent,
  UsageTotals,
} from "./api";
import type { IntegrationCoverage } from "./data";
import { emptyTotals, type UsageRange } from "./usage";

type FixtureEvent = UsageEvent & { project_name: string };
type FixtureEventInput = Pick<
  FixtureEvent,
  | "id"
  | "run_id"
  | "session_id"
  | "agent"
  | "model"
  | "project_name"
  | "message_id"
  | "input_tokens"
  | "output_tokens"
  | "cost_usd"
  | "cost_source"
> &
  Partial<Pick<FixtureEvent, "parent_message_id" | "is_subagent" | "subagent_id" | "cache_read_tokens" | "cache_write_tokens" | "reasoning_tokens">> & {
    hoursAgo: number;
  };

/**
 * Small, explicit synthetic fixture data for local UI development and smoke
 * tests. This module is loaded only from the import.meta.env.DEV fixture path;
 * it is not a production data source.
 */
const FIXTURE_EVENTS: FixtureEvent[] = [
  event({
    id: "fixture-event-claude-main",
    hoursAgo: 2,
    run_id: "fixture-run-claude",
    session_id: "fixture-session-claude",
    agent: "claude",
    model: "claude-3-5-sonnet-20241022",
    project_name: "trace",
    message_id: "fixture-message-claude-main",
    input_tokens: 5_000,
    output_tokens: 2_500,
    cache_read_tokens: 1_000,
    cache_write_tokens: 100,
    cost_usd: 4.2,
    cost_source: "reported",
  }),
  event({
    id: "fixture-event-claude-subagent",
    hoursAgo: 1.5,
    run_id: "fixture-run-claude",
    session_id: "fixture-session-claude",
    agent: "claude",
    model: "claude-3-5-sonnet-20241022",
    project_name: "trace",
    message_id: "fixture-message-claude-subagent",
    parent_message_id: "fixture-message-claude-main",
    is_subagent: true,
    subagent_id: "fixture-subagent-build",
    input_tokens: 1_200,
    output_tokens: 600,
    cache_read_tokens: 0,
    cache_write_tokens: 0,
    cost_usd: 1.8,
    cost_source: "computed",
  }),
  event({
    id: "fixture-event-codex",
    hoursAgo: 26,
    run_id: "fixture-run-codex",
    session_id: "fixture-session-codex",
    agent: "codex",
    model: "gpt-5-codex",
    project_name: "nomo-engine",
    message_id: "fixture-message-codex",
    input_tokens: 3_000,
    output_tokens: 900,
    reasoning_tokens: 400,
    cost_usd: 6.4,
    cost_source: "computed",
  }),
  event({
    id: "fixture-event-cursor-unknown-price",
    hoursAgo: 72,
    run_id: "fixture-run-cursor",
    session_id: "fixture-session-cursor",
    agent: "cursor",
    model: "cursor-unpriced-model",
    project_name: "trace",
    message_id: "fixture-message-cursor",
    input_tokens: 800,
    output_tokens: 300,
    cost_usd: null,
    cost_source: "unknown",
  }),
  event({
    id: "fixture-event-claude-unknown-model",
    hoursAgo: 120,
    run_id: "fixture-run-claude-unknown",
    session_id: "fixture-session-unknown-model",
    agent: "claude",
    model: "future-unknown-model",
    project_name: "nomo-engine",
    message_id: "fixture-message-unknown-model",
    input_tokens: 1_100,
    output_tokens: 400,
    cost_usd: null,
    cost_source: "unknown",
  }),
];

export function createUsageFixture(range: UsageRange, sessionId?: string): UsageResponse {
  const start = rangeStart(range);
  const events = FIXTURE_EVENTS.filter((row) => {
    const ts = row.ts ? new Date(row.ts).getTime() : Number.NaN;
    return ts >= start && (!sessionId || row.session_id === sessionId || row.run_id === sessionId);
  });
  return aggregateFixture(events, range);
}

export function createConnectionsFixture(): IntegrationCoverage[] {
  return [
    {
      agent: "claude",
      display_name: "Claude Code",
      installed: true,
      connected: true,
      receiving_events: true,
      last_event_at: FIXTURE_EVENTS[0].ts,
      session_data_path: "~/.claude/projects/",
      events_imported: 2,
      parse_errors: 0,
      command_enforcement: true,
      file_review: false,
      status: "connected",
      note: "Synthetic development fixture",
      capabilities: { tokens: true, cost: true, subagents: true, tool_calls: true, command_guard: true },
    },
    {
      agent: "codex",
      display_name: "Codex",
      installed: false,
      connected: false,
      receiving_events: false,
      last_event_at: null,
      session_data_path: "~/.codex/sessions/",
      events_imported: 0,
      parse_errors: null,
      command_enforcement: true,
      file_review: false,
      status: "not_installed",
      note: "Install Codex, run one task, then refresh.",
      capabilities: { tokens: null, cost: null, subagents: null, tool_calls: true, command_guard: true },
    },
    {
      agent: "cursor",
      display_name: "Cursor",
      installed: true,
      connected: false,
      receiving_events: false,
      last_event_at: null,
      session_data_path: null,
      events_imported: 0,
      parse_errors: null,
      command_enforcement: true,
      file_review: false,
      status: "not_connected",
      note: "Trace has not found a connected Cursor hook.",
      capabilities: { tokens: null, cost: null, subagents: null, tool_calls: null, command_guard: true },
    },
  ];
}

function event(input: FixtureEventInput): FixtureEvent {
  const { hoursAgo, ...row } = input;
  return {
    ...row,
    parent_message_id: row.parent_message_id ?? null,
    is_subagent: row.is_subagent ?? false,
    subagent_id: row.subagent_id ?? null,
    cache_read_tokens: row.cache_read_tokens ?? null,
    cache_write_tokens: row.cache_write_tokens ?? null,
    reasoning_tokens: row.reasoning_tokens ?? null,
    ts: new Date(Date.now() - hoursAgo * 60 * 60 * 1000).toISOString(),
    raw_ref: `synthetic://usage/${row.id}`,
  };
}

function aggregateFixture(events: FixtureEvent[], range: UsageRange): UsageResponse {
  const sessions = [...new Set(events.map((row) => row.session_id).filter(Boolean))].map((id) =>
    buildSession(id as string, events.filter((row) => row.session_id === id))
  );
  const totals = totalsFor(events);
  totals.session_count = sessions.length;
  totals.priced_session_count = sessions.filter((row) => row.cost_usd !== null && row.unknown_event_count === 0).length;
  totals.unknown_session_count = sessions.filter((row) => row.unknown_event_count !== 0).length;

  return {
    range,
    source: "synthetic",
    totals,
    by_agent: groupBy(events, (row) => row.agent ?? "unknown", (row) => row.agent ?? "Unattributed"),
    by_model: groupBy(events, (row) => row.model ?? "unknown", (row) => row.model ?? "Unknown model"),
    by_project: groupBy(events, (row) => row.project_name, (row) => row.project_name),
    over_time: timeSeries(events),
    sessions: sessions.sort(compareCostDescending),
    events: events.map(({ project_name: _projectName, ...row }) => row),
    pagination: {
      page_size: events.length,
      total_events: events.length,
      has_more: false,
      next_cursor: null,
    },
    warning: "Synthetic development data. It is never loaded in production builds.",
  };
}

function totalsFor(events: FixtureEvent[]): UsageTotals {
  const totals = emptyTotals();
  totals.input_tokens = sum(events.map((row) => row.input_tokens));
  totals.output_tokens = sum(events.map((row) => row.output_tokens));
  totals.cache_read_tokens = sum(events.map((row) => row.cache_read_tokens));
  totals.cache_write_tokens = sum(events.map((row) => row.cache_write_tokens));
  totals.reasoning_tokens = sum(events.map((row) => row.reasoning_tokens));
  const priced = events.filter((row) => row.cost_usd !== null);
  totals.cost_usd = priced.length ? priced.reduce((total, row) => total + (row.cost_usd ?? 0), 0) : null;
  totals.cost_source = sourceFor(events.map((row) => row.cost_source));
  totals.event_count = events.length;
  totals.priced_event_count = priced.length;
  totals.unknown_event_count = events.length - priced.length;
  return totals;
}

function groupBy(
  events: FixtureEvent[],
  keyOf: (event: FixtureEvent) => string,
  labelOf: (event: FixtureEvent) => string
): UsageBreakdown[] {
  const groups = new Map<string, FixtureEvent[]>();
  for (const row of events) {
    const key = keyOf(row);
    groups.set(key, [...(groups.get(key) ?? []), row]);
  }
  return [...groups.entries()]
    .map(([key, rows]) => ({
      ...totalsFor(rows),
      key,
      label: labelOf(rows[0]),
      event_ids: rows.map((row) => row.id),
    }))
    .sort(compareCostDescending);
}

function timeSeries(events: FixtureEvent[]): UsageSeriesPoint[] {
  const groups = new Map<string, FixtureEvent[]>();
  for (const row of events) {
    const bucket = row.ts?.slice(0, 10) ?? "unknown";
    groups.set(bucket, [...(groups.get(bucket) ?? []), row]);
  }
  return [...groups.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([bucket, rows]) => ({
      ...totalsFor(rows),
      bucket,
      label: bucket === "unknown" ? "Unknown date" : bucket,
    }));
}

function buildSession(sessionId: string, events: FixtureEvent[]): UsageSession {
  const totals = totalsFor(events);
  const subagents = [...new Set(events.filter((row) => row.is_subagent).map((row) => row.subagent_id).filter(Boolean))].map(
    (subagentId) => {
      const rows = events.filter((row) => row.subagent_id === subagentId);
      return {
        ...totalsFor(rows),
        key: subagentId as string,
        label: subagentId as string,
        event_ids: rows.map((row) => row.id),
        subagent_id: subagentId as string,
        parent_subagent_id: null,
        children: [],
      } satisfies UsageSubagent;
    }
  );
  const timestamps = events.map((row) => row.ts).filter((value): value is string => Boolean(value)).sort();
  return {
    ...totals,
    session_id: sessionId,
    run_id: events[0]?.run_id ?? null,
    project_id: null,
    project_name: events[0]?.project_name ?? null,
    agent: events[0]?.agent ?? null,
    model: events[0]?.model ?? null,
    started_at: timestamps[0] ?? null,
    ended_at: timestamps[timestamps.length - 1] ?? null,
    subagents,
    event_ids: events.map((row) => row.id),
  };
}

function rangeStart(range: UsageRange): number {
  const days = range === "today" ? 1 : range === "7d" ? 7 : 30;
  return Date.now() - days * 24 * 60 * 60 * 1000;
}

function sum(values: Array<number | null>): number | null {
  const known = values.filter((value): value is number => value !== null);
  return known.length ? known.reduce((total, value) => total + value, 0) : null;
}

function sourceFor(values: Array<Exclude<CostSource, "mixed">>): CostSource {
  const distinct = [...new Set(values)];
  return distinct.length === 1 ? distinct[0] : distinct.length ? "mixed" : "unknown";
}

function compareCostDescending(a: { cost_usd: number | null }, b: { cost_usd: number | null }): number {
  if (a.cost_usd === null && b.cost_usd === null) return 0;
  if (a.cost_usd === null) return 1;
  if (b.cost_usd === null) return -1;
  return b.cost_usd - a.cost_usd;
}
