import { Link } from "react-router-dom";
import { api } from "../api";
import { StatusBadge, fmtCost, fmtTime, Loading, stagger, useAsync } from "../components";

function IntelStrip() {
  const benchQ = useAsync(() => api.benchmarks());
  const redteamQ = useAsync(() => api.redteamBenchmarks());

  if (benchQ.loading || redteamQ.loading) {
    return null; // this strip is a bonus, not worth a loading skeleton delaying the page
  }

  const bench = benchQ.data;
  const rt = redteamQ.data;
  const rtCaught = rt ? rt.engines.reduce((n, e) => n + e.caught, 0) : 0;
  const rtThreats = rt ? rt.engines.reduce((n, e) => n + e.threats, 0) : 0;

  return (
    <div className="kpis" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", marginBottom: 26 }}>
      <Link to="/benchmarks" className="card card-link">
        <div className="run-head">
          <b>Policy engine benchmark</b>
          {bench && <span className={`pill ${bench.passed === bench.total ? "allow" : "block"}`}>{bench.passed}/{bench.total}</span>}
        </div>
        <p style={{ margin: "8px 0 0" }}>
          {bench ? `${Math.round(bench.precision * 100)}% precision, ${Math.round(bench.recall * 100)}% recall.` : "—"}
        </p>
      </Link>
      <Link to="/benchmarks" className="card card-link">
        <div className="run-head">
          <b>Red-team detection</b>
          {rt && <span className={`pill ${rt.passed ? "allow" : "block"}`}>{rtCaught}/{rtThreats}</span>}
        </div>
        <p style={{ margin: "8px 0 0" }}>
          {rt ? `${rtThreats} adversarial threats caught, ${rt.engines.reduce((n, e) => n + e.false_positives, 0)} false positives.` : "—"}
        </p>
      </Link>
      <Link to="/ratify" className="card card-link">
        <div className="run-head">
          <b>Ratify</b>
          <span className="muted">GitHub</span>
        </div>
        <p style={{ margin: "8px 0 0" }}>
          Run the deterministic policy engine over a pull request on your
          connected repo — no API key needed.
        </p>
      </Link>
    </div>
  );
}

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

      <IntelStrip />

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
