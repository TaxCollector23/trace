import { v4 } from "../data";
import { useResource } from "../useResource";
import { ResourceGate, ToneIcon } from "../v4/ui";

// System & Integrations: an honest view of what the daemon reports about
// itself, and which read endpoints the v4 dashboard depends on are actually
// instrumented. Nothing is asserted that the backend did not confirm.

export default function SystemPage() {
  const health = useResource((s) => v4.health(s), { pollMs: 10000 });
  const coverage = useResource((s) => v4.coverage(s));

  return (
    <div>
      <h1 className="page-title">System &amp; Integrations</h1>
      <p className="page-sub">
        What the local daemon reports about itself, and which intelligence endpoints are live.
      </p>

      <h2 className="section-title">Daemon health</h2>
      <ResourceGate resource={health.resource} what="daemon health" onRetry={health.reload}>
        {(h) => (
          <div className="v4-table-wrap">
            <table>
              <tbody>
                <tr>
                  <th>Status</th>
                  <td>
                    <span className="v4-cov tone-success">
                      <ToneIcon tone="success" size={12} /> {h.status}
                    </span>
                  </td>
                </tr>
                <tr>
                  <th>Service</th>
                  <td className="mono">{h.service ?? "—"}</td>
                </tr>
                <tr>
                  <th>Version</th>
                  <td className="mono">{h.version ?? "—"}</td>
                </tr>
              </tbody>
            </table>
          </div>
        )}
      </ResourceGate>

      <h2 className="section-title">Integration coverage</h2>
      <ResourceGate resource={coverage.resource} what="integration coverage" onRetry={coverage.reload}>
        {(rows) => (
          <div className="v4-table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Agent</th>
                  <th>Command enforcement</th>
                  <th>File review</th>
                  <th>Status</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((c) => (
                  <tr key={c.agent}>
                    <td>
                      <b>{c.agent}</b>
                    </td>
                    <td>{yn(c.command_enforcement)}</td>
                    <td>{yn(c.file_review)}</td>
                    <td className="muted">{c.note ?? c.status}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </ResourceGate>
    </div>
  );
}

function yn(v: boolean | null) {
  if (v == null) return <span className="muted">unknown</span>;
  return v ? (
    <span className="v4-cov tone-success">
      <ToneIcon tone="success" size={12} /> yes
    </span>
  ) : (
    <span className="v4-cov tone-muted">
      <ToneIcon tone="muted" size={12} /> no
    </span>
  );
}
