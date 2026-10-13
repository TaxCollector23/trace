import type {
  AnalyticsSummary,
  CostSource,
  RunSummary,
  UsageBreakdown,
  UsageEvent,
  UsagePagination,
  UsageResponse,
  UsageSeriesPoint,
  UsageSession,
  UsageSubagent,
  UsageTotals,
} from "./api";
import { fetchResource, v4, type Resource } from "./data";

export type UsageRange = "today" | "7d" | "30d";
type CanonicalGroup = "model" | "agent" | "project" | "day" | "session";
const CANONICAL_GROUPS: CanonicalGroup[] = ["model", "agent", "project", "day", "session"];

const EMPTY_TOKEN_TOTALS = {
  input_tokens: null,
  output_tokens: null,
  cache_read_tokens: null,
  cache_write_tokens: null,
  reasoning_tokens: null,
};

export function emptyTotals(): UsageTotals {
  return {
    ...EMPTY_TOKEN_TOTALS,
    cost_usd: null,
    cost_source: "unknown",
    event_count: null,
    priced_event_count: null,
    unknown_event_count: null,
    session_count: null,
    priced_session_count: null,
    unknown_session_count: null,
  };
}

export function costSourceLabel(source: CostSource): string {
  return source === "unknown" ? "unavailable" : source;
}

/** A cost source is evidence, not decoration. This badge is used beside every
 * rendered cost number, including aggregate cards and session rows. */
export function CostSourceBadge({ source }: { source: CostSource }) {
  return (
    <span
      className={`usage-source usage-source-${source}`}
      title={
        source === "unknown"
          ? "Trace does not have a verified price for this value"
          : `Cost ${source}`
      }
    >
      {costSourceLabel(source)}
    </span>
  );
}

export function isDevFixtureMode(): boolean {
  return import.meta.env.DEV && import.meta.env.VITE_TRACE_FIXTURES === "true";
}

/**
 * Load one server-aggregated usage page. The endpoint is intentionally
 * read-only and cursor/page based: the browser never scans the usage_events
 * table and never recomputes pricing.
 */
export async function loadUsage(
  range: UsageRange,
  sessionId?: string,
  signal?: AbortSignal,
  cursor?: string
): Promise<Resource<UsageResponse>> {
  if (isDevFixtureMode()) {
    return loadDevUsageFixture(range, sessionId, signal);
  }

  const query = new URLSearchParams({
    range,
    page_size: "50",
    by: sessionId ? "session" : "model",
    since: sinceForRange(range),
  });
  if (sessionId) query.set("session_id", sessionId);
  if (cursor) query.set("cursor", cursor);
  const live = await v4.usage(query.toString(), signal);

  if (live.state === "ok") {
    try {
      const root = unwrap(live.data);
      if (isCanonicalAggregate(root)) {
        return loadCanonicalUsage(range, sessionId, signal, root);
      }
      return { state: "ok", data: normalizeUsage(live.data, range, "live") };
    } catch (error) {
      return {
        state: "unavailable",
        kind: "corrupt",
        reason: `The daemon returned an invalid usage response: ${String(error)}`,
      };
    }
  }

  // Existing released daemons predate usage_events. Keep their token view
  // useful, but do not inherit their unlabeled cost total as if its provenance
  // were known. The warning is visible on the page.
  // Older daemon builds route unknown /api paths to the static app shell and
  // therefore surface as a corrupt (HTML) response rather than a 404. The
  // legacy path is still explicit about which fields are unavailable.
  if (
    live.state === "not_instrumented" ||
    (live.state === "unavailable" && live.kind === "corrupt")
  ) {
    return loadLegacyUsage(range, signal);
  }
  return preserveResourceFailure(live);
}

/**
 * The current daemon exposes one aggregate dimension per GET /api/usage call:
 * { by, rows, totals, import }. Requesting the dimensions separately keeps
 * aggregation in SQLite and lets the browser render truthful splits without
 * downloading or reducing the usage_events table itself.
 */
