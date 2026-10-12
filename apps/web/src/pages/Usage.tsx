import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import type {
  CostSource,
  UsageBreakdown,
  UsageEvent,
  UsageResponse,
  UsageSession,
  UsageSubagent,
  UsageTotals,
} from "../api";
import { Loading, agentLabel, fmtCost, fmtNum } from "../components";
import type { Resource } from "../data";
import { useResource } from "../useResource";
import { CostSourceBadge, loadUsage, type UsageRange } from "../usage";
import { ResourceGate } from "../v4/ui";

const PERIODS: Array<{ key: UsageRange; label: string }> = [
  { key: "today", label: "Today" },
  { key: "7d", label: "7 days" },
  { key: "30d", label: "30 days" },
];

export default function Usage() {
  const { runId } = useParams<{ runId?: string }>();
  const [range, setRange] = useState<UsageRange>("30d");
  const [extraPage, setExtraPage] = useState<UsageResponse | null>(null);
  const [loadingMore, setLoadingMore] = useState(false);

  const today = useResource((signal) => loadUsage("today", undefined, signal), { pollMs: 30_000 });
  const week = useResource((signal) => loadUsage("7d", undefined, signal), { pollMs: 30_000 });
  const month = useResource((signal) => loadUsage("30d", undefined, signal), { pollMs: 30_000 });
  const detail = useResource(
    (signal) =>
      runId
        ? loadUsage(range, runId, signal)
        : Promise.resolve<Resource<UsageResponse>>({ state: "empty" }),
    { deps: [runId, range] }
  );

  useEffect(() => {
    setExtraPage(null);
  }, [runId, range]);

  const resources: Record<UsageRange, Resource<UsageResponse>> = {
    today: today.resource,
    "7d": week.resource,
    "30d": month.resource,
  };
  const overview = resources[range];
  const baseData = runId
    ? detail.resource.state === "ok"
      ? detail.resource.data
      : null
    : overview.state === "ok"
      ? overview.data
      : null;
  const data = baseData && extraPage ? mergeEventPage(baseData, extraPage) : baseData;

  async function loadMoreEvents() {
    if (!data?.pagination.next_cursor || loadingMore) return;
    setLoadingMore(true);
    try {
      const next = await loadUsage(range, runId, undefined, data.pagination.next_cursor);
      if (next.state === "ok") {
        setExtraPage((previous) => (previous ? mergeEventPage(previous, next.data) : next.data));
      }
    } finally {
      setLoadingMore(false);
    }
  }

  return (
    <div>
      <div className="usage-heading-row">
        <div>
          <h1 className="page-title">Usage</h1>
          <p className="page-sub">
            Verified token and cost records from local agent sessions. Trace keeps unknown pricing
            visible instead of turning it into zero.
          </p>
        </div>
        {runId && <Link className="back" to="/usage">← All usage</Link>}
      </div>

      <div className="usage-toolbar" role="group" aria-label="Usage range">
        {PERIODS.map((period) => (
          <button
            key={period.key}
            type="button"
            className={`btn-ghost ${range === period.key ? "active" : ""}`}
            aria-pressed={range === period.key}
            onClick={() => setRange(period.key)}
          >
            {period.label}
          </button>
        ))}
        <span className="muted usage-toolbar-note">Aggregated in the daemon · event rows are paginated</span>
      </div>

      {!runId && <PeriodCards resources={resources} />}

      {runId ? (
        <ResourceGate
          resource={detail.resource}
          what="session usage"
          onRetry={detail.reload}
          empty={<div className="empty">That session has no usage events in this range.</div>}
        >
          {(sessionData) => (
            <SessionDrilldown data={sessionData} onLoadMore={loadMoreEvents} loadingMore={loadingMore} />
          )}
        </ResourceGate>
      ) : overview.state === "loading" ? (
        <Loading variant="cards" rows={4} />
      ) : overview.state !== "ok" ? (
        <ResourceGate
          resource={overview}
          what="usage"
          onRetry={range === "today" ? today.reload : range === "7d" ? week.reload : month.reload}
        >
          {() => null}
        </ResourceGate>
      ) : data ? (
        <UsageOverview data={data} onLoadMore={loadMoreEvents} loadingMore={loadingMore} />
      ) : null}
    </div>
  );
}

