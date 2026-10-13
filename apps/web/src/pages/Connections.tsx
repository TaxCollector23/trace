import { CodeBox, Loading, relTime } from "../components";
import { loadConnections, type CapabilityStatus, type ConnectionRow } from "../connections";
import { useResource } from "../useResource";
import { ResourceGate, ToneIcon } from "../v4/ui";

export default function Connections() {
  const connections = useResource((signal) => loadConnections(signal), { pollMs: 30_000 });

  return (
    <div>
      <h1 className="page-title">Connections</h1>
      <p className="page-sub">
        Evidence from this machine about which coding agents are installed, wired to Trace, and
        actually sending events. Missing telemetry stays unavailable.
      </p>

      <ResourceGate
        resource={connections.resource}
        what="agent connections"
        onRetry={connections.reload}
        skeleton={<Loading variant="table" rows={5} />}
        empty={<ConnectionOnboarding />}
      >
        {(rows) => {
          const hasEvents = rows.some((row) => row.events_imported > 0 || row.last_event_at !== null);
          return (
            <>
              {!hasEvents && <ConnectionOnboarding />}
              <ConnectionTable rows={rows} />
            </>
          );
        }}
      </ResourceGate>
    </div>
  );
}

function ConnectionOnboarding() {
  return (
    <section className="connections-onboarding" data-testid="connections-empty">
      <div className="connections-onboarding-icon"><ToneIcon tone="info" size={22} /></div>
      <div>
        <h2>No agent events yet</h2>
        <p className="muted">
          Install the Trace integrations, run one coding-agent task, then refresh this page. The
          table will fill from real local detection and imported events.
        </p>
        <ol className="connections-steps">
          <li><b>Install</b><span>Run <code>trc integrations install all</code>.</span></li>
          <li><b>Run once</b><span>Start an agent task, for example <code>trc run &quot;claude&quot;</code>.</span></li>
          <li><b>Refresh</b><span>Use the browser refresh or the button below.</span></li>
        </ol>
        <button className="btn-ghost" type="button" onClick={() => window.location.reload()}>Refresh connections</button>
      </div>
    </section>
  );
}

function ConnectionTable({ rows }: { rows: ConnectionRow[] }) {
  if (!rows.length) {
    return <div className="empty">The daemon did not return any supported integrations.</div>;
  }
  return (
    <>
      <div className="v4-table-wrap connections-table-wrap">
        <table className="connections-table">
          <thead>
            <tr>
              <th>Agent</th>
              <th>Connection</th>
              <th>Local source path</th>
              <th>Last event</th>
              <th>Imported</th>
              <th>Parse errors</th>
              <th>Next action</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.id}>
                <td>
                  <b>{row.name}</b>
                  {row.version && <div className="muted mono">{row.version}</div>}
                </td>
                <td><ConnectionStatus row={row} /></td>
                <td className="mono connections-path">
                  {row.session_paths.length ? row.session_paths.join("\n") : <span className="muted">not reported by daemon</span>}
                </td>
                <td>{row.last_event_at ? relTime(row.last_event_at) : <span className="muted">unavailable</span>}</td>
                <td>{row.events_imported.toLocaleString()}</td>
                <td>{row.parse_errors.toLocaleString()}</td>
                <td className="connections-action">{row.action}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <section className="usage-panel connections-capabilities" aria-labelledby="capability-title">
        <div className="usage-panel-heading">
          <h2 id="capability-title">Capability matrix</h2>
          <span className="muted">Unavailable capability data is shown explicitly, never as zero.</span>
        </div>
        <div className="v4-table-wrap">
          <table className="connections-table">
            <thead><tr><th>Agent</th><th>Tokens</th><th>Cost</th><th>Subagents</th><th>Tool calls</th><th>Command guard</th></tr></thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.id}>
                  <td><b>{row.name}</b></td>
                  <td>{capability(row.capabilities.tokens)}</td>
                  <td>{capability(row.capabilities.cost)}</td>
                  <td>{capability(row.capabilities.subagents)}</td>
                  <td>{capability(row.capabilities.tool_calls)}</td>
                  <td>{capability(row.capabilities.command_guard)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      <div className="connections-help">
        <h2>Need to connect an agent?</h2>
        <CodeBox label="Install all supported hooks" command="trc integrations install all" />
        <p className="muted">If an agent is installed but not connected, its row includes the exact next action reported by the daemon.</p>
      </div>
    </>
  );
}

function ConnectionStatus({ row }: { row: ConnectionRow }) {
  const state = row.connected ? "connected" : row.status.split("_").join(" ");
  return (
    <div>
      <span className={`connection-state ${row.connected ? "connected" : ""}`}>
        <span className="connection-dot" aria-hidden="true" /> {state}
      </span>
      <div className="muted connections-note">{row.action || row.note || row.install_hint || "No next action was reported."}</div>
    </div>
  );
}

function capability(value: CapabilityStatus) {
  if (value === "available") return <span className="connection-cap connection-cap-yes">available</span>;
  if (value === "partial") return <span className="connection-cap">partial</span>;
  if (value === "not_available") return <span className="connection-cap connection-cap-no">not available for this agent</span>;
  return <span className="muted">not verified for this agent</span>;
}
