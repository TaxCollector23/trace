import { useParams } from "react-router-dom";
import { api } from "../api";
import { Loading, RunPicker, stagger, useAsync } from "../components";

export default function RiskCenter() {
  const { runId } = useParams();
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
  const judgeQ = useAsync(
    () => (current ? api.judgeVerdicts(current) : Promise.resolve([])),
    [current]
  );

  const cmds = cmdsQ.data ?? [];
  const secrets = secretsQ.data ?? [];
  const policyFindings = policyQ.data ?? [];
  const verdicts = judgeQ.data ?? [];

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
        Command decisions, policy-engine findings, 3-LLM judge verdicts,
        protected-file warnings, and detected secrets for this run. Secret
        values are always redacted.
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
            <table>
              <thead>
                <tr>
                  <th>Severity</th>
                  <th>Finding</th>
                  <th>File</th>
                </tr>
              </thead>
              <tbody>
                {policyFindings.map((f, i) => (
                  <tr key={f.id} className="enter" style={stagger(i, 20, 160)}>
                    <td>
                      <span className={`pill ${f.severity === "high" ? "block" : f.severity === "medium" ? "require_approval" : ""}`}>
                        {f.severity}
                      </span>
                    </td>
                    <td>
                      <b>{f.title}</b>
                      <div className="muted" style={{ fontSize: 12.5 }}>{f.description}</div>
                    </td>
                    <td className="mono">{f.file_path ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}

          <div className="section-title">3-LLM judge verdicts</div>
          {judgeQ.loading ? (
            <Loading error={judgeQ.error} variant="cards" rows={1} />
          ) : verdicts.length === 0 ? (
            <div className="empty">
              No judge verdicts for this run — either the judge is disabled, or
              analysis hasn't run yet.
            </div>
          ) : (
            verdicts.map((v) => (
              <div key={v.id} className="card" style={{ marginBottom: 12 }}>
                <div className="run-head">
                  <span className={`pill ${v.consensus}`}>{v.consensus.replace("_", " ")}</span>
                  <span className="muted">
                    {Math.round(v.agreement * 100)}% agreement · {Math.round(v.confidence * 100)}% confidence
                  </span>
                </div>
                <p style={{ margin: "8px 0 0" }}>{v.summary}</p>
              </div>
            ))
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