function PeriodCards({ resources }: { resources: Record<UsageRange, Resource<UsageResponse>> }) {
  return (
    <div className="usage-period-grid" data-testid="usage-periods">
      {PERIODS.map((period) => {
        const resource = resources[period.key];
        return (
          <section className="usage-period" key={period.key} aria-label={`${period.label} cost`}>
            <div className="usage-eyebrow">{period.label}</div>
            {resource.state === "ok" ? (
              <>
                <CostValue value={resource.data.totals.cost_usd} source={resource.data.totals.cost_source} testId={`cost-${period.key}`} />
                <CoverageLine totals={resource.data.totals} />
              </>
            ) : resource.state === "loading" ? (
              <div className="skel skel-line" style={{ width: "65%" }} aria-label="Loading" />
            ) : (
              <div className="usage-unavailable">unavailable</div>
            )}
          </section>
        );
      })}
    </div>
  );
}

function UsageOverview({
  data,
  onLoadMore,
  loadingMore,
}: {
  data: UsageResponse;
  onLoadMore: () => void;
  loadingMore: boolean;
}) {
  return (
    <>
      <DataNotice data={data} />

      <section className="usage-panel" aria-labelledby="token-split-title">
        <PanelHeading id="token-split-title" title="Token split" subtitle="The fields the source actually reported" />
        <TokenGrid totals={data.totals} />
      </section>

      <div className="usage-two-column">
        <BreakdownTable title="Cost by agent" rows={data.by_agent} />
        <BreakdownTable title="Cost by model" rows={data.by_model} mono />
      </div>

      <div className="usage-two-column">
        <BreakdownTable title="Cost by project" rows={data.by_project} />
        <TimeSeries points={data.over_time} />
      </div>

      <SessionTable sessions={data.sessions} />
      <EventTable
        events={data.events}
        pagination={data.pagination}
        onLoadMore={onLoadMore}
        loadingMore={loadingMore}
      />
    </>
  );
}

function SessionDrilldown({
  data,
  onLoadMore,
  loadingMore,
}: {
  data: UsageResponse;
  onLoadMore: () => void;
  loadingMore: boolean;
}) {
  const session = data.sessions[0] ?? null;
  if (!session) {
    return (
      <div className="empty">
        This daemon returned the session page without a session summary. Trace will not infer one
        from the event count.
      </div>
    );
  }
  return (
    <>
      <DataNotice data={data} />
      <section className="usage-session-head" data-testid="session-drilldown">
        <div>
          <div className="usage-eyebrow">Session</div>
          <h2>{session.project_name ?? "Unattributed project"}</h2>
          <p className="muted">
            {agentLabel(session.agent ?? "unknown")} · {session.model ?? "Unknown model"} ·{" "}
            <span className="mono">{session.session_id}</span>
          </p>
        </div>
        <CostValue value={session.cost_usd} source={session.cost_source} />
      </section>
      <div className="usage-two-column">
        <section className="usage-panel">
          <PanelHeading title="Session tokens" subtitle="No token field is filled unless the source supplied it" />
          <TokenGrid totals={session} />
        </section>
        <section className="usage-panel">
          <PanelHeading title="Subagent tree" subtitle="Parent links come from usage events" />
          {session.subagents.length ? (
            <div className="usage-tree">
              {session.subagents.map((node) => <SubagentNode key={node.subagent_id} node={node} depth={0} />)}
            </div>
          ) : (
            <div className="usage-unavailable-block">
              No subagent events were recorded. This is not proof that the agent never used a
              subagent if that agent does not expose subagent metadata.
            </div>
          )}
        </section>
      </div>
      <EventTable
        events={data.events}
        pagination={data.pagination}
        onLoadMore={onLoadMore}
        loadingMore={loadingMore}
      />
    </>
  );
}

function DataNotice({ data }: { data: UsageResponse }) {
  if (data.source === "synthetic") {
    return <div className="usage-notice usage-notice-warning">Synthetic development fixture — not production data.</div>;
  }
  if (data.warning) return <div className="usage-notice">{data.warning}</div>;
  const unknown = data.totals.unknown_event_count ?? 0;
  if (unknown > 0) {
    return (
      <div className="usage-notice usage-notice-warning">
        {coverageText(data.totals)} Unknown pricing is kept out of the cost total.
      </div>
    );
  }
  return null;
}

function CoverageLine({ totals }: { totals: UsageTotals }) {
  return <div className="usage-coverage">{coverageText(totals)}</div>;
}

