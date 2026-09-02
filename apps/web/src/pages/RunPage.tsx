import { useCallback, useEffect, useMemo, useRef } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { runApi } from "../rundata";
import { v4 } from "../data";
import { useResource } from "../useResource";
import {
  deriveState,
  describe,
  type Action,
  type DashboardState,
  type StateInputs,
} from "../state";
import {
  StatusBadge,
  DiffView,
  fmtTime,
  fmtCost,
  relTime,
  runTitle,
  whatChanged,
} from "../components";
import { ResourceGate, RiskTag, ToneIcon, Stalled, Unavailable } from "../v4/ui";
import { StatusStrip } from "../v4/StatusStrip";
import { SignalCard, signalPriority } from "../v4/SignalCard";
import { ApprovalPanel, buildApprovals } from "../v4/ApprovalPanel";
import { toDisplayEvents } from "../v4/events";

// ---------------------------------------------------------------------------
// THE RUN PAGE IS THE PRODUCT (§93). One page: header, live status, timeline,
// risk, incidents, files, tests, diff, controls — understandable without
// visiting six other pages. The layout adapts to the derived dashboard state.
// ---------------------------------------------------------------------------

const LIVE_POLL_MS = 4000;
const STALL_MS = 20000;

export default function RunPage() {
  const { runId } = useParams();
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();

  // Deep-linkable view state (§87): live on/off + severity filter survive reload.
  const liveParam = params.get("live");
  const isLive = liveParam !== "0";
  const setLive = (on: boolean) => {
    const next = new URLSearchParams(params);
    if (on) next.delete("live");
    else next.set("live", "0");
    setParams(next, { replace: true });
  };

  // Resolve which run to show: explicit :runId, else the latest run.
  const runsRes = useResource((s) => runApi.runs(s), { pollMs: 15000 });
  const runs = runsRes.resource.state === "ok" ? runsRes.resource.data : [];
  const currentId = runId ?? runs[0]?.id;

  const active = currentId ? runs.find((r) => r.id === currentId) : undefined;
  const runIsActive = active ? !isTerminal(active.status) : false;
  const pollMs = isLive && runIsActive ? LIVE_POLL_MS : undefined;

  // Per-run resources. Live ones poll while the run is active.
  const runRes = useResource((s) => (currentId ? runApi.run(currentId, s) : neverResolve()), {
    deps: [currentId],
    pollMs,
  });
  const eventsRes = useResource((s) => (currentId ? v4.events(currentId, s) : neverResolve()), {
    deps: [currentId],
    pollMs,
    stallAfterMs: runIsActive ? STALL_MS : undefined,
  });
  const signalsRes = useResource((s) => (currentId ? v4.signals(currentId, s) : neverResolve()), {
    deps: [currentId],
    pollMs,
  });
  const incidentsRes = useResource(
    (s) => (currentId ? v4.incidents(currentId, s) : neverResolve()),
    { deps: [currentId], pollMs }
  );
  const commandsRes = useResource((s) => (currentId ? runApi.commands(currentId, s) : neverResolve()), {
    deps: [currentId],
    pollMs,
  });
  const secretsRes = useResource((s) => (currentId ? runApi.secrets(currentId, s) : neverResolve()), {
    deps: [currentId],
  });
  const filesRes = useResource((s) => (currentId ? runApi.files(currentId, s) : neverResolve()), {
    deps: [currentId],
    pollMs,
  });
  const testsRes = useResource((s) => (currentId ? runApi.tests(currentId, s) : neverResolve()), {
    deps: [currentId],
  });
  const diffRes = useResource((s) => (currentId ? runApi.diff(currentId, s) : neverResolve()), {
    deps: [currentId],
  });
  const checkpointsRes = useResource(
    (s) => (currentId ? runApi.checkpoints(currentId, s) : neverResolve()),
    { deps: [currentId] }
  );
  const costRes = useResource((s) => (currentId ? runApi.cost(currentId, s) : neverResolve()), {
    deps: [currentId],
  });
  const policyRes = useResource((s) => (currentId ? runApi.policy(currentId, s) : neverResolve()), {
    deps: [currentId],
  });

  const run = runRes.resource.state === "ok" ? runRes.resource.data : active ?? null;

  const stateInputs: StateInputs = {
    health: { state: "ok", data: { status: "ok" } }, // page reached ⇒ daemon answered
    runs: runsRes.resource,
    run: run,
    events: eventsRes.resource,
    signals: signalsRes.resource,
    incidents: incidentsRes.resource,
  };
  const state: DashboardState = deriveState(stateInputs);
  const descriptor = describe(state);

  // --- section refs for keyboard / action focus ---------------------------
  const refs = {
    attention: useRef<HTMLDivElement>(null),
    timeline: useRef<HTMLDivElement>(null),
    risk: useRef<HTMLDivElement>(null),
    files: useRef<HTMLDivElement>(null),
    tests: useRef<HTMLDivElement>(null),
    diff: useRef<HTMLDivElement>(null),
    controls: useRef<HTMLDivElement>(null),
  };
  const scrollTo = (k: keyof typeof refs) =>
    refs[k].current?.scrollIntoView({ behavior: "smooth", block: "start" });

  const reloadAll = useCallback(() => {
    runsRes.reload();
    runRes.reload();
    eventsRes.reload();
    signalsRes.reload();
    incidentsRes.reload();
    commandsRes.reload();
    filesRes.reload();
    diffRes.reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const onAction = (a: Action) => {
    if (a.to) return navigate(a.to);
    switch (a.command) {
      case "focus-approval":
      case "focus-controls":
        return scrollTo(a.command === "focus-approval" ? "attention" : "controls");
      case "focus-risk":
        return scrollTo("risk");
      case "focus-failure":
        return scrollTo("timeline");
      case "focus-diff":
        return scrollTo("diff");
      case "focus-timeline":
        return scrollTo("timeline");
      case "focus-latest":
        return; // already here
      case "toggle-live":
        return setLive(!isLive);
      case "reload":
        return reloadAll();
      default:
        return;
    }
  };

  // Keyboard shortcuts (§85). Ignored while typing in a field.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const t = e.target as HTMLElement;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
      switch (e.key) {
        case "a":
          scrollTo("attention");
          break;
        case "t":
          scrollTo("timeline");
          break;
        case "r":
          scrollTo("risk");
          break;
        case "d":
          scrollTo("diff");
          break;
        case "f":
          scrollTo("files");
          break;
        case "l":
          setLive(!isLive);
          break;
        case "Escape":
          navigate("/");
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isLive]);

  // Approvals + prioritized signals (derived from real data only).
  const approvals = useMemo(
    () =>
      buildApprovals(
        run,
        eventsRes.resource.state === "ok" ? toRawEvents(eventsRes.resource.data) : [],
        signalsRes.resource.state === "ok" ? signalsRes.resource.data : [],
        commandsRes.resource.state === "ok" ? commandsRes.resource.data : []
      ),
    [run, eventsRes.resource, signalsRes.resource, commandsRes.resource]
  );

  if (!currentId) {
    return (
      <div>
        <h1 className="page-title">Run</h1>
        <ResourceGate resource={runsRes.resource} what="runs" onRetry={runsRes.reload}>
          {() => <div className="v4-empty muted">No run selected.</div>}
        </ResourceGate>
      </div>
    );
  }

  const duration = run ? durationOf(run.started_at, run.ended_at) : "—";

  return (
    <div className="v4-runpage">
      {/* Header ---------------------------------------------------------- */}
      <div className="v4-runhead">
        <div className="v4-runhead-top">
          <div>
            <div className="v4-runhead-title">
              {run ? runTitle(run) : "Run"}
              {run && <span className="muted"> — {run.project_name}</span>}
            </div>
            {run && <div className="run-cmd-line mono">{run.command}</div>}
          </div>
          {run && <StatusBadge status={run.status} />}
        </div>
        {run && (
          <div className="run-meta">
            <span>started {fmtTime(run.started_at)}</span>
            <span>{run.ended_at ? `duration ${duration}` : `running ${duration}`}</span>
            <span>exit {run.exit_code ?? "—"}</span>
            <span>cost {fmtCost(run.estimated_cost)}</span>
            {whatChanged(run) && <span>{whatChanged(run)}</span>}
          </div>
        )}
      </div>

      <StatusStrip
        descriptor={descriptor}
        onAction={onAction}
        right={
          runIsActive ? (
            <span className={`v4-live ${isLive ? "on" : "off"}`}>
              <span className="v4-live-dot" /> {isLive ? "Live" : "Paused"}
            </span>
          ) : undefined
        }
      />

      {eventsRes.stalled && runIsActive && <Stalled onRetry={eventsRes.reload} />}

      {/* Attention: approvals + incidents + top signals ------------------ */}
      <section ref={refs.attention} id="sec-attention" aria-label="Attention">
        <h2 className="section-title">Attention</h2>
        {approvals.length > 0 &&
          approvals.map((r) => <ApprovalPanel key={r.id} req={r} />)}

        <ResourceGate
          resource={incidentsRes.resource}
          what="incidents"
          onRetry={incidentsRes.reload}
          empty={<div className="v4-ok-line"><ToneIcon tone="success" /> No open incidents.</div>}
        >
          {(incidents) => (
            <div className="v4-incidents">
              {incidents.map((inc) => (
                <div key={inc.id} className={`v4-incident sev-${String(inc.severity).toLowerCase()}`}>
                  <div className="v4-incident-head">
                    <RiskTag level={String(inc.severity)} />
                    <b>{inc.title}</b>
                    <span className="pill">{inc.status}</span>
                  </div>
                  <p className="muted">{inc.summary}</p>
                  <div className="muted v4-incident-meta">
                    first seen {relTime(inc.first_seen)} · last seen {relTime(inc.last_seen)} ·{" "}
                    {inc.signal_ids.length} signal{inc.signal_ids.length === 1 ? "" : "s"}
                  </div>
                </div>
              ))}
            </div>
          )}
        </ResourceGate>

        <div ref={refs.risk} id="sec-risk">
          <ResourceGate
            resource={signalsRes.resource}
            what="risk signals"
            onRetry={signalsRes.reload}
            empty={<div className="v4-ok-line"><ToneIcon tone="success" /> No risk signals raised.</div>}
          >
            {(signals) => (
              <div className="v4-signals">
                {[...signals]
                  .sort((a, b) => signalPriority(b) - signalPriority(a))
                  .map((s) => (
                    <SignalCard key={s.id} signal={s} onEvidence={() => scrollTo("timeline")} />
                  ))}
              </div>
            )}
          </ResourceGate>
        </div>
      </section>

      {/* Command guard + policy (real today) ----------------------------- */}
      <section aria-label="Guarded commands & policy">
        <h2 className="section-title">Command guard & policy</h2>
        <ResourceGate
          resource={commandsRes.resource}
          what="guarded commands"
          onRetry={commandsRes.reload}
          empty={<div className="v4-empty muted">No guarded commands recorded.</div>}
        >
          {(cmds) => (
            <div className="v4-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>Decision</th>
                    <th>Command</th>
                    <th>Exit</th>
                    <th>When</th>
                  </tr>
                </thead>
                <tbody>
                  {cmds.map((c) => (
                    <tr key={c.id}>
                      <td>
                        <span className={`pill ${c.decision}`}>{c.decision.replace("_", " ")}</span>
                      </td>
                      <td className="mono v4-cmd-cell">{c.command}</td>
                      <td>{c.exit_code ?? "—"}</td>
                      <td className="muted">{relTime(c.created_at)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </ResourceGate>

        <div style={{ marginTop: 14 }}>
          <ResourceGate
            resource={policyRes.resource}
            what="policy findings"
            onRetry={policyRes.reload}
            empty={<div className="v4-ok-line"><ToneIcon tone="success" /> No policy findings.</div>}
          >
            {(findings) => (
              <div className="v4-findings">
                {findings.map((f) => (
                  <div key={f.id} className={`v4-finding sev-${f.severity}`}>
                    <div className="finding-head">
                      <RiskTag level={f.severity} />
                      <b>{f.title}</b>
                      <span className="rule-key">{f.rule_key}</span>
                    </div>
                    <div className="finding-desc">{f.description}</div>
                    {f.file_path && <div className="mono muted">{f.file_path}</div>}
                  </div>
                ))}
              </div>
            )}
          </ResourceGate>
        </div>
      </section>

      {/* Timeline -------------------------------------------------------- */}
      <section ref={refs.timeline} id="sec-timeline" aria-label="Timeline">
        <h2 className="section-title">Timeline</h2>
        <ResourceGate resource={eventsRes.resource} what="events" onRetry={eventsRes.reload}>
          {(events) => (
            <div className="timeline">
              {toDisplayEvents(events).map((e, i) => (
                <div className="tl-item" key={e.id + i}>
                  <div className="tl-time">{e.ts ? fmtTime(e.ts) : "—"}</div>
                  <div className="tl-msg">
                    {e.risk && e.risk !== "none" && <RiskTag level={e.risk} />} {e.message}
                  </div>
                  <div className="tl-type">
                    {e.kind}
                    {e.actor ? ` · ${e.actor}` : ""}
                    {e.status ? ` · ${e.status}` : ""}
                  </div>
                </div>
              ))}
            </div>
          )}
        </ResourceGate>
      </section>

      {/* Files ----------------------------------------------------------- */}
      <section ref={refs.files} id="sec-files" aria-label="Files changed">
        <h2 className="section-title">Files changed</h2>
        <ResourceGate
          resource={filesRes.resource}
          what="file changes"
          onRetry={filesRes.reload}
          empty={<div className="v4-empty muted">No file changes recorded.</div>}
        >
          {(files) => (
            <div className="v4-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>Change</th>
                    <th>Path</th>
                    <th>Summary</th>
                  </tr>
                </thead>
                <tbody>
                  {files.map((f) => (
                    <tr key={f.id}>
                      <td>
                        <span className={`pill ${f.change_type}`}>{f.change_type}</span>
                      </td>
                      <td className="mono">{f.path}</td>
                      <td className="muted">{f.diff_summary ?? "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </ResourceGate>

        <div style={{ marginTop: 12 }}>
          <ResourceGate
            resource={secretsRes.resource}
            what="secret findings"
            onRetry={secretsRes.reload}
            empty={<div className="v4-ok-line"><ToneIcon tone="success" /> No secrets flagged.</div>}
          >
            {(secrets) => (
              <div className="v4-findings">
                {secrets.map((s) => (
                  <div key={s.id} className="v4-finding sev-medium">
                    <div className="finding-head">
                      <RiskTag level="medium" />
                      <b>{s.secret_type.replace("_", " ")}</b>
                      <span className="pill">{s.action_taken}</span>
                    </div>
                    <div className="finding-desc mono">
                      {s.file_path ?? "—"} · {s.redacted_value}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </ResourceGate>
        </div>
      </section>

      {/* Tests ----------------------------------------------------------- */}
      <section ref={refs.tests} id="sec-tests" aria-label="Tests">
        <h2 className="section-title">Tests</h2>
        <ResourceGate
          resource={testsRes.resource}
          what="test results"
          onRetry={testsRes.reload}
          empty={<div className="v4-empty muted">No test results recorded for this run.</div>}
        >
          {(tests) => (
            <div className="v4-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>Status</th>
                    <th>Command</th>
                    <th>Summary</th>
                  </tr>
                </thead>
                <tbody>
                  {tests.map((t) => (
                    <tr key={t.id}>
                      <td>
                        <span className={`pill ${t.status}`}>{t.status}</span>
                      </td>
                      <td className="mono">{t.command}</td>
                      <td className="muted">{t.output_summary ?? "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </ResourceGate>
      </section>

      {/* Diff ------------------------------------------------------------ */}
      <section ref={refs.diff} id="sec-diff" aria-label="Diff">
        <h2 className="section-title">Diff</h2>
        <ResourceGate resource={diffRes.resource} what="diff" onRetry={diffRes.reload}>
          {(d) => <DiffView diff={d.diff} />}
        </ResourceGate>
      </section>

      {/* Controls: checkpoints, rollback, cost --------------------------- */}
      <section ref={refs.controls} id="sec-controls" aria-label="Controls">
        <h2 className="section-title">Controls</h2>
        <div className="v4-controls">
          <ResourceGate
            resource={checkpointsRes.resource}
            what="restore points"
            onRetry={checkpointsRes.reload}
            empty={<div className="v4-empty muted">No restore points captured.</div>}
          >
            {(cps) => (
              <div className="v4-checkpoints">
                <div className="muted" style={{ marginBottom: 8 }}>
                  {cps.length} restore point{cps.length === 1 ? "" : "s"}. Roll back from the
                  Rollback page, where the exact restore target is shown before you confirm.
                </div>
                {cps.map((c) => (
                  <div key={c.id} className="v4-checkpoint mono">
                    {c.checkpoint_type} · {c.git_ref ?? "no git ref"} · {relTime(c.created_at)}
                  </div>
                ))}
                <button className="btn" style={{ marginTop: 10 }} onClick={() => navigate("/rollback")}>
                  Open rollback controls
                </button>
              </div>
            )}
          </ResourceGate>

          <div className="v4-cost">
            <ResourceGate
              resource={costRes.resource}
              what="token usage"
              onRetry={costRes.reload}
              empty={<div className="v4-empty muted">No token usage recorded.</div>}
            >
              {(c) => (
                <div>
                  <div className="muted">Estimated cost</div>
                  <div className="k-val" style={{ fontSize: 24 }}>
                    {c.total_estimated != null ? fmtCost(c.total_estimated) : "unavailable"}
                  </div>
                  {c.has_unavailable && (
                    <div className="muted" style={{ fontSize: 12 }}>
                      Some usage rows lack pricing and are not estimated.
                    </div>
                  )}
                </div>
              )}
            </ResourceGate>
          </div>
        </div>
      </section>

      {runRes.resource.state === "unavailable" && !run && (
        <Unavailable
          title="Run header unavailable"
          reason={runRes.resource.reason}
          onRetry={runRes.reload}
        />
      )}
    </div>
  );
}

// --- helpers ---------------------------------------------------------------

function isTerminal(status: string): boolean {
  return ["completed", "failed", "blocked", "rolled_back", "aborted", "interrupted"].includes(
    status
  );
}

/** A promise that stays pending; used when there is no run id so the hook
 * simply stays in its loading state instead of firing a bad request. */
function neverResolve<T>(): Promise<T> {
  return new Promise<T>(() => {});
}

function durationOf(start: string, end: string | null): string {
  const s = new Date(start).getTime();
  const e = end ? new Date(end).getTime() : Date.now();
  if (Number.isNaN(s) || Number.isNaN(e) || e < s) return "—";
  const sec = Math.round((e - s) / 1000);
  if (sec < 60) return `${sec}s`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ${sec % 60}s`;
  const hr = Math.floor(min / 60);
  return `${hr}h ${min % 60}m`;
}

/** buildApprovals expects the raw NormalizedEvent[] shape; pass it through. */
function toRawEvents(
  events: import("../data").NormalizedEvent[]
): import("../data").NormalizedEvent[] {
  return events;
}