async function loadCanonicalUsage(
  range: UsageRange,
  sessionId: string | undefined,
  signal: AbortSignal | undefined,
  first: Record<string, unknown>
): Promise<Resource<UsageResponse>> {
  const firstGroup = (stringOrNull(first.by) as CanonicalGroup | null) ?? "model";
  const responses = new Map<CanonicalGroup, Record<string, unknown>>([[firstGroup, first]]);
  const failures: string[] = [];
  const groups = sessionId ? ["session" as const] : CANONICAL_GROUPS;

  const remaining = groups.filter((group) => !responses.has(group));
  const loaded = await Promise.all(
    remaining.map(async (group) => {
      const query = new URLSearchParams({
        range,
        page_size: "50",
        by: group,
        since: sinceForRange(range),
      });
      const result = await v4.usage(query.toString(), signal);
      return [group, result] as const;
    })
  );
  for (const [group, result] of loaded) {
    if (result.state === "ok") {
      const root = unwrap(result.data);
      if (isCanonicalAggregate(root)) responses.set(group, root);
      else failures.push(`${group} split returned an unexpected response shape`);
    } else if (result.state !== "loading") {
      failures.push(`${group} split is unavailable`);
    }
  }

  const model = responses.get("model") ?? first;
  const agent = responses.get("agent");
  const project = responses.get("project");
  const day = responses.get("day");
  const session = responses.get("session");
  const totals = normalizeTotals(asRecord(model.totals) ?? {});
  const sessions = session ? canonicalSessions(firstArray(session.rows), sessionId) : [];

  if (sessions.length) {
    totals.session_count = sessions.length;
    totals.unknown_session_count = sessions.filter((row) => row.unknown_event_count !== 0).length;
    totals.priced_session_count = sessions.length - (totals.unknown_session_count ?? 0);
  }

  const selectedSession = sessionId ? sessions.find((row) => row.session_id === sessionId) : null;
  const responseTotals = selectedSession ? { ...selectedSession } : totals;
  const importWarning = importWarningFrom(model.import);
  const warning = [importWarning, ...failures].filter(Boolean).join(" ") || null;

  return {
    state: "ok",
    data: {
      range,
      source: "live",
      totals: responseTotals,
      by_agent: agent ? normalizeBreakdowns(firstArray(agent.rows), "agent") : [],
      by_model: normalizeBreakdowns(firstArray(model.rows), "model"),
      by_project: project ? normalizeBreakdowns(firstArray(project.rows), "project") : [],
      over_time: day ? normalizeSeries(firstArray(day.rows)) : [],
      sessions: selectedSession ? [selectedSession] : sessions,
      events: [],
      pagination: {
        page_size: null,
        total_events: totals.event_count,
        has_more: false,
        next_cursor: null,
      },
      warning,
    },
  };
}

async function loadDevUsageFixture(
  range: UsageRange,
  sessionId: string | undefined,
  signal?: AbortSignal
): Promise<Resource<UsageResponse>> {
  const query = new URLSearchParams({ range });
  if (sessionId) query.set("session_id", sessionId);
  try {
    const response = await fetch(`/__trace-fixtures/usage?${query.toString()}`, { signal });
    if (!response.ok) {
      return { state: "unavailable", kind: "server", reason: `Fixture server returned ${response.status}.` };
    }
    return { state: "ok", data: normalizeUsage(await response.json(), range, "synthetic") };
  } catch (error) {
    if ((error as { name?: string })?.name === "AbortError") return { state: "loading" };
    return { state: "unavailable", kind: "server", reason: `Fixture data could not be loaded: ${String(error)}` };
  }
}

async function loadLegacyUsage(
  range: UsageRange,
  signal?: AbortSignal
): Promise<Resource<UsageResponse>> {
  const analytics = fetchResource<AnalyticsSummary>("/analytics", {
    emptyIsEmpty: false,
    signal,
    validate: (value: unknown): value is AnalyticsSummary => {
      const v = asRecord(value);
      return v !== null && Array.isArray(v.by_agent) && typeof v.total_runs === "number";
    },
  });
  const runs = fetchResource<RunSummary[]>("/runs?limit=50", {
    signal,
    validate: (value: unknown): value is RunSummary[] => Array.isArray(value),
  });
  const [analyticsRes, runsRes] = await Promise.all([analytics, runs]);

  if (analyticsRes.state !== "ok") return preserveResourceFailure(analyticsRes);
  const runRows = runsRes.state === "ok" ? runsRes.data : [];
  return {
    state: "ok",
    data: legacyResponse(analyticsRes.data, runRows, range),
  };
}

