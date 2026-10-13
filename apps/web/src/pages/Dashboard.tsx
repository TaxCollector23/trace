import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, type DashboardData, type RunSummary } from "../api";
import { CodeBox, fmtCost, relTime, runTitle, whatChanged } from "../components";

function useLiveDashboard() {
  const [data, setData] = useState<DashboardData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<number | null>(null);
  const [retry, setRetry] = useState(0);

  useEffect(() => {
    let mounted = true;
    let inFlight = false;
    const refresh = async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        const next = await api.dashboard();
        if (mounted) {
          setData(next);
          setError(null);
          setLastUpdated(Date.now());
        }
      } catch (cause) {
        if (mounted) setError(String(cause));
      } finally {
        inFlight = false;
      }
    };

    void refresh();
    const interval = window.setInterval(() => void refresh(), 3000);
    return () => {
      mounted = false;
      window.clearInterval(interval);
    };
  }, [retry]);

  return { data, error, lastUpdated, refresh: () => setRetry((value) => value + 1) };
}

function RunFacts({ run }: { run: RunSummary }) {
  return (
    <dl className="dashboard-facts">
      <div><dt>Files changed</dt><dd>{run.files_changed}</dd></div>
      <div><dt>Commands run</dt><dd>{run.command_count}</dd></div>
      <div><dt>Secrets flagged</dt><dd>{run.secret_warnings}</dd></div>
      <div><dt>Estimated cost</dt><dd>{fmtCost(run.estimated_cost)}</dd></div>
    </dl>
  );
}

