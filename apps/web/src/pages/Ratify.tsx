import { useState } from "react";
import type { Project, RatifyReport, Severity } from "../api";
import { api } from "../api";
import { Loading, SourceBadge, stagger, useAsync } from "../components";

const VERDICT_PILL: Record<RatifyReport["verdict"], string> = {
  pass: "allow",
  review: "warn",
  block: "block",
};

const VERDICT_LABEL: Record<RatifyReport["verdict"], string> = {
  pass: "pass",
  review: "needs review",
  block: "block",
};

// High → low, so the riskiest findings are grouped first.
const SEVERITY_ORDER: Severity[] = ["high", "medium", "low"];
const SEVERITY_LABEL: Record<Severity, string> = {
  high: "High",
  medium: "Medium",
  low: "Low",
};
const SEVERITY_PILL: Record<Severity, string> = {
  high: "block",
  medium: "warn",
  low: "",
};

export default function Ratify() {
  const projectsQ = useAsync(() => api.dashboard());
  const projects: Project[] = projectsQ.data?.projects ?? [];
  const [projectId, setProjectId] = useState<string>("");
  const current = projectId || projects[0]?.id;

  const statusQ = useAsync(
    () => (current ? api.githubStatus(current) : Promise.resolve(null)),
    [current]
  );
  const pullsQ = useAsync(
    () => (current ? api.githubPulls(current) : Promise.resolve([])),
    [current]
  );

  const [pr, setPr] = useState<string>("");
  const [report, setReport] = useState<RatifyReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  // Switching projects must clear any prior verdict — otherwise PR #N's result
  // from the old repo stays on screen, now falsely attributed to the new one.
  function changeProject(id: string) {
    setProjectId(id);
    setReport(null);
    setErr(null);
    setPr("");
  }

  async function ratify(prNumber: number) {
    if (!current) return;
    setBusy(true);
    setErr(null);
    setReport(null);
    setPr(String(prNumber));
    try {
      setReport(await api.ratifyPull(current, prNumber));
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  const status = statusQ.data;

  return (
    <div>
      <h1 className="page-title">Ratify</h1>
      <p className="page-sub">
        Ratify a pull request on your connected GitHub repository against
        Trace's deterministic policy engine — secret scanning, risky-change
        detection, and the rest of the same rules the live guard uses. Pure
        pattern matching: no LLM, <b>no API key required</b>. A pull request is{" "}
        <span className="pill block">block</span> if it trips any high-severity
        rule, <span className="pill warn">needs review</span> for medium-only,
        else <span className="pill allow">pass</span>.
      </p>

      {projectsQ.loading ? (
        <Loading error={projectsQ.error} variant="cards" rows={1} />
      ) : projects.length === 0 ? (
        <div className="empty">No projects yet. Run `trc init` in a repo.</div>
      ) : (
        <>
          {projects.length > 1 && (
            <div className="run-picker">
              <label className="muted" style={{ marginRight: 8 }}>
                Project:
              </label>
              <select
                aria-label="Project"
                value={current}
                onChange={(e) => changeProject(e.target.value)}
              >
                {projects.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </div>
          )}

          {status && !status.repo_ref && (
            <div className="note warn-note">
              This project has no GitHub <span className="mono">origin</span>{" "}
              remote, so there's nothing to ratify. Add a remote, or use{" "}
              <span className="mono">trc review-diff</span> to ratify a local
              diff instead.
            </div>
          )}

          {/* Ratify a specific PR by number. Gated on a real GitHub remote so
              the button can't fire a request that's guaranteed to fail. */}
          <div className="section-title">Ratify by number</div>
          <form
            className="btn-row"
            onSubmit={(e) => {
              e.preventDefault();
              if (pr && status?.repo_ref) ratify(Number(pr));
            }}
          >
            <input
              className="num"
              style={{ width: 120 }}
              aria-label="Pull request number"
              value={pr}
              onChange={(e) => setPr(e.target.value.replace(/[^0-9]/g, ""))}
              placeholder="PR #"
            />
            <button
              className="btn"
              type="submit"
              disabled={busy || !current || !pr || !status?.repo_ref}
            >
              {busy ? "Ratifying…" : "Ratify"}
            </button>
          </form>

          {/* Open PRs, each ratifiable in one click */}
          <div className="section-title">Open pull requests</div>
          {pullsQ.loading ? (
            <Loading error={pullsQ.error} variant="table" rows={2} />
          ) : (pullsQ.data ?? []).length === 0 ? (
            <div className="empty">No open pull requests.</div>
          ) : (
            <table>
              <tbody>
                {(pullsQ.data ?? []).map((p, i) => (
                  <tr key={p.number} className="enter" style={stagger(i, 20, 160)}>
                    <td className="mono">#{p.number}</td>
                    <td>
                      <a href={p.html_url} target="_blank" rel="noreferrer">
                        {p.title}
                      </a>
                    </td>
                    <td className="muted">@{p.user}</td>
                    <td style={{ textAlign: "right" }}>
                      <button
                        className="btn"
                        onClick={() => ratify(p.number)}
                        disabled={busy || !current}
                      >
                        Ratify
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}

          {/* Result */}
          {err && (
            <div className="empty" role="alert">
              Error: {err}
            </div>
          )}
          {report && (
            <div role="status" aria-live="polite">
              <div className="section-title" style={{ marginTop: 30 }}>
                Verdict for PR #{report.pr}
              </div>
              <div className="kpis">
                <div className="kpi">
                  <div className="k-val">
                    <span className={`pill ${VERDICT_PILL[report.verdict]}`}>
                      {VERDICT_LABEL[report.verdict]}
                    </span>
                  </div>
                  <div className="k-label">Verdict</div>
                </div>
                <div className="kpi">
                  <div className="k-val">{report.files_reviewed}</div>
                  <div className="k-label">Files reviewed</div>
                </div>
                <div className="kpi">
                  <div className="k-val">{report.findings.length}</div>
                  <div className="k-label">Findings</div>
                </div>
                <div className="kpi">
                  <div className="k-val">
                    {report.counts.high}·{report.counts.medium}·{report.counts.low}
                  </div>
                  <div className="k-label">High · Med · Low</div>
                </div>
              </div>

              {report.findings.length === 0 ? (
                <div className="empty">
                  Clean — the policy engine found nothing to flag in this PR.
                </div>
              ) : (
                // Grouped High → Medium → Low; empty groups are omitted so a
                // PR that only trips medium rules shows just the Medium group.
                SEVERITY_ORDER.map((sev) => {
                  const group = report.findings.filter((f) => f.severity === sev);
                  if (group.length === 0) return null;
                  return (
                    <div key={sev}>
                      <div className="section-title">
                        <span className={`pill ${SEVERITY_PILL[sev]}`}>
                          {SEVERITY_LABEL[sev]}
                        </span>{" "}
                        {SEVERITY_LABEL[sev]} severity{" "}
                        <span className="muted">({group.length})</span>
                      </div>
                      <table>
                        <thead>
                          <tr>
                            <th>Finding</th>
                            <th>File</th>
                          </tr>
                        </thead>
                        <tbody>
                          {group.map((f, i) => (
                            <tr
                              key={`${f.rule_key}-${i}`}
                              className="enter"
                              style={stagger(i, 15, 200)}
                            >
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
                    </div>
                  );
                })
              )}
            </div>
          )}
        </>
      )}
    </div>
  );
}
