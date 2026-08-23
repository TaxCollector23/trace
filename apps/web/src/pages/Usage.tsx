import { useParams } from "react-router-dom";
import { api } from "../api";
import {
  Loading,
  RunPicker,
  agentLabel,
  fmtCost,
  fmtNum,
  stagger,
  useAsync,
} from "../components";

function fmtRate(n: number | null): string {
  if (n === null || n === undefined) return "—";
  if (n < 0.1) return n.toFixed(2);
  if (n < 10) return n.toFixed(1);
  return Math.round(n).toLocaleString();
}

function Kpi({ value, label }: { value: string; label: string }) {
  return (
    <div className="kpi">
      <div className="k-val">{value}</div>
      <div className="k-label">{label}</div>
    </div>
  );
}

/** Single source of truth for token/usage reporting. The main view answers
 * "how many tokens, which agent, what cost, and how the run rate trends";
 * the per-run provider/model breakdown lives behind the run picker drill-down. */
export default function Usage() {
  const { runId } = useParams();
  const analyticsQ = useAsync(() => api.analytics());
  const runsQ = useAsync(() => api.runs());
  const runs = runsQ.data ?? [];
  const current = runId ?? runs[0]?.id;

  const costQ = useAsync(
    () => (current ? api.cost(current) : Promise.resolve(null)),
    [current]
  );
  const cost = costQ.data;
  const data = analyticsQ.data;

  const totalTokens = data
    ? data.by_agent.reduce((s, a) => s + a.input_tokens + a.output_tokens, 0)
    : 0;
  const totalCost = data ? data.by_agent.reduce((s, a) => s + a.estimated_cost, 0) : 0;

  return (
    <div>
      <h1 className="page-title">Token Usage</h1>
      <p className="page-sub">
        Tokens, cost and run frequency across your agents — computed locally from
        your own run history. Nothing here is sent anywhere.
      </p>

      {analyticsQ.loading ? (
        <Loading error={analyticsQ.error} variant="kpis" />
      ) : !data || data.total_runs === 0 ? (
        <div className="empty">
          No runs recorded yet — usage will show up once you've run a few sessions.
        </div>
      ) : (
        <>
          {/* All-time totals: how many tokens and what cost, at a glance. */}
          <div className="kpis">
            <Kpi value={fmtNum(data.total_runs)} label="Total runs" />
            <Kpi value={fmtNum(totalTokens)} label="Total tokens" />
            <Kpi value={fmtCost(totalCost)} label="Total cost" />
            <Kpi value={data.by_agent.length.toString()} label="Agents used" />
          </div>

          {/* Trend over time: how often runs happen. */}
          <div className="section-title">Run frequency</div>
          {data.avg_per_hour === null ? (
            <div className="note">
              Not enough history yet for a meaningful average — this fills in after
              your first run is more than an hour old.
            </div>
          ) : (
            <div className="kpis">
              <Kpi value={fmtRate(data.avg_per_hour)} label="Avg runs / hour" />
              <Kpi value={fmtRate(data.avg_per_day)} label="Avg runs / day" />
              <Kpi value={fmtRate(data.avg_per_week)} label="Avg runs / week" />
              <Kpi value={fmtRate(data.avg_per_month)} label="Avg runs / month" />
            </div>
          )}

          {/* Which agent: per-agent token + cost breakdown. */}
          <div className="section-title" style={{ marginTop: 30 }}>
            By agent
          </div>
          {data.by_agent.length === 0 ? (
            <div className="empty">
              No token usage recorded yet — this fills in once an agent reports
              usage through a Trace adapter.
            </div>
          ) : (
            <div
              className="kpis"
              style={{ gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))" }}
            >
              {data.by_agent.map((a, i) => {
                const total = a.input_tokens + a.output_tokens;
                return (
                  <div key={a.agent_name} className="card enter" style={stagger(i)}>
                    <div className="run-head">
                      <b>{agentLabel(a.agent_name)}</b>
                      <span className="muted">{a.run_count} runs</span>
                    </div>
                    <p style={{ margin: "10px 0 0" }}>
                      <b>{fmtNum(total)}</b> tokens
                      {a.estimated_cost > 0 && (
                        <> · about <b>{fmtCost(a.estimated_cost)}</b></>
                      )}
                    </p>
                    <div className="run-meta">
                      <span>
                        in: <b>{fmtNum(a.input_tokens)}</b>
                      </span>
                      <span>
                        out: <b>{fmtNum(a.output_tokens)}</b>
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </>
      )}

      {/* Drill-down: per-run provider/model detail. Cost is labelled honestly
          — "unavailable" when it can't be computed. */}
      <div className="section-title" style={{ marginTop: 30 }}>
        Per-run breakdown
      </div>
      {runsQ.loading ? (
        <Loading error={runsQ.error} variant="cards" rows={1} />
      ) : runs.length === 0 ? (
        <div className="empty">No runs recorded yet.</div>
      ) : (
        <>
          <RunPicker runs={runs} current={current} base="/usage" />
          {costQ.loading ? (
            <Loading error={costQ.error} variant="table" rows={3} />
          ) : !cost || cost.usage.length === 0 ? (
            <div className="empty">
              Cost data appears when the agent reports usage or traffic flows
              through a Trace adapter.
            </div>
          ) : (
            <>
              <div className="note">
                Total estimated cost for this run: <b>{fmtCost(cost.total_estimated)}</b>
                {cost.has_unavailable &&
                  " (some entries have unavailable cost and are excluded)"}
              </div>
              <table>
                <thead>
                  <tr>
                    <th>Provider</th>
                    <th>Model</th>
                    <th>Input</th>
                    <th>Output</th>
                    <th>Cached</th>
                    <th>Latency</th>
                    <th>Est. cost</th>
                  </tr>
                </thead>
                <tbody>
                  {cost.usage.map((u, i) => (
                    <tr key={u.id} className="enter" style={stagger(i, 20, 160)}>
                      <td>{u.provider}</td>
                      <td className="mono">{u.model}</td>
                      <td>{fmtNum(u.input_tokens)}</td>
                      <td>{fmtNum(u.output_tokens)}</td>
                      <td>{fmtNum(u.cached_tokens)}</td>
                      <td>{u.latency_ms ? `${u.latency_ms} ms` : "—"}</td>
                      <td>{fmtCost(u.estimated_cost)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </>
          )}
        </>
      )}
    </div>
  );
}