function coverageText(totals: UsageTotals): string {
  if (totals.cost_usd === null) {
    return "Cost unavailable: no verified price in this window.";
  }
  if (totals.session_count !== null && totals.unknown_session_count !== null && totals.unknown_session_count > 0) {
    const priced = totals.priced_session_count ?? Math.max(0, totals.session_count - totals.unknown_session_count);
    return `$${totals.cost_usd.toFixed(2)} across ${priced} of ${totals.session_count} sessions; ${totals.unknown_session_count} have unknown pricing.`;
  }
  if (totals.unknown_event_count !== null && totals.unknown_event_count > 0) {
    return `$${totals.cost_usd.toFixed(2)} from ${totals.priced_event_count ?? "some"} priced events; ${totals.unknown_event_count} events have unknown pricing.`;
  }
  return "All returned cost records have a verified source.";
}

function TokenGrid({ totals }: { totals: UsageTotals }) {
  const fields: Array<[string, number | null]> = [
    ["Input", totals.input_tokens],
    ["Output", totals.output_tokens],
    ["Cache read", totals.cache_read_tokens],
    ["Cache write", totals.cache_write_tokens],
    ["Reasoning", totals.reasoning_tokens],
  ];
  return (
    <div className="usage-token-grid">
      {fields.map(([label, value]) => (
        <div className="usage-token" key={label}>
          <div className="usage-token-value">{fmtNum(value)}</div>
          <div className="usage-token-label">{label}</div>
          <CostSourceBadge source={totals.cost_source} />
        </div>
      ))}
    </div>
  );
}

