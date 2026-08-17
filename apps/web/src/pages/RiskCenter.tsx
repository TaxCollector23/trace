import { useState } from "react";
import { useParams } from "react-router-dom";
import { api } from "../api";
import type { Severity } from "../api";
import { Loading, RunPicker, SourceBadge, stagger, useAsync } from "../components";

// High → low. Drives both the default sort and the filter-chip order.
const SEVERITY_ORDER: Severity[] = ["high", "medium", "low"];
const SEVERITY_RANK: Record<Severity, number> = { high: 0, medium: 1, low: 2 };
type SeverityFilter = "all" | Severity;

export default function RiskCenter() {
  const { runId } = useParams();
  const [sevFilter, setSevFilter] = useState<SeverityFilter>("all");
  const runsQ = useAsync(() => api.runs());
  const runs = runsQ.data ?? [];
  const current = runId ?? runs[0]?.id;

  const cmdsQ = useAsync(
    () => (current ? api.commands(current) : Promise.resolve([])),
    [current]
  );
  const secretsQ = useAsync(
    () => (current ? api.secrets(current) : Promise.resolve([])),
    [current]
  );
  const policyQ = useAsync(
    () => (current ? api.policyFindings(current) : Promise.resolve([])),
    [current]
  );

  const cmds = cmdsQ.data ?? [];
  const secrets = secretsQ.data ?? [];
  const policyFindings = policyQ.data ?? [];

  // Per-severity counts across all findings (independent of the active filter).
  const sevCounts = policyFindings.reduce(
    (acc, f) => {
      acc[f.severity] += 1;
      return acc;
    },
    { high: 0, medium: 0, low: 0 } as Record<Severity, number>
  );

  // Filter by chip, then sort high → low so the riskiest findings surface first.
  const shownFindings = policyFindings
    .filter((f) => sevFilter === "all" || f.severity === sevFilter)
    .slice()
    .sort((a, b) => SEVERITY_RANK[a.severity] - SEVERITY_RANK[b.severity]);

  // Only the meaningful guard decisions; "executed" updates are not risk signals.
  const guarded = cmds.filter((c) =>
    ["block", "warn", "require_approval", "allow"].includes(c.decision)
  );
  const protectedFiles = secrets.filter((s) => s.secret_type === "protected_file");
  const realSecrets = secrets.filter((s) => s.secret_type !== "protected_file");

  return (
    <div>
      <h1 className="page-title">Command Risk</h1>
      <p className="page-sub">
        Command decisions, policy-engine findings, protected-file warnings, and
        detected secrets for this run. Secret values are always redacted.
      </p>

      {runsQ.loading ? (
        <Loading error={runsQ.error} variant="cards" rows={1} />
      ) : runs.length === 0 ? (
        <div className="empty">No runs recorded yet.</div>
      ) : (
        <>
          <RunPicker runs={runs} current={current} base="/risk" />

          <div className="section-title">Policy engine findings</div>
          {policyQ.loading ? (
            <Loading error={policyQ.error} variant="table" rows={2} />
          ) : policyFindings.length === 0 ? (
            <div className="empty">No deterministic policy findings for this run.</div>
          ) : (
            <>
              <div className="sev-bar" role="group" aria-label="Filter findings by severity">
                <button
                  type="button"
                  className={`sev-chip ${sevFilter === "all" ? "active" : ""}`}
                  aria-pressed={sevFilter === "all"}
                  onClick={() => setSevFilter("all")}
                >
                  All <span className="sev-count">{policyFindings.length}</span>
                </button>
                {SEVERITY_ORDER.map((sev) => (
                  <button
                    key={sev}
                    type="button"
                    className={`sev-chip ${sevFilter === sev ? "active" : ""}`}
                    aria-pressed={sevFilter === sev}
                    onClick={() => setSevFilter(sev)}
                  >
                    {sev.charAt(0).toUpperCase() + sev.slice(1)}{" "}
                    <span className="sev-count">{sevCounts[sev]}</span>
                  </button>
                ))}
              </div>
              {shownFindings.length === 0 ? (
                <div className="empty">No {sevFilter} findings for this run.</div>
              ) : (
                <table>
                  <thead>
                    <tr>
                      <th>Severity</th>
                      <th>Finding</th>
                      <th>File</th>
                    </tr>
                  </thead>
                  <tbody>
                    {shownFindings.map((f, i) => (
                      <tr key={f.id} className="enter" style={stagger(i, 20, 160)}>
                        <td>
                          <span className={`pill sev-${f.severity}`}>{f.severity}</span>
                        </td>
                        <td>
                          <span className="finding-head">
                            <b>{f.title}</b>
                            <span className="rule-key">{f.rule_key}</span>
                            <SourceBadge source={f.source} />
                          </span>
                          <div className="muted finding-desc">{f.description}</div>
                        </td>
                        <td className="mono">{f.file_path ?? "—"}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </>
          )}

          <div className="section-title">Command decisions</div>
          {cmdsQ.loading ? (
            <Loading error={cmdsQ.error} variant="table" rows={3} />
          ) : guarded.length === 0 ? (
            <div className="empty">No guarded commands recorded.</div>
          ) : (
            <table>
              <thead>
                <tr>
                  <th>Decision</th>
                  <th>Command</th>
                  <th>Exit</th>
                </tr>
              </thead>
              <tbody>
                {guarded.map((c, i) => (
                  <tr key={c.id} className="enter" style={stagger(i, 20, 160)}>
                    <td>
                      <span className={`pill ${c.decision}`}>
                        {c.decision.replace("_", " ")}
                      </span>
                    </td>
                    <td className="mono">{c.command}</td>
                    <td>{c.exit_code ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}

          <div className="section-title">Protected file warnings</div>
          {protectedFiles.length === 0 ? (
            <div className="empty">No protected files were touched.</div>
          ) : (
            <table>
              <tbody>
                {protectedFiles.map((s) => (
                  <tr key={s.id}>
                    <td>
                      <span className="pill block">protected</span>
                    </td>
                    <td className="mono">{s.file_path}</td>
                    <td className="muted">{s.action_taken}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}

          <div className="section-title">Secret detection warnings</div>
          {secretsQ.loading ? (
            <Loading error={secretsQ.error} variant="table" rows={2} />
          ) : realSecrets.length === 0 ? (
            <div className="empty">No secrets detected.</div>
          ) : (
            <table>
              <thead>
                <tr>
                  <th>Type</th>
                  <th>Redacted value</th>
                  <th>File</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody>
                {realSecrets.map((s, i) => (
                  <tr key={s.id} className="enter" style={stagger(i, 20, 160)}>
                    <td>{s.secret_type}</td>
                    <td className="mono">{s.redacted_value}</td>
                    <td className="mono">{s.file_path ?? "(output/diff)"}</td>
                    <td className="muted">{s.action_taken}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </>
      )}
    </div>
  );
}