export default function Dashboard() {
  const { data, error, lastUpdated, refresh } = useLiveDashboard();
  const runs = data?.runs ?? [];
  const active = runs.find((run) => run.status === "running") ?? null;
  const latest = runs.find((run) => run.status !== "running") ?? null;
  const integrations = data?.integrations ?? [];
  const connected = integrations.filter((item) => item.connected);
  const codexConfigured = connected.some((item) => item.id === "codex");

  return (
    <div className="dashboard-page">
      <header className="dashboard-header">
        <div>
          <div className="eyebrow">Current activity</div>
          <h1 className="page-title">
            {active
              ? `${runTitle(active)} ${error ? "was working" : "is working"} in ${active.project_name}`
              : !data ? error ? "Trace is offline" : "Checking Trace…"
              : error ? "Last known: no agent was running" : "No agent is running"}
          </h1>
          <p className="page-sub">
            {active && !error ? "Trace is recording this session on your machine."
              : error ? "Live activity is unavailable until the local daemon responds."
              : "Trace is ready. New activity will appear here."}
          </p>
        </div>
        <div className={`live-refresh ${error ? "live-refresh-error" : !lastUpdated ? "live-refresh-pending" : ""}`} aria-live="polite">
          <span className="live-dot" aria-hidden="true" />
          {error ? "Connection lost" : lastUpdated ? "Live · 3s" : "Connecting…"}
        </div>
      </header>

      {error && (
        <div className="dashboard-warning" role="status">
          <span>
            {data
              ? `The last refresh failed. Showing data from ${new Date(lastUpdated!).toLocaleTimeString()}.`
              : "The local daemon is not responding. Start it with `trc dashboard`, then try again."}
          </span>
          <button className="dashboard-text-button" type="button" onClick={refresh}>Retry</button>
        </div>
      )}

      {!data && !error && <div className="dashboard-loading" role="status">Checking the local daemon…</div>}

      {data && (
        <>
          {active ? (
            <section className="current-run-panel" aria-labelledby="current-run-title">
              <div className="current-run-topline">
                <span className="section-kicker">{error ? "Last known activity" : "Working now"}</span>
                <span className="dashboard-run-time">Started {relTime(active.started_at)}</span>
              </div>
              <div className="current-run-heading">
                <div>
                  <h2 id="current-run-title">{runTitle(active)} · {active.project_name}</h2>
                  <p>{error ? "Last recorded status: running" : "Running"}</p>
                </div>
                <Link className="button button-primary" to={`/timeline/${active.id}`}>Open timeline →</Link>
              </div>
              <div className="current-run-command mono">{active.command || "Agent session"}</div>
              <RunFacts run={active} />
              <p className="current-run-summary">{whatChanged(active) || "The session has started. New activity will appear here."}</p>
            </section>
          ) : (
            <section className="idle-panel" aria-labelledby="idle-title">
              <span className="idle-mark" aria-hidden="true">✓</span>
              <div className="idle-copy">
                <div className="section-kicker">{error ? "Last known state" : "Ready to record"}</div>
                <h2 id="idle-title">{error ? "Waiting for Trace" : "Trace is ready"}</h2>
                <p>
                  {latest
                    ? `Last recorded: ${runTitle(latest)} in ${latest.project_name}, ${relTime(latest.started_at)}.`
                    : "No session has been recorded yet."}
                </p>
              </div>
              {latest && <Link className="button button-secondary" to={`/timeline/${latest.id}`}>View last session →</Link>}
            </section>
          )}

          <section className="dashboard-connection" aria-labelledby="connection-title">
            <div>
              <div className="section-kicker">Connection</div>
              <h2 id="connection-title">
                {connected.length ? `${connected.length} agent integration${connected.length === 1 ? "" : "s"} configured` : "No agent integrations configured"}
              </h2>
              <p>
                {connected.length
                  ? `Trace configuration is present for ${connected.map((item) => item.name).join(", ")}. Activity appears after a supported agent starts a session and sends events.`
                  : "Connect an agent to record its sessions automatically, or start a monitored command yourself."}
              </p>
              {codexConfigured && (
                <p className="dashboard-connection-note">Codex hook configuration is present. In Codex, use <code>/hooks</code> to check and approve Trace; configuration alone cannot confirm approval or GUI activity.</p>
              )}
            </div>
            <Link className="dashboard-inline-link" to="/connections">View connections →</Link>
          </section>

          {!active && runs.length === 0 && (
            <section className="dashboard-next" aria-labelledby="next-title">
              <h2 id="next-title">Start a monitored session</h2>
              <p>In your project directory, initialize Trace, then run your agent through it. Installed hooks can also record supported agent sessions.</p>
              <div className="dashboard-next-commands">
                <CodeBox label="In your project" command="trc init" />
                {!connected.length && <CodeBox label="Connect supported agents" command="trc integrations install all" />}
                <CodeBox label="Run an agent" command="trc run claude" />
              </div>
            </section>
          )}

          <section className="recent-sessions" aria-labelledby="recent-title">
            <div className="section-heading">
              <div>
                <div className="section-kicker">History</div>
                <h2 id="recent-title">Recent activity</h2>
              </div>
              <Link className="dashboard-inline-link" to="/timeline">All activity →</Link>
            </div>
            {runs.length === 0 ? (
              <p className="quiet-empty">No sessions recorded yet.</p>
            ) : (
              <div className="session-list">
                <div className="session-list-head" aria-hidden="true">
                  <span>Agent</span><span>Project</span><span>What changed</span><span>Status</span><span>Time</span><span>Cost</span>
                </div>
                {runs.slice(0, 8).map((run) => (
                  <Link key={run.id} to={`/timeline/${run.id}`} className="session-row" aria-label={`Open ${runTitle(run)} session in ${run.project_name}`}>
                    <span className="session-row-agent"><span className={`session-status-dot ${run.status}`} aria-hidden="true" />{runTitle(run)}</span>
                    <span className="session-row-project">{run.project_name}</span>
                    <span className="session-row-change">{whatChanged(run) || "No changes recorded"}</span>
                    <span className="session-row-status">{run.status.replace("_", " ")}</span>
                    <span className="session-row-time">{relTime(run.started_at)}</span>
                    <span className="session-row-cost">{fmtCost(run.estimated_cost)}</span>
                  </Link>
                ))}
              </div>
            )}
          </section>
        </>
      )}
    </div>
  );
}
