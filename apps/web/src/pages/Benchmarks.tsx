import { api } from "../api";
import { Loading, stagger, useAsync } from "../components";

export default function Benchmarks() {
  const q = useAsync(() => api.benchmarks());
  const report = q.data;
  const rt = useAsync(() => api.redteamBenchmarks());
  const redteam = rt.data;

  return (
    <div>
      <h1 className="page-title">Benchmarks</h1>
      <p className="page-sub">
        Trace's policy engine scored against its own labeled fixture set —
        computed fresh on every load, straight from the same code the daemon
        and CI use. Not a static snapshot from a past run.
      </p>

      {q.loading ? (
        <Loading error={q.error} variant="kpis" />
      ) : !report ? (
        <div className="empty">Could not load benchmark results.</div>
      ) : (
        <>
          <div className="kpis">
            <div className="kpi">
              <div className="k-val">
                {report.passed}/{report.total}
              </div>
              <div className="k-label">Fixtures passed</div>
            </div>
            <div className="kpi">
              <div className="k-val">{Math.round(report.precision * 100)}%</div>
              <div className="k-label">Precision</div>
            </div>
            <div className="kpi">
              <div className="k-val">{Math.round(report.recall * 100)}%</div>
              <div className="k-label">Recall</div>
            </div>
          </div>
          <p className="note" style={{ marginTop: 14 }}>
            <b>Precision</b>: of everything the engine flagged, how much was
            actually supposed to fire — low precision means false positives
            interrupting real work. <b>Recall</b>: of everything that should
            have fired, how much actually did — low recall means real issues
            slipping through. Every rule is tested with both a case that
            should fire and a deliberate near-miss that shouldn't.
          </p>

          <div className="section-title" style={{ marginTop: 30 }}>
            Fixture results
          </div>
          <table>
            <thead>
              <tr>
                <th>Result</th>
                <th>Fixture</th>
                <th>Expected rule</th>
                <th>Fired</th>
              </tr>
            </thead>
            <tbody>
              {report.results.map((r, i) => (
                <tr key={r.name} className="enter" style={stagger(i, 15, 200)}>
                  <td>
                    <span className={`pill ${r.passed ? "allow" : "block"}`}>{r.passed ? "pass" : "fail"}</span>
                  </td>
                  <td>{r.name}</td>
                  <td className="mono">{r.expected_rule ?? "(nothing)"}</td>
                  <td className="mono">{r.fired_rules.join(", ") || "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>

          <p className="note" style={{ marginTop: 20 }}>
            Want to see the exact fixtures, or add your own? They're in{" "}
            <code>crates/trace-core/src/eval.rs</code> — also runnable from a
            terminal with <code>trace self-check</code>.
          </p>
        </>
      )}

      <h2 className="page-title" style={{ marginTop: 48 }}>
        Red-team detection
      </h2>
      <p className="page-sub">
        An adversarial corpus — dangerous commands (including evasions like{" "}
        <code>curl … | sudo bash</code> and base64-piped shells), planted API
        keys, and unsafe prompts — run through the real command guard, secret
        scanner, and prompt-risk engine. Recall is how many planted threats were
        caught; false positives are benign inputs wrongly flagged.
      </p>

      {rt.loading ? (
        <Loading error={rt.error} variant="kpis" />
      ) : !redteam ? (
        <div className="empty">Could not load red-team results.</div>
      ) : (
        <>
          <div className="kpis">
            <div className="kpi">
              <div className="k-val">
                {redteam.engines.reduce((n, e) => n + e.caught, 0)}/
                {redteam.engines.reduce((n, e) => n + e.threats, 0)}
              </div>
              <div className="k-label">Threats caught</div>
            </div>
            <div className="kpi">
              <div className="k-val">
                {redteam.engines.reduce((n, e) => n + e.false_positives, 0)}
              </div>
              <div className="k-label">False positives</div>
            </div>
            <div className="kpi">
              <div className="k-val">
                {Math.round(
                  (redteam.engines.reduce((n, e) => n + e.caught, 0) /
                    Math.max(
                      1,
                      redteam.engines.reduce((n, e) => n + e.threats, 0),
                    )) *
                    100,
                )}
                %
              </div>
              <div className="k-label">Recall</div>
            </div>
          </div>

          <div className="section-title" style={{ marginTop: 30 }}>
            Per-engine results
          </div>
          <table>
            <thead>
              <tr>
                <th>Result</th>
                <th>Engine</th>
                <th>Caught</th>
                <th>Missed</th>
                <th>Downgraded</th>
                <th>False+</th>
                <th>Recall</th>
              </tr>
            </thead>
            <tbody>
              {redteam.engines.map((e, i) => {
                const clean =
                  e.missed === 0 &&
                  e.downgraded === 0 &&
                  e.false_positives === 0;
                return (
                  <tr key={e.name} className="enter" style={stagger(i, 15, 200)}>
                    <td>
                      <span className={`pill ${clean ? "allow" : "block"}`}>
                        {clean ? "pass" : "fail"}
                      </span>
                    </td>
                    <td>{e.name}</td>
                    <td className="mono">
                      {e.caught}/{e.threats}
                    </td>
                    <td className="mono">{e.missed}</td>
                    <td className="mono">{e.downgraded}</td>
                    <td className="mono">
                      {e.false_positives}/{e.benign}
                    </td>
                    <td className="mono">{Math.round(e.recall * 100)}%</td>
                  </tr>
                );
              })}
            </tbody>
          </table>

          <p className="note" style={{ marginTop: 14 }}>
            Rule pack <code>{redteam.pack_version}</code> —{" "}
            {redteam.injection_phrases} injection phrases,{" "}
            {redteam.command_rules} supplemental command rules,{" "}
            {redteam.secret_patterns} extra secret patterns. The corpus lives in{" "}
            <code>crates/trace-core/src/redteam.rs</code>; reproduce it from a
            terminal with <code>trace self-check</code> or{" "}
            <code>cargo run -p trace-core --example redteam_bench</code>.
          </p>
        </>
      )}
    </div>
  );
}
