import { useSearchParams } from "react-router-dom";
import { runApi } from "../rundata";
import { v4 } from "../data";
import type { RunCounts } from "../data";
import type { RunSummary } from "../api";
import { useResource } from "../useResource";
import { relTime, runTitle } from "../components";
import { ResourceGate, ToneIcon } from "../v4/ui";
import { fmtDuration } from "./RunPage";

// ---------------------------------------------------------------------------
// Run diff / behavior diff (Wave 2, Agent 2). NOT a git diff — execution
// behavior: command/file/test-cycle/approval/block counts for two runs, plus
// a one-line factual narrative synthesized from the real deltas by the
// backend (a fixed template over numbers, never an LLM call — see
// `trace_core::intel::similarity::compare_runs`). Reachable at
// `#/compare?a=<runId>&b=<runId>`, typically from the Run Page's "Similar
// runs" panel.
// ---------------------------------------------------------------------------

export default function ComparePage() {
  const [params, setParams] = useSearchParams();
  const aId = params.get("a") ?? undefined;
  const bId = params.get("b") ?? undefined;

  const runsRes = useResource((s) => runApi.runs(s));
  const runs = runsRes.resource.state === "ok" ? runsRes.resource.data : [];

  const compareRes = useResource(
    (s) => (aId && bId ? v4.compareRuns(aId, bId, s) : neverResolve()),
    { deps: [aId, bId] }
  );

  const setRun = (which: "a" | "b", runId: string) => {
    const next = new URLSearchParams(params);
    if (runId) next.set(which, runId);
    else next.delete(which);
    setParams(next, { replace: true });
  };

  return (
    <div className="v4-runpage">
      <h1 className="page-title">Compare runs</h1>
      <p className="muted" style={{ marginTop: -8, marginBottom: 18 }}>
        Execution behavior, not a git diff — command, file, test, and approval counts for two
        runs, side by side.
      </p>

      <div className="v4-compare-picker">
        <label className="muted" htmlFor="compare-run-a">
          Run A:
        </label>
        <select
          id="compare-run-a"
          value={aId ?? ""}
          onChange={(e) => setRun("a", e.target.value)}
        >
          <option value="" disabled>
            Select a run…
          </option>
          {runs.map((r) => (
            <option key={r.id} value={r.id}>
              {pickerLabel(r)}
            </option>
          ))}
        </select>
        <label className="muted" htmlFor="compare-run-b">
          Run B:
        </label>
        <select
          id="compare-run-b"
          value={bId ?? ""}
          onChange={(e) => setRun("b", e.target.value)}
        >
          <option value="" disabled>
            Select a run…
          </option>
          {runs.map((r) => (
            <option key={r.id} value={r.id}>
              {pickerLabel(r)}
            </option>
          ))}
        </select>
      </div>

      {!aId || !bId ? (
        <div className="v4-empty muted">Select two runs above to compare their behavior.</div>
      ) : (
        <ResourceGate resource={compareRes.resource} what="run comparison" onRetry={compareRes.reload}>
          {(cmp) => (
            <>
              {cmp.narrative ? (
                <div className="v4-compare-narrative">
                  <ToneIcon tone="info" /> {cmp.narrative}
                </div>
              ) : (
                <div className="v4-uninstrumented" role="note">
                  <ToneIcon tone="muted" />
                  <div>
                    <b>Not enough data to compare meaningfully</b>
                    <div className="muted">
                      At least one of these runs recorded zero commands — a ratio against that
                      would misrepresent the comparison, so Trace shows the counts below without
                      a synthesized summary.
                    </div>
                  </div>
                </div>
              )}

              <div className="v4-compare-grid">
                <RunCountsCard label="Run A" counts={cmp.run_a} />
                <RunCountsCard label="Run B" counts={cmp.run_b} />
              </div>
            </>
          )}
        </ResourceGate>
      )}
    </div>
  );
}

function RunCountsCard({ label, counts }: { label: string; counts: RunCounts }) {
  const rows: [string, string][] = [
    ["Commands", String(counts.commands)],
    ["Files changed", String(counts.files)],
    ["Test cycles", String(counts.test_cycles)],
    ["Approvals required", String(counts.approvals)],
    ["Blocked commands", String(counts.blocks)],
    ["Duration", fmtDuration(counts.duration_seconds)],
    ["Outcome", counts.outcome.replace(/_/g, " ")],
  ];
  return (
    <div className="v4-compare-col">
      <div className="muted" style={{ fontSize: 12, textTransform: "uppercase", letterSpacing: 0.4 }}>
        {label}
      </div>
      <h3 className="mono">{counts.command}</h3>
      <div className="muted" style={{ fontSize: 12 }}>
        started {relTime(counts.started_at)}
      </div>
      <dl className="v4-compare-rows">
        {rows.map(([k, v]) => (
          <div className="v4-compare-row" key={k}>
            <dt>{k}</dt>
            <dd>{v}</dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

/** Picker label: agent + the run's actual command, truncated, plus a short
 * id — the command text is what actually distinguishes runs from the same
 * agent, so `runTitle` alone (just the agent name) is not enough here. */
function pickerLabel(r: RunSummary): string {
  const cmd = r.command.length > 48 ? `${r.command.slice(0, 48)}…` : r.command;
  return `${runTitle(r)} — ${cmd} (${r.id.slice(0, 8)})`;
}

function neverResolve<T>(): Promise<T> {
  return new Promise<T>(() => {});
}