function BreakdownTable({ title, rows, mono = false }: { title: string; rows: UsageBreakdown[]; mono?: boolean }) {
  return (
    <section className="usage-panel">
      <PanelHeading title={title} subtitle="Select an event link to inspect the source record" />
      {!rows.length ? (
        <div className="usage-unavailable-block">This split is unavailable from the current daemon.</div>
      ) : (
        <div className="v4-table-wrap">
          <table className="usage-table">
            <thead>
              <tr><th>{title.replace("Cost by ", "")}</th><th>Cost</th><th>Tokens</th><th>Source events</th></tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.key}>
                  <td className={mono ? "mono" : undefined}>{row.label}</td>
                  <td><CostValue value={row.cost_usd} source={row.cost_source} /></td>
                  <td>{fmtNum(totalTokens(row))}</td>
                  <td><SourceEventLinks eventIds={row.event_ids} /></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

function TimeSeries({ points }: { points: UsageResponse["over_time"] }) {
  const max = Math.max(...points.map((point) => point.cost_usd ?? 0), 0);
  return (
    <section className="usage-panel">
      <PanelHeading title="Cost over time" subtitle="Server-aggregated day buckets" />
      {!points.length ? (
        <div className="usage-unavailable-block">Time series unavailable from the current daemon.</div>
      ) : (
        <div className="usage-series">
          {points.map((point) => (
            <div className="usage-series-row" key={point.bucket}>
              <span className="usage-series-label">{point.label}</span>
              <span className="usage-series-track">
                {point.cost_usd !== null && <span className="usage-series-bar" style={{ width: `${max ? Math.max(4, (point.cost_usd / max) * 100) : 4}%` }} />}
              </span>
              <CostValue value={point.cost_usd} source={point.cost_source} />
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function SessionTable({ sessions }: { sessions: UsageSession[] }) {
  return (
    <section className="usage-panel">
      <PanelHeading title="Most expensive sessions" subtitle="Known costs sort first; unavailable sessions remain visible" />
      {!sessions.length ? (
        <div className="usage-unavailable-block">Session aggregates are unavailable from the current daemon.</div>
      ) : (
        <div className="v4-table-wrap">
          <table className="usage-table">
            <thead><tr><th>Session</th><th>Agent</th><th>Project</th><th>Cost</th><th>Tokens</th></tr></thead>
            <tbody>
              {sessions.map((session) => (
                <tr key={session.session_id}>
                  <td><Link to={`/usage/${encodeURIComponent(session.session_id)}`} className="mono">{session.session_id}</Link></td>
                  <td>{agentLabel(session.agent ?? "unknown")}</td>
                  <td>{session.project_name ?? <span className="muted">unavailable</span>}</td>
                  <td><CostValue value={session.cost_usd} source={session.cost_source} /></td>
                  <td>{fmtNum(totalTokens(session))}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

function EventTable({
  events,
  pagination,
  onLoadMore,
  loadingMore,
}: {
  events: UsageEvent[];
  pagination: UsageResponse["pagination"];
  onLoadMore: () => void;
  loadingMore: boolean;
}) {
  return (
    <section className="usage-panel" id="usage-events" aria-labelledby="usage-events-title">
      <PanelHeading id="usage-events-title" title="Underlying usage events" subtitle="Raw prompts and responses are never rendered here" />
      {!events.length ? (
        <div className="usage-unavailable-block">Event rows are unavailable from the current daemon.</div>
      ) : (
        <>
          <div className="v4-table-wrap">
            <table className="usage-table">
              <thead><tr><th>Time</th><th>Agent / model</th><th>Tokens</th><th>Cost</th><th>Open</th></tr></thead>
              <tbody>
                {events.map((event) => (
                  <tr id={eventAnchorId(event.id)} key={event.id}>
                    <td className="mono">{event.ts ? new Date(event.ts).toLocaleString() : "unavailable"}</td>
                    <td>{agentLabel(event.agent ?? "unknown")} · <span className="mono">{event.model ?? "unknown model"}</span>{event.is_subagent && <span className="pill">subagent</span>}</td>
                    <td>{fmtNum(totalTokens(event))}</td>
                    <td><CostValue value={event.cost_usd} source={event.cost_source} /></td>
                    <td>{event.run_id ? <Link to={`/timeline/${encodeURIComponent(event.run_id)}?event=${encodeURIComponent(event.id)}`}>timeline ↗</Link> : <a href={`#${eventAnchorId(event.id)}`}>event ↗</a>}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {pagination.has_more && (
            <div className="usage-pagination">
              <span className="muted">Showing {events.length.toLocaleString()} of {pagination.total_events?.toLocaleString() ?? "more"} events.</span>
              <button className="btn-ghost" type="button" onClick={onLoadMore} disabled={loadingMore}>
                {loadingMore ? "Loading…" : "Load next page"}
              </button>
            </div>
          )}
        </>
      )}
    </section>
  );
}

function SubagentNode({ node, depth }: { node: UsageSubagent; depth: number }) {
  return (
    <div className="usage-tree-node" style={{ marginLeft: depth * 16 }}>
      <div className="usage-tree-head">
        <span className="usage-tree-branch">{depth ? "↳" : "•"}</span>
        <b>{node.label}</b>
        <CostValue value={node.cost_usd} source={node.cost_source} />
      </div>
      <div className="usage-tree-meta">
        {fmtNum(totalTokens(node))} tokens · {node.event_count === null ? "unavailable" : node.event_count} event{node.event_count === 1 ? "" : "s"}
        {node.event_ids.length > 0 && <span> · <SourceEventLinks eventIds={node.event_ids} /></span>}
      </div>
      {node.children.map((child) => <SubagentNode key={child.subagent_id} node={child} depth={depth + 1} />)}
    </div>
  );
}

function PanelHeading({ id, title, subtitle }: { id?: string; title: string; subtitle: string }) {
  return (
    <div className="usage-panel-heading">
      <h2 id={id}>{title}</h2>
      <span className="muted">{subtitle}</span>
    </div>
  );
}

function CostValue({ value, source, testId }: { value: number | null; source: CostSource; testId?: string }) {
  return (
    <span className="usage-cost" data-testid={testId}>
      <strong>{value === null ? "unavailable" : fmtCost(value)}</strong>
      <CostSourceBadge source={source} />
    </span>
  );
}

function totalTokens(row: Pick<UsageTotals, "input_tokens" | "output_tokens" | "cache_read_tokens" | "cache_write_tokens" | "reasoning_tokens">): number | null {
  const values = [row.input_tokens, row.output_tokens, row.cache_read_tokens, row.cache_write_tokens, row.reasoning_tokens];
  const known = values.filter((value): value is number => value !== null);
  return known.length ? known.reduce((total, value) => total + value, 0) : null;
}

function SourceEventLinks({ eventIds }: { eventIds: string[] }) {
  if (!eventIds.length) return <span className="muted">unavailable</span>;
  const visible = eventIds.slice(0, 3);
  return (
    <span className="usage-event-links">
      {visible.map((eventId) => (
        <a key={eventId} href={`#${eventAnchorId(eventId)}`}>event ↗</a>
      ))}
      {eventIds.length > visible.length && <a href="#usage-events">+{eventIds.length - visible.length} more ↗</a>}
    </span>
  );
}

function eventAnchorId(eventId: string): string {
  return `event-${encodeURIComponent(eventId)}`;
}

function mergeEventPage(base: UsageResponse, page: UsageResponse): UsageResponse {
  const existing = new Set(base.events.map((event) => event.id));
  const events = [...base.events, ...page.events.filter((event) => !existing.has(event.id))];
  return { ...base, events, pagination: page.pagination };
}
