import { useState } from "react";
import type { CompressionResult } from "../api";
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
  const [budgetBlock, setBudgetBlock] = useState("");

  async function compress() {
    if (!input.trim()) return;
    setBusy(true);
    setError(null);
    try {
      setResult(await api.compressPrompt(input, mode));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function buildBudget() {
    const res = await api.outputBudget(budgetPreset);
    setBudgetBlock(res.instruction_block);
  }

  function copy(text: string) {
    navigator.clipboard?.writeText(text);
  }

  function useInNextRun() {
    if (!result) return;
    // The dashboard cannot launch a terminal run; give the exact command and
    // copy the compressed prompt so the user can paste it.
    copy(result.compressed);
    alert(
      "Compressed prompt copied.\n\nRun it with:\n  trg run --compress --mode " +
        result.mode +
        ' "<agent> <your prompt>"\n\nor paste the copied prompt into your agent.'
    );
  }

  return (
    <div>
      <h1 className="page-title">Prompt Compressor</h1>
      <p className="page-sub">
        TraceCompress shrinks prompts and enforces output discipline. It runs
        locally and deterministically — nothing is sent to any model, and nothing
        is sent to your agent automatically. Token counts are estimates.
      </p>

      {/* 1. Prompt input */}
      <div className="section-title">Prompt</div>
      <textarea
        className="ta"
        placeholder="Paste your prompt…"
        value={input}
        onChange={(e) => setInput(e.target.value)}
        rows={7}
      />

      {/* 2. Mode selector */}
      <div className="section-title">Mode</div>
      <div className="mode-row">
        {MODES.map(([val, label]) => (
          <label key={val} className={`mode-card ${mode === val ? "active" : ""}`}>
            <input
              type="radio"
              name="mode"
              checked={mode === val}
              onChange={() => setMode(val)}
            />
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
          {/* Conflicts first — do not hide them */}
          {result.conflicts.length > 0 && (
            <div className="note warn-note" style={{ marginTop: 16 }}>
              <b>Possible conflicting instructions</b> — resolve before relying on
              this compression:
              <ul style={{ margin: "6px 0 0" }}>
                {result.conflicts.map((c, i) => (
                  <li key={i}>{c}</li>
                ))}
              </ul>
            </div>
          )}

          {/* 4. Token estimate */}
          <div className="note" style={{ marginTop: 14 }}>
            Estimated tokens (approximate): <b>{result.original_tokens}</b> →{" "}
            <b>{result.compressed_tokens}</b> (~
            {result.reduction_pct.toFixed(0)}% reduction) · mode{" "}
            <b>{result.mode}</b>
          </div>

          {/* 3. Compressed output */}
          <div className="section-title">Compressed prompt</div>
          <pre className="diff" style={{ whiteSpace: "pre-wrap" }}>
            {result.compressed}
          </pre>
          <div className="btn-row">
            <button className="btn" onClick={() => copy(result.compressed)}>
              Copy prompt
            </button>
            <button className="btn" onClick={useInNextRun}>
              Use in next run
            </button>
          </div>

          {/* 7. Response rules block */}
          <div className="section-title">Response rules (output discipline)</div>
          <pre className="diff" style={{ whiteSpace: "pre-wrap" }}>
            {result.response_rules}
          </pre>
          <button className="btn" onClick={() => copy(result.response_rules)}>
            Copy response rules
          </button>

          {/* 5. Preserved constraints */}
          <div className="section-title">Preserved constraints</div>
          {result.preserved_constraints.length === 0 ? (
            <div className="muted">No explicit constraints detected.</div>
          ) : (
            <ul className="check-list">
              {result.preserved_constraints.map((c, i) => (
                <li key={i}>✓ {c}</li>
              ))}
            </ul>
          )}

          {/* 6. Removed redundancy */}
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

      {/* Output budget */}
      <div className="section-title">Output budget</div>
      <p className="muted">
        Generate a response-size guidance block. Guidance only, unless your tool
        supports hard output limits.
      </p>
      <div className="mode-row">
        {BUDGETS.map(([val, label]) => (
          <label
            key={val}
            className={`mode-card ${budgetPreset === val ? "active" : ""}`}
          >
            <input
              type="radio"
              name="budget"
              checked={budgetPreset === val}
              onChange={() => setBudgetPreset(val)}
            />
            {label}
          </label>
        ))}
      </div>
      <div style={{ marginTop: 10 }}>
        <button className="btn" onClick={buildBudget}>
          Build block
        </button>
      </div>
      {budgetBlock && (
        <>
          <pre className="diff" style={{ whiteSpace: "pre-wrap", marginTop: 12 }}>
            {budgetBlock}
          </pre>
          <button className="btn" onClick={() => copy(budgetBlock)}>
            Copy budget block
          </button>
        </>
      )}
    </div>
  );
}