function legacyResponse(
  analytics: AnalyticsSummary,
  runs: RunSummary[],
  range: UsageRange
): UsageResponse {
  const byAgent = analytics.by_agent.map((row) => {
    const tokens = {
      input_tokens: row.input_tokens,
      output_tokens: row.output_tokens,
      cache_read_tokens: null,
      cache_write_tokens: null,
      reasoning_tokens: null,
    };
    return {
      ...emptyTotals(),
      ...tokens,
      key: row.agent_name,
      label: row.agent_name,
      event_ids: [],
      session_count: row.run_count,
      unknown_session_count: row.run_count,
    } satisfies UsageBreakdown;
  });

  const input = sumKnown(byAgent.map((row) => row.input_tokens));
  const output = sumKnown(byAgent.map((row) => row.output_tokens));
  const sessions = runs.map((run) => ({
    ...emptyTotals(),
    session_id: run.id,
    run_id: run.id,
    project_id: run.project_id,
    project_name: run.project_name,
    agent: run.agent_name,
    model: null,
    started_at: run.started_at,
    ended_at: run.ended_at,
    input_tokens: null,
    output_tokens: null,
    subagents: [],
    event_ids: [],
  })) satisfies UsageSession[];

  return {
    range,
    source: "legacy",
    totals: {
      ...emptyTotals(),
      input_tokens: input,
      output_tokens: output,
      session_count: analytics.total_runs,
      unknown_session_count: analytics.total_runs,
    },
    by_agent: byAgent,
    by_model: [],
    by_project: [],
    over_time: [],
    sessions,
    events: [],
    pagination: {
      page_size: 50,
      total_events: null,
      has_more: runs.length >= 50,
      next_cursor: null,
    },
    warning:
      "This daemon predates the usage_events API. Token totals are available; cost provenance, model/project splits, event links, and time aggregates are unavailable until the daemon is upgraded.",
  };
}

export function normalizeUsage(
  raw: unknown,
  range: UsageRange,
  source: UsageResponse["source"] = "live"
): UsageResponse {
  const root = unwrap(raw);
  const summary = asRecord(root.summary) ?? asRecord(root.totals) ?? root;
  const events = normalizeEvents(firstArray(root.events, root.items)).slice(0, 100);
  const totals = normalizeTotals(summary);
  const sessions = normalizeSessions(
    firstArray(root.sessions, root.most_expensive_sessions, root.top_sessions)
  ).slice(0, 50);
  const pagination = normalizePagination(root.pagination ?? root);

  return {
    range,
    source,
    totals,
    by_agent: normalizeBreakdowns(firstArray(root.by_agent, root.agents), "agent"),
    by_model: normalizeBreakdowns(firstArray(root.by_model, root.models), "model"),
    by_project: normalizeBreakdowns(firstArray(root.by_project, root.projects), "project"),
    over_time: normalizeSeries(
      firstArray(root.over_time, root.by_day, root.time_series, root.cost_over_time)
    ),
    sessions,
    events,
    pagination,
    warning: stringOrNull(root.warning) ?? warningFrom(root.warnings),
  };
}

