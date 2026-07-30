import { useEffect, useState } from "react";
import type { AgentStatus, CompressionResult, LaunchOutcome } from "../api";
import { api } from "../api";

const MODES: [string, string][] = [
  ["normal", "Normal — light cleanup, keep readability"],
  ["concise", "Concise — remove redundancy, full sentences"],
  ["bare", "Bare Mode — maximum compression + strict response rules"],
];

const BUDGETS: [string, string][] = [
  ["tiny", "Tiny"],
  ["short", "Short"],
  ["normal", "Normal"],
  ["detailed", "Detailed"],
];

export default function PromptCompressor() {
  const [input, setInput] = useState("");
  const [mode, setMode] = useState("concise");
  const [result, setResult] = useState<CompressionResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [budgetPreset, setBudgetPreset] = useState("short");
  const [includeRules, setIncludeRules] = useState(true);

  const [agents, setAgents] = useState<AgentStatus[]>([]);
  const [target, setTarget] = useState("auto");
  const [launching, setLaunching] = useState(false);
  const [outcome, setOutcome] = useState<LaunchOutcome | null>(null);

  useEffect(() => {
    api.agents().then(setAgents).catch(() => {});
  }, []);

  async function compress() {
    if (!input.trim()) return;
    setBusy(true);
    setError(null);
    setOutcome(null);
    try {
      setResult(await api.compressPrompt(input, mode));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  /** Compressed prompt + (optionally) the output-discipline rules. */
  async function finalPrompt(): Promise<string> {
    if (!result) return "";
    let out = result.compressed;
    if (budgetPreset) {
      try {
        const b = await api.outputBudget(budgetPreset);
        if (b.instruction_block) out += "\n\n" + b.instruction_block;
      } catch {
        /* budget is optional */
      }
    }
    if (includeRules && result.response_rules) {
      out += "\n\n" + result.response_rules;
    }
    return out;
  }

  function copy(text: string) {
    navigator.clipboard?.writeText(text);
  }

  async function launch() {
    if (!result) return;
    setLaunching(true);
    setOutcome(null);
    try {
      const prompt = await finalPrompt();
      setOutcome(await api.launch(target, prompt));
    } catch (e) {
      setOutcome({ method: "error", launched: false, message: String(e) });
    } finally {
      setLaunching(false);
    }
  }

  const launchable = [
    { id: "auto", name: "Auto (route to best installed)" },
    ...agents.map((a) => ({
      id: a.id,
      name: `${a.name}${a.surface !== "web" ? (a.installed ? " — installed" : " — not installed") : " — web"}`,
    })),
  ];

  return (
    <div>
      <h1 className="page-title">Prompt Compressor</h1>
      <p className="page-sub">
        TraceCompress shrinks your prompt, enforces output discipline, and
        launches it straight into your AI agent. Compression is local and
        deterministic; token counts are estimates.
      </p>

      <div className="section-title">Prompt</div>
      <textarea
        className="ta"
        placeholder="Paste your messy prompt…"
        value={input}
        onChange={(e) => setInput(e.target.value)}
        rows={6}
      />

      <div className="section-title">Mode</div>
      <div className="mode-row">
        {MODES.map(([val, label]) => (
          <label key={val} className={`mode-card ${mode === val ? "active" : ""}`}>
            <input type="radio" name="mode" checked={mode === val} onChange={() => setMode(val)} />
            {label}
          </label>
        ))}
      </div>

      <div style={{ marginTop: 12 }}>
        <button className="btn" onClick={compress} disabled={busy || !input.trim()}>
          {busy ? "Compressing…" : "Compress"}
        </button>
      </div>
      {error && <div className="empty">Error: {error}</div>}

      {result && (
        <>
          {result.conflicts.length > 0 && (
            <div className="note warn-note" style={{ marginTop: 16 }}>
              <b>Possible conflicting instructions</b> — resolve before relying on this compression:
              <ul style={{ margin: "6px 0 0" }}>
                {result.conflicts.map((c, i) => (
                  <li key={i}>{c}</li>
                ))}
              </ul>
            </div>
          )}

          <div className="note" style={{ marginTop: 14 }}>
            Estimated tokens: <b>{result.original_tokens}</b> →{" "}
            <b>{result.compressed_tokens}</b> (~{result.reduction_pct.toFixed(0)}% smaller) · mode{" "}
            <b>{result.mode}</b>
          </div>

          <div className="section-title">Compressed prompt</div>
          <pre className="diff" style={{ whiteSpace: "pre-wrap" }}>
            {result.compressed}
          </pre>

          {/* Launch — the headline action */}
          <div className="section-title">Launch into an agent</div>
          <div className="launch-row">
            <select className="run-picker-select" value={target} onChange={(e) => setTarget(e.target.value)}>
              {launchable.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name}
                </option>
              ))}
            </select>
            <label className="check" style={{ whiteSpace: "nowrap" }}>
              <input type="checkbox" checked={includeRules} onChange={(e) => setIncludeRules(e.target.checked)} />
              attach response rules
            </label>
            <button className="btn" onClick={launch} disabled={launching}>
              {launching ? "Launching…" : "Launch"}
            </button>
            <button className="btn" style={{ background: "var(--bg-elev-2)", color: "var(--text)" }} onClick={async () => copy(await finalPrompt())}>
              Copy
            </button>
          </div>
          <p className="muted" style={{ fontSize: 12, marginTop: 8 }}>
            Web tools open with the prompt on your clipboard. CLI tools (Claude
            Code, Codex, …) open in a new terminal already running with your
            compressed prompt.
          </p>

          {outcome && (
            <div className={`note ${outcome.launched ? "" : "warn-note"}`} style={{ marginTop: 12 }}>
              {outcome.message}
              {outcome.command && (
                <pre className="diff" style={{ whiteSpace: "pre-wrap", marginTop: 8 }}>
                  {outcome.command}
                </pre>
              )}
              {outcome.secrets && outcome.secrets.length > 0 && (
                <div style={{ marginTop: 6 }}>Detected: {outcome.secrets.join(", ")}</div>
              )}
            </div>
          )}

          {includeRules && (
            <>
              <div className="section-title">Response rules (attached)</div>
              <pre className="diff" style={{ whiteSpace: "pre-wrap" }}>
                {result.response_rules}
              </pre>
            </>
          )}

          {result.preserved_constraints.length > 0 && (
            <>
              <div className="section-title">Preserved constraints</div>
              <ul className="check-list">
                {result.preserved_constraints.map((c, i) => (
                  <li key={i}>✓ {c}</li>
                ))}
              </ul>
            </>
          )}

          <div className="section-title">Removed redundancy</div>
          <ul className="check-list">
            {result.removed_redundancy.map((r, i) => (
              <li key={i} className="muted">
                – {r}
              </li>
            ))}
          </ul>
        </>
      )}

      <div className="section-title">Output budget</div>
      <p className="muted">Attached to the launched prompt as response-size guidance.</p>
      <div className="mode-row">
        {BUDGETS.map(([val, label]) => (
          <label key={val} className={`mode-card ${budgetPreset === val ? "active" : ""}`}>
            <input type="radio" name="budget" checked={budgetPreset === val} onChange={() => setBudgetPreset(val)} />
            {label}
          </label>
        ))}
      </div>
    </div>
  );
}
