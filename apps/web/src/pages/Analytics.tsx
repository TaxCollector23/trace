import { api } from "../api";
import { Loading, fmtCost, fmtNum, stagger, useAsync } from "../components";

const AGENT_LABELS: Record<string, string> = {
  claude: "Claude Code",
  codex: "Codex CLI",
  cursor: "Cursor",
  opencode: "OpenCode",
  copilot: "GitHub Copilot",
  unknown: "Unattributed",
};

function agentLabel(name: string): string {
  return AGENT_LABELS[name.toLowerCase()] ?? name;
}

function fmtRate(n: number | null): string {
  if (n === null || n === undefined) return "—";
  if (n < 0.1) return n.toFixed(2);
  if (n < 10) return n.toFixed(1);
  return Math.round(n).toLocaleString();
}

export default function Analytics() {
  const q = useAsync(() => api.analytics());
  const data = q.data;

  return (
    <div>
      <h1 className="page-title">Trace Analytics</h1>
      <p className="page-sub">
        How often you run agents, and how many tokens each one has burned —
        computed locally from your own run history. Nothing here is sent
        anywhere.
      </p>

      {q.loading ? (
        <Loading error={q.error} variant="kpis" />
      ) : !data || data.total_runs === 0 ? (
        <div className="empty">No runs recorded yet — analytics will show up once you've run a few sessions.</div>
      ) : (
        <>
          <div className="section-title">Prompt frequency</div>
          {data.avg_per_hour === null ? (
            <div className="note">
              Not enough history yet for a meaningful average — this fills in
              after your first run is more than an hour old.
            </div>
          ) : (
            <div className="kpis">
              <Kpi value={fmtRate(data.avg_per_hour)} label="Avg runs / hour" />
              <Kpi value={fmtRate(data.avg_per_day)} label="Avg runs / day" />
              <Kpi value={fmtRate(data.avg_per_week)} label="Avg runs / week" />
              <Kpi value={fmtRate(data.avg_per_month)} label="Avg runs / month" />
            </div>
          )}

          <div className="section-title" style={{ marginTop: 30 }}>
            Cool facts
          </div>
          {data.by_agent.length === 0 ? (
            <div className="empty">
              No token usage recorded yet — this fills in once an agent
              reports usage through a Trace adapter.
            </div>
          ) : (
            <div className="kpis" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))" }}>
              {data.by_agent.map((a, i) => {
                const total = a.input_tokens + a.output_tokens;
                return (
                  <div key={a.agent_name} className="card enter" style={stagger(i)}>
                    <div className="run-head">
                      <b>{agentLabel(a.agent_name)}</b>
                      <span className="muted">{a.run_count} runs</span>
                    </div>
                    <p style={{ margin: "10px 0 0" }}>
                      You've used <b>{fmtNum(total)}</b> tokens with {agentLabel(a.agent_name)}
                      {a.estimated_cost > 0 && (
                        <> — about <b>{fmtCost(a.estimated_cost)}</b></>
                      )}
                      .
                    </p>
                    <div className="run-meta">
                      <span>in: <b>{fmtNum(a.input_tokens)}</b></span>
                      <span>out: <b>{fmtNum(a.output_tokens)}</b></span>
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          <div className="section-title" style={{ marginTop: 30 }}>
            All-time
          </div>
          <div className="kpis">
            <Kpi value={fmtNum(data.total_runs)} label="Total runs" />
            <Kpi
              value={fmtNum(data.by_agent.reduce((s, a) => s + a.input_tokens + a.output_tokens, 0))}
              label="Total tokens"
            />
            <Kpi
              value={fmtCost(data.by_agent.reduce((s, a) => s + a.estimated_cost, 0))}
              label="Total cost"
            />
            <Kpi value={data.by_agent.length.toString()} label="Agents used" />
          </div>
        </>
      )}
    </div>
  );
}

function Kpi({ value, label }: { value: string; label: string }) {
  return (
    <div className="kpi">
      <div className="k-val">{value}</div>
      <div className="k-label">{label}</div>
    </div>
  );
}