function normalizeTotals(raw: Record<string, unknown>): UsageTotals {
  const totals = emptyTotals();
  const cost = firstNumber(raw.cost_usd, raw.total_cost, raw.total_estimated);
  const eventCount = firstNumber(raw.event_count, raw.events, raw.total_events);
  const pricedEventCount = firstNumber(
    raw.priced_event_count,
    raw.known_event_count,
    raw.priced_events
  );
  const unknownEventCount = firstNumber(
    raw.unknown_event_count,
    raw.unknown_events,
    raw.unpriced_events,
    raw.unknown_pricing_events,
    raw.unavailable_cost_events
  );
  const sessionCount = firstNumber(raw.session_count, raw.sessions, raw.total_sessions);
  const pricedSessionCount = firstNumber(
    raw.priced_session_count,
    raw.known_session_count,
    raw.priced_sessions
  );
  const unknownSessionCount = firstNumber(
    raw.unknown_session_count,
    raw.unknown_sessions,
    raw.unpriced_sessions
  );

  totals.input_tokens = firstNumber(raw.input_tokens, raw.input);
  totals.output_tokens = firstNumber(raw.output_tokens, raw.output);
  totals.cache_read_tokens = firstNumber(raw.cache_read_tokens, raw.cache_read, raw.cached_tokens);
  totals.cache_write_tokens = firstNumber(raw.cache_write_tokens, raw.cache_write);
  totals.reasoning_tokens = firstNumber(raw.reasoning_tokens, raw.reasoning);
  totals.cost_usd = cost;
  totals.cost_source = normalizeCostSource(raw.cost_source, raw.sources);
  totals.event_count = eventCount;
  totals.priced_event_count =
    pricedEventCount ??
    (eventCount !== null && unknownEventCount !== null
      ? Math.max(0, eventCount - unknownEventCount)
      : null);
  totals.unknown_event_count =
    unknownEventCount ??
    (eventCount !== null && pricedEventCount !== null
      ? Math.max(0, eventCount - pricedEventCount)
      : null);
  totals.session_count = sessionCount;
  totals.priced_session_count = pricedSessionCount;
  totals.unknown_session_count =
    unknownSessionCount ??
    (sessionCount !== null && pricedSessionCount !== null
      ? Math.max(0, sessionCount - pricedSessionCount)
      : null);
  return totals;
}

function canonicalSessions(rows: Record<string, unknown>[], requestedId?: string): UsageSession[] {
  const grouped = new Map<string, { rows: UsageBreakdown[]; subagents: UsageSubagent[] }>();
  for (const row of normalizeBreakdowns(rows, "session")) {
    const [sessionId, subagentId] = row.key.split(" / subagent:", 2);
    if (requestedId && sessionId !== requestedId) continue;
    const group = grouped.get(sessionId) ?? { rows: [], subagents: [] };
    if (subagentId) {
      group.subagents.push({
        ...row,
        subagent_id: subagentId,
        parent_subagent_id: null,
        children: [],
      });
    } else {
      group.rows.push(row);
    }
    grouped.set(sessionId, group);
  }

  return [...grouped.entries()].map(([sessionId, group]) => {
    const totals = mergeTotals(group.rows.length ? group.rows : group.subagents);
    totals.session_count = 1;
    totals.unknown_session_count =
      totals.unknown_event_count === null ? null : totals.unknown_event_count > 0 ? 1 : 0;
    totals.priced_session_count =
      totals.unknown_session_count === null ? null : totals.unknown_session_count ? 0 : 1;
    return {
      ...totals,
      session_id: sessionId,
      run_id: null,
      project_id: null,
      project_name: null,
      agent: null,
      model: null,
      started_at: null,
      ended_at: null,
      subagents: group.subagents,
      event_ids: [],
    };
  });
}

function mergeTotals(rows: UsageBreakdown[]): UsageTotals {
  const totals = emptyTotals();
  totals.input_tokens = sumKnown(rows.map((row) => row.input_tokens));
  totals.output_tokens = sumKnown(rows.map((row) => row.output_tokens));
  totals.cache_read_tokens = sumKnown(rows.map((row) => row.cache_read_tokens));
  totals.cache_write_tokens = sumKnown(rows.map((row) => row.cache_write_tokens));
  totals.reasoning_tokens = sumKnown(rows.map((row) => row.reasoning_tokens));
  totals.event_count = sumKnown(rows.map((row) => row.event_count));
  totals.unknown_event_count = sumKnown(rows.map((row) => row.unknown_event_count));
  totals.priced_event_count =
    totals.event_count !== null && totals.unknown_event_count !== null
      ? Math.max(0, totals.event_count - totals.unknown_event_count)
      : null;
  const hasUnknownCost = rows.some(
    (row) => row.cost_usd === null || row.cost_source === "unknown" || (row.unknown_event_count ?? 0) > 0
  );
  totals.cost_usd = hasUnknownCost ? null : sumKnown(rows.map((row) => row.cost_usd));
  totals.cost_source = combineCostSources(rows.map((row) => row.cost_source));
  return totals;
}

