import { useState } from "react";
import type { Project, RatifyReport } from "../api";
import { api } from "../api";
import { Loading, stagger, useAsync } from "../components";

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
        <div className="empty">No projects yet. Run `trace init` in a repo.</div>
      ) : (
        <>
          {projects.length > 1 && (
            <div className="run-picker">
              <label className="muted" style={{ marginRight: 8 }}>
                Project:
              </label>
              <select value={current} onChange={(e) => setProjectId(e.target.value)}>
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
              <span className="mono">trace review-diff</span> to ratify a local
              diff instead.
            </div>
          )}

          {/* Ratify a specific PR by number */}
          <div className="section-title">Ratify by number</div>
          <div className="btn-row">
            <input
              className="num"
              style={{ width: 120 }}
              value={pr}
              onChange={(e) => setPr(e.target.value.replace(/[^0-9]/g, ""))}
              placeholder="PR #"
            />
            <button
              className="btn"
              onClick={() => pr && ratify(Number(pr))}
              disabled={busy || !current || !pr}
            >
              {busy ? "Ratifying…" : "Ratify"}
            </button>
          </div>

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
          {err && <div className="empty">Error: {err}</div>}
          {report && (
            <>
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
                <table>
                  <thead>
                    <tr>
                      <th>Severity</th>
                      <th>Rule</th>
                      <th>Finding</th>
                      <th>File</th>
                    </tr>
                  </thead>
                  <tbody>
                    {report.findings.map((f, i) => (
                      <tr key={`${f.rule_key}-${i}`} className="enter" style={stagger(i, 15, 200)}>
                        <td>
                          <span
                            className={`pill ${
                              f.severity === "high"
                                ? "block"
                                : f.severity === "medium"
                                ? "warn"
                                : ""
                            }`}
                          >
                            {f.severity}
                          </span>
                        </td>
                        <td className="mono">{f.rule_key}</td>
                        <td>
                          <b>{f.title}</b>
                          <div className="muted">{f.description}</div>
                        </td>
                        <td className="mono">{f.file_path ?? "—"}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </>
          )}
        </>
      )}
    </div>
  );
}
