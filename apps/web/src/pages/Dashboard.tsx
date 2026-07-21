import { Link } from "react-router-dom";
import { api } from "../api";
import { StatusBadge, fmtCost, fmtTime, Loading, stagger, useAsync } from "../components";

export default function Dashboard() {
  const { data, error, loading } = useAsync(() => api.dashboard());

  if (loading || error || !data) {
    return (
      <div>
        <h1 className="page-title">Dashboard</h1>
        <p className="page-sub">Recent monitored runs across your projects.</p>
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
      <h1 className="page-title">Dashboard</h1>
      <p className="page-sub">
        Recent monitored runs across {projects.length} project
        {projects.length === 1 ? "" : "s"}. All data is local to this machine.
      </p>

      <div className="kpis">
        <div className="kpi enter" style={stagger(0)}>
          <div className="k-val">{runs.length}</div>
          <div className="k-label">Recent runs</div>
        </div>
        <div className="kpi enter" style={stagger(1)}>
          <div className="k-val">{projects.length}</div>
          <div className="k-label">Projects</div>
        </div>
        <div className="kpi enter" style={stagger(2)}>
          <div className="k-val">{secretWarnings}</div>
          <div className="k-label">Secret warnings</div>
        </div>
        <div className="kpi enter" style={stagger(3)}>
          <div className="k-val">{totalCost > 0 ? fmtCost(totalCost) : "—"}</div>
          <div className="k-label">Estimated cost</div>
        </div>
      </div>

      {runs.length === 0 ? (
        <div className="empty">
          Start your first monitored AI coding session with{" "}
          <span className="mono">trace run claude</span>.
        </div>
      ) : (
        runs.map((r, i) => (
          <Link
            key={r.id}
            to={`/timeline/${r.id}`}
            className="card card-link enter"
            style={stagger(i)}
          >
            <div className="run-head">
              <div className="run-cmd">{r.command}</div>
              <StatusBadge status={r.status} />
            </div>
            <div className="run-meta">
              <span>
                <b>{r.project_name}</b>
              </span>
              {r.agent_name && (
                <span>
                  agent <b>{r.agent_name}</b>
                </span>
              )}
              <span>started {fmtTime(r.started_at)}</span>
              <span>ended {fmtTime(r.ended_at)}</span>
              <span>
                <b>{r.files_changed}</b> files
              </span>
              <span>
                <b>{r.command_count}</b> commands
              </span>
              <span>
                <b>{r.secret_warnings}</b> secrets
              </span>
              <span>cost {fmtCost(r.estimated_cost)}</span>
              {r.checks_status && <span>checks {r.checks_status}</span>}
            </div>
          </Link>
        ))
      )}
    </div>
  );
}