function combineCostSources(sources: CostSource[]): CostSource {
  const distinct = [...new Set(sources.filter((source) => source !== "unknown"))];
  if (sources.some((source) => source === "unknown")) return distinct.length ? "mixed" : "unknown";
  if (distinct.length === 1) return distinct[0];
  return distinct.length ? "mixed" : "unknown";
}

function isCanonicalAggregate(root: Record<string, unknown>): boolean {
  return typeof root.by === "string" && Array.isArray(root.rows) && asRecord(root.totals) !== null;
}

function sinceForRange(range: UsageRange): string {
  const days = range === "today" ? 1 : range === "7d" ? 7 : 30;
  return new Date(Date.now() - days * 24 * 60 * 60 * 1000).toISOString();
}

function importWarningFrom(value: unknown): string | null {
  const report = asRecord(value);
  if (!report) return null;
  const malformed = firstNumber(report.malformed_lines, report.parse_errors);
  return malformed && malformed > 0
    ? `${malformed.toLocaleString()} source lines could not be parsed; totals may be partial.`
    : null;
}

function normalizeBreakdowns(rawRows: Record<string, unknown>[], kind: string): UsageBreakdown[] {
  return rawRows.map((raw) => {
    const key =
      stringOrNull(raw.key) ??
      stringOrNull(raw.id) ??
      stringOrNull(raw[kind]) ??
      stringOrNull(raw.name) ??
      "unknown";
    const label = stringOrNull(raw.label) ?? stringOrNull(raw.name) ?? key;
    return {
      ...normalizeTotals(raw),
      key,
      label,
      event_ids: stringArray(raw.event_ids ?? raw.eventIds),
      api_equivalent_usd: firstNumber(raw.api_equivalent_usd),
    };
  });
}

function normalizeSeries(rawRows: Record<string, unknown>[]): UsageSeriesPoint[] {
  return rawRows.map((raw) => ({
    ...normalizeTotals(raw),
    bucket:
      stringOrNull(raw.bucket) ??
      stringOrNull(raw.date) ??
      stringOrNull(raw.day) ??
      stringOrNull(raw.ts) ??
      "unknown",
    label:
      stringOrNull(raw.label) ??
      stringOrNull(raw.date) ??
      stringOrNull(raw.day) ??
      "unknown",
  }));
}

function normalizeSessions(rawRows: Record<string, unknown>[]): UsageSession[] {
  return rawRows.map((raw) => ({
    ...normalizeTotals(raw),
    session_id: stringOrNull(raw.session_id) ?? stringOrNull(raw.id) ?? "unknown-session",
    run_id: stringOrNull(raw.run_id),
    project_id: stringOrNull(raw.project_id),
    project_name: stringOrNull(raw.project_name) ?? stringOrNull(raw.project),
    agent: stringOrNull(raw.agent) ?? stringOrNull(raw.agent_name),
    model: stringOrNull(raw.model),
    started_at: stringOrNull(raw.started_at) ?? stringOrNull(raw.start_ts),
    ended_at: stringOrNull(raw.ended_at) ?? stringOrNull(raw.end_ts),
    subagents: normalizeSubagents(firstArray(raw.subagents, raw.children)),
    event_ids: stringArray(raw.event_ids ?? raw.eventIds),
  }));
}

function normalizeSubagents(rawRows: Record<string, unknown>[]): UsageSubagent[] {
  const nodes: UsageSubagent[] = rawRows.map((raw) => ({
    ...normalizeBreakdown(raw, "subagent"),
    subagent_id:
      stringOrNull(raw.subagent_id) ?? stringOrNull(raw.id) ?? stringOrNull(raw.key) ?? "unknown",
    parent_subagent_id:
      stringOrNull(raw.parent_subagent_id) ?? stringOrNull(raw.parent_id),
    children: [] as UsageSubagent[],
  }));
  const byId = new Map(nodes.map((node) => [node.subagent_id, node]));
  const roots: UsageSubagent[] = [];
  for (const node of nodes) {
    const parent = node.parent_subagent_id ? byId.get(node.parent_subagent_id) : undefined;
    if (parent && parent !== node) parent.children.push(node);
    else roots.push(node);
  }
  return roots;
}

function normalizeBreakdown(raw: Record<string, unknown>, kind: string): UsageBreakdown {
  return normalizeBreakdowns([raw], kind)[0];
}

