import { Link } from "react-router-dom";
import { api } from "../api";
import {
  CodeBox,
  StatusBadge,
  fmtCost,
  Loading,
  relTime,
  runTitle,
  stagger,
  useAsync,
  whatChanged,
} from "../components";

export default function Dashboard() {
  const { data, error, loading } = useAsync(() => api.dashboard(), [], 3000);

  if (loading || !data) {
    return (
      <div>
        <h1 className="page-title">What is Trace doing?</h1>
        <p className="page-sub">This page watches your local coding sessions.</p>
        <Loading error={error} variant="kpis" />
        {!error && <Loading variant="cards" rows={4} />}
      </div>
    );
  }

  const { runs, projects } = data;
  const totalCost = runs.reduce((s, r) => s + (r.estimated_cost ?? 0), 0);
  const secretWarnings = runs.reduce((s, r) => s + r.secret_warnings, 0);

  return (
    <div>
      <div className="dashboard-heading">
        <div>
          <h1 className="page-title">What is Trace doing?</h1>
          <p className="page-sub">This page watches your local coding sessions.</p>
        </div>
        <span className="live-refresh"><span className="live-dot" /> Updates every 3 seconds</span>
      </div>
      <p className="local-inline">All data is local to this machine.</p>

      <div className="kpis">
        <div className="kpi enter" style={stagger(0)}>
          <div className="k-val">{runs.length}</div>
          <div className="k-label">Sessions</div>
        </div>
        <div className="kpi enter" style={stagger(1)}>
          <div className="k-val">{projects.length}</div>
          <div className="k-label">Projects</div>
        </div>
        <div className="kpi enter" style={stagger(2)}>
          <div className="k-val">{secretWarnings}</div>
          <div className="k-label">Secrets flagged</div>
        </div>
        <div className="kpi enter" style={stagger(3)}>
          <div className="k-val">{totalCost > 0 ? fmtCost(totalCost) : "—"}</div>
          <div className="k-label">Estimated cost</div>
        </div>
      </div>

      {runs.length === 0 ? (
        <div className="empty">
          <p style={{ margin: "0 0 14px" }}>
            Nothing is being watched yet. Connect an agent, then start a session:
          </p>
          <CodeBox label="Connect your agents" command="trc integrations install all" />
          <CodeBox label="Start a session" command='trc run "claude"' />
        </div>
      ) : (
        runs.map((r, i) => {
          const changed = whatChanged(r);
          return (
            <Link
              key={r.id}
              to={`/timeline/${r.id}`}
              className="card card-link enter"
              style={stagger(i)}
            >
              <div className="run-head">
                <div className="run-title">
                  <span className="run-who">{r.status === "running" ? "Working now" : runTitle(r)}</span>
                  <span className="run-project"> · {r.project_name}</span>
                </div>
                <StatusBadge status={r.status} />
              </div>
              <div className="run-cmd-line mono">{r.command || "Agent session"}</div>
              {changed && <div className="run-changed">{changed}</div>}
              <div className="run-meta">
                <span>{relTime(r.started_at)}</span>
                <span>cost {fmtCost(r.estimated_cost)}</span>
                {r.checks_status && <span>checks {r.checks_status}</span>}
              </div>
            </Link>
          );
        })
      )}
    </div>
  );
}
