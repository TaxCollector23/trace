import { Link, useNavigate } from "react-router-dom";
import { v4 } from "../data";
import { runApi } from "../rundata";
import { useResource } from "../useResource";
import {
  deriveState,
  describe,
  type Action,
  type StateInputs,
} from "../state";
import { StatusBadge, relTime, runTitle, whatChanged, fmtCost } from "../components";
import { ResourceGate, ToneIcon } from "../v4/ui";
import { StatusStrip } from "../v4/StatusStrip";
import { DisconnectedScreen, DbUnavailableScreen, OnboardingScreen } from "../v4/screens";

// ---------------------------------------------------------------------------
// The Control Room is the adaptive index (§2). It answers, in seconds: is the
// daemon up, is anything running right now, what changed, and what needs me. It
// never buries the active run — if one is live, it is the first thing you see.
// ---------------------------------------------------------------------------

const ACTIVE = ["running", "starting"];
function isTerminal(status: string): boolean {
  return !ACTIVE.includes(status);
}

export default function ControlRoom() {
  const navigate = useNavigate();
  const healthRes = useResource((s) => v4.health(s), { pollMs: 10000 });
  const runsRes = useResource((s) => runApi.runs(s), { pollMs: 8000 });
  const coverageRes = useResource((s) => v4.coverage(s));

  const runs = runsRes.resource.state === "ok" ? runsRes.resource.data : [];
  const activeRun = runs.find((r) => !isTerminal(r.status)) ?? null;

  const inputs: StateInputs = {
    health: healthRes.resource,
    runs: runsRes.resource,
    run: activeRun,
    events: { state: "loading" },
    signals: { state: "loading" },
    incidents: { state: "loading" },
  };
  const state = deriveState(inputs);
  const descriptor = describe(state);

  const onAction = (a: Action) => {
    if (a.to) return navigate(a.to);
    switch (a.command) {
      case "focus-latest":
        if (activeRun) return navigate(`/run/${activeRun.id}`);
        if (runs[0]) return navigate(`/run/${runs[0].id}`);
        return;
      case "reload":
        healthRes.reload();
        runsRes.reload();
        return;
      case "palette":
        window.dispatchEvent(
          new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true })
        );
        return;
      case "help-onboarding":
      case "help-daemon":
        return; // guidance already on screen for these states
      default:
        return;
    }
  };

  // Whole-screen honest states first.
  if (state === "DISCONNECTED") {
    return <DisconnectedScreen onRetry={onAction.bind(null, { label: "", command: "reload" })} />;
  }
  if (runsRes.resource.state === "unavailable" && runsRes.resource.kind === "server") {
    return (
      <DbUnavailableScreen
        reason={runsRes.resource.reason}
        onRetry={() => runsRes.reload()}
      />
    );
  }
  if (state === "NO_DATA") {
    return (
      <div>
        <StatusStrip descriptor={descriptor} onAction={onAction} />
        <OnboardingScreen />
        <CoveragePanel res={coverageRes.resource} onRetry={coverageRes.reload} />
      </div>
    );
  }

  return (
    <div>
      <h1 className="page-title">Control Room</h1>
      <p className="page-sub">
        Live state of everything Trace is watching on this machine. Press ⌘K to jump anywhere.
      </p>

      <StatusStrip descriptor={descriptor} onAction={onAction} />

      {activeRun && (
        <Link to={`/run/${activeRun.id}`} className="v4-active-run">
          <div className="v4-active-run-head">
            <span className="v4-live on">
              <span className="v4-live-dot" /> Live now
            </span>
            <span className="v4-active-run-title">
              {runTitle(activeRun)} <span className="muted">— {activeRun.project_name}</span>
            </span>
          </div>
          <div className="run-cmd-line mono">{activeRun.command}</div>
          <div className="muted" style={{ marginTop: 6 }}>
            started {relTime(activeRun.started_at)} · open the Run Page to follow live →
          </div>
        </Link>
      )}

      <h2 className="section-title">Recent activity</h2>
      <ResourceGate
        resource={runsRes.resource}
        what="runs"
        onRetry={runsRes.reload}
        empty={<OnboardingScreen />}
      >
        {(all) => (
          <div className="v4-table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Status</th>
                  <th>Run</th>
                  <th>What changed</th>
                  <th>Cost</th>
                  <th>When</th>
                </tr>
              </thead>
              <tbody>
                {all.map((r) => (
                  <tr
                    key={r.id}
                    className="v4-run-row"
                    onClick={() => navigate(`/run/${r.id}`)}
                    tabIndex={0}
                    onKeyDown={(e) => e.key === "Enter" && navigate(`/run/${r.id}`)}
                  >
                    <td>
                      <StatusBadge status={r.status} />
                    </td>
                    <td>
                      <div>
                        <b>{runTitle(r)}</b> <span className="muted">— {r.project_name}</span>
                      </div>
                      <div className="mono muted v4-cmd-cell">{r.command}</div>
                    </td>
                    <td className="muted">{whatChanged(r) || "—"}</td>
                    <td className="muted">{fmtCost(r.estimated_cost)}</td>
                    <td className="muted">{relTime(r.started_at)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </ResourceGate>

      <CoveragePanel res={coverageRes.resource} onRetry={coverageRes.reload} />
    </div>
  );
}

function CoveragePanel({
  res,
  onRetry,
}: {
  res: import("../data").Resource<import("../data").IntegrationCoverage[]>;
  onRetry: () => void;
}) {
  return (
    <div style={{ marginTop: 26 }}>
      <h2 className="section-title">Integration coverage</h2>
      <ResourceGate resource={res} what="integration coverage" onRetry={onRetry}>
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
                    <td>
                      <Cov v={c.command_enforcement} />
                    </td>
                    <td>
                      <Cov v={c.file_review} />
                    </td>
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

function Cov({ v }: { v: boolean | null }) {
  if (v == null)
    return (
      <span className="v4-cov muted">
        <ToneIcon tone="muted" size={12} /> unknown
      </span>
    );
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