function normalizeEvents(rawRows: Record<string, unknown>[]): UsageEvent[] {
  return rawRows.map((raw, index) => {
    const cost = firstNumber(raw.cost_usd, raw.cost, raw.estimated_cost);
    return {
      id: stringOrNull(raw.id) ?? stringOrNull(raw.event_id) ?? `event-${index + 1}`,
      run_id: stringOrNull(raw.run_id),
      agent: stringOrNull(raw.agent) ?? stringOrNull(raw.agent_name),
      session_id: stringOrNull(raw.session_id),
      message_id: stringOrNull(raw.message_id),
      parent_message_id: stringOrNull(raw.parent_message_id) ?? stringOrNull(raw.parent_id),
      is_subagent: raw.is_subagent === true || raw.isSubagent === true,
      subagent_id: stringOrNull(raw.subagent_id),
      model: stringOrNull(raw.model),
      input_tokens: firstNumber(raw.input_tokens, raw.input),
      output_tokens: firstNumber(raw.output_tokens, raw.output),
      cache_read_tokens: firstNumber(raw.cache_read_tokens, raw.cache_read, raw.cached_tokens),
      cache_write_tokens: firstNumber(raw.cache_write_tokens, raw.cache_write),
      reasoning_tokens: firstNumber(raw.reasoning_tokens, raw.reasoning),
      ts: stringOrNull(raw.ts) ?? stringOrNull(raw.created_at),
      cost_usd: cost,
      cost_source: cost === null ? "unknown" : normalizeEventCostSource(raw.cost_source),
      raw_ref: stringOrNull(raw.raw_ref),
    };
  });
}

function normalizePagination(raw: unknown): UsagePagination {
  const value = asRecord(raw) ?? {};
  return {
    page_size: firstNumber(value.page_size, value.limit),
    total_events: firstNumber(value.total_events, value.event_count),
    has_more: value.has_more === true || value.hasMore === true,
    next_cursor: stringOrNull(value.next_cursor) ?? stringOrNull(value.nextCursor),
  };
}

function normalizeCostSource(value: unknown, sources: unknown): CostSource {
  const direct = normalizeSourceValue(value);
  if (direct) return direct;
  if (Array.isArray(sources)) {
    const unique = sources.map(normalizeSourceValue).filter(Boolean) as CostSource[];
    const distinct = [...new Set(unique)];
    if (distinct.length === 1) return distinct[0];
    if (distinct.length > 1) return "mixed";
  }
  return "unknown";
}

function normalizeEventCostSource(value: unknown): Exclude<CostSource, "mixed"> {
  const source = normalizeSourceValue(value);
  return source === "reported" || source === "computed" ? source : "unknown";
}

function normalizeSourceValue(value: unknown): CostSource | null {
  return value === "reported" || value === "computed" || value === "unknown" || value === "mixed"
    ? value
    : null;
}

function preserveResourceFailure<T>(resource: Resource<T>): Resource<UsageResponse> {
  if (resource.state === "loading") return { state: "loading" };
  if (resource.state === "ok") return { state: "ok", data: resource.data as UsageResponse };
  return resource;
}

function unwrap(raw: unknown): Record<string, unknown> {
  const root = asRecord(raw);
  if (!root) throw new Error("response was not an object");
  const data = asRecord(root.data);
  return data ?? root;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function firstArray(...values: unknown[]): Record<string, unknown>[] {
  for (const value of values) {
    if (Array.isArray(value)) return value.filter((item): item is Record<string, unknown> => asRecord(item) !== null);
  }
  return [];
}

function firstNumber(...values: unknown[]): number | null {
  for (const value of values) {
    if (typeof value === "number" && Number.isFinite(value)) return value;
  }
  return null;
}

function sumKnown(values: Array<number | null>): number | null {
  const known = values.filter((value): value is number => value !== null);
  return known.length ? known.reduce((sum, value) => sum + value, 0) : null;
}

function stringOrNull(value: unknown): string | null {
  return typeof value === "string" && value.length ? value : null;
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : [];
}

function warningFrom(value: unknown): string | null {
  if (!Array.isArray(value)) return null;
  const warnings = value.filter((item): item is string => typeof item === "string");
  return warnings.length ? warnings.join(" ") : null;
}
