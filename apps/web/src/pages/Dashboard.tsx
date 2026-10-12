import { Link } from "react-router-dom";
import { api } from "../api";
import {
  CodeBox,
  StatusBadge,
  fmtCost,
  Loading,
  relTime,
  runTitle,
  useAsync,
  whatChanged,
} from "../components";

function RunMetrics({ run }: { run: Parameters<typeof whatChanged>[0] }) {
  return (
    <div className="current-run-metrics">
      <div>
        <strong>{run.files_changed}</strong>
        <span>files changed</span>
      </div>
      <div>
        <strong>{run.command_count}</strong>
        <span>commands</span>
      </div>
      <div>
        <strong>{run.secret_warnings}</strong>
        <span>secrets flagged</span>
      </div>
      <div>
        <strong>{fmtCost(run.estimated_cost)}</strong>
        <span>estimated cost</span>
      </div>
    </div>
  );
}

export default function Dashboard() {
  const { data, error, loading } = useAsync(() => api.dashboard(), [], 3000);

  if (loading || !data) {
    return (
      <div>
        <div className="eyebrow">Local activity</div>
        <h1 className="page-title">What is happening now?</h1>
        <p className="page-sub">Trace is checking your local coding sessions.</p>
        <Loading error={error} variant="cards" rows={3} />
      </div>
    );
  }

  const { runs, projects } = data;
  const active = runs.find((run) => run.status === "running") ?? null;
  const latest = runs[0] ?? null;

  return (
    <div className="dashboard-page">
      <header className="dashboard-header">
        <div>
          <div className="eyebrow">Local activity</div>
          <h1 className="page-title">What is happening now?</h1>
          <p className="page-sub">
            {active
              ? `${runTitle(active)} is working in ${active.project_name}.`
              : "See what your agents changed, ran, and stopped."}
          </p>
        </div>
        <div className="live-refresh" title="The dashboard checks the local daemon every three seconds">
          <span className="live-dot" /> Live · 3s
        </div>
      </header>

      <div className="dashboard-context">
        <span>Watching {projects.length} project{projects.length === 1 ? "" : "s"}</span>
        <span>Everything stays on this machine</span>
        {error && <span className="refresh-warning">Last refresh failed; showing saved data</span>}
      </div>

      {active ? (
        <section className="current-run-panel" aria-labelledby="current-run-title">
          <div className="current-run-topline">
            <span className="section-kicker">Current run</span>
            <StatusBadge status={active.status} />
          </div>
          <div className="current-run-heading">
            <div>
              <h2 id="current-run-title">{runTitle(active)}</h2>
              <p>{active.project_name}</p>
            </div>
            <Link className="button button-primary" to={`/timeline/${active.id}`}>
              Open timeline
            </Link>
          </div>
          <div className="current-run-command mono">{active.command || "Agent session"}</div>
          <RunMetrics run={active} />
          <div className="current-run-summary">
            {whatChanged(active) || "The run has started. New activity will appear here."}
          </div>
        </section>
      ) : latest ? (
        <section className="idle-panel" aria-labelledby="idle-title">
          <div className="idle-mark">✓</div>
          <div>
            <div className="section-kicker">Nothing running right now</div>
            <h2 id="idle-title">Trace is ready</h2>
            <p>
              The last session was {runTitle(latest)} in {latest.project_name} {relTime(latest.started_at)}.
            </p>
          </div>
          <Link className="button button-secondary" to={`/timeline/${latest.id}`}>
            View last session
          </Link>
        </section>
      ) : (
        <section className="empty dashboard-empty">
          <div className="section-kicker">First time here?</div>
          <h2>Connect an agent to start watching.</h2>
          <p>Trace will show the current run here as soon as it records activity.</p>
          <CodeBox label="Connect your agents" command="trc integrations install all" />
          <CodeBox label="Start a session" command='trc run "claude"' />
        </section>
      )}

      <section className="recent-sessions" aria-labelledby="recent-title">
        <div className="section-heading">
          <div>
            <div className="section-kicker">History</div>
            <h2 id="recent-title">Recent sessions</h2>
          </div>
          <span>{runs.length} recorded</span>
        </div>
        {runs.length === 0 ? (
          <div className="quiet-empty">No sessions recorded yet.</div>
        ) : (
          <div className="session-list">
            {runs.slice(0, 8).map((run) => (
              <Link key={run.id} to={`/timeline/${run.id}`} className="session-row">
                <span className={`session-status-dot ${run.status}`} aria-hidden="true" />
                <span className="session-row-main">
                  <span className="session-row-title">{runTitle(run)}</span>
                  <span className="session-row-project">{run.project_name}</span>
                  <span className="session-row-change">{whatChanged(run) || "No file or command changes recorded"}</span>
                </span>
                <span className="session-row-meta">
                  <StatusBadge status={run.status} />
                  <span>{relTime(run.started_at)}</span>
                  <span>{fmtCost(run.estimated_cost)}</span>
                </span>
              </Link>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
