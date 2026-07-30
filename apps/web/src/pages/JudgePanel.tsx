import { useEffect, useState } from "react";
import { api, type JudgeSettings, type ProviderSlot } from "../api";
import { Loading, fmtTime, stagger, useAsync } from "../components";

const PROVIDER_LABELS: Record<string, string> = {
  anthropic: "Anthropic",
  openai: "OpenAI",
  google: "Google",
};

export default function JudgePanel() {
  const verdictsQ = useAsync(() => api.recentJudge(50));
  const verdicts = verdictsQ.data ?? [];

  return (
    <div>
      <h1 className="page-title">Judge Panel</h1>
      <p className="page-sub">
        Every judgment beyond Trace's rule-based guard goes through three
        independent models that vote separately and reason together. The
        judge can only add caution on top of the deterministic rules — it
        can never talk a blocked action down to "allow."
      </p>

      <JudgeSettingsPanel />
      <DoctrinePanel />

      <div className="section-title" style={{ marginTop: 30 }}>
        Recent judgments
      </div>
      {verdictsQ.loading ? (
        <Loading error={verdictsQ.error} variant="cards" rows={3} />
      ) : verdicts.length === 0 ? (
        <div className="empty">
          No judgments yet — these appear once the judge is enabled above and
          an agent run triggers analysis.
        </div>
      ) : (
        <div className="verdict-list">
          {verdicts.map((v, i) => (
            <div key={v.id} className="card enter" style={stagger(i)}>
              <div className="run-head">
                <span className={`pill ${v.consensus}`}>{v.consensus.replace("_", " ")}</span>
                <span className="muted">{fmtTime(v.created_at)}</span>
              </div>
              <p style={{ margin: "10px 0" }}>{v.summary}</p>
              <div className="run-meta">
                <span>
                  agreement: <b>{Math.round(v.agreement * 100)}%</b>
                </span>
                <span>
                  confidence: <b>{Math.round(v.confidence * 100)}%</b>
                </span>
                <span>
                  {v.action_taken === "agent_prompted" ? (
                    <span className="pill require_approval">sent to agent</span>
                  ) : (
                    <span className="pill">flagged only</span>
                  )}
                </span>
              </div>
              {v.votes.length > 0 && (
                <div className="vote-grid">
                  {v.votes.map((vote) => (
                    <div key={vote.id} className="vote-card">
                      <div className="run-head">
                        <b>{PROVIDER_LABELS[vote.provider] ?? vote.provider}</b>
                        {vote.error ? (
                          <span className="pill block">failed</span>
                        ) : (
                          <span className={`pill ${vote.decision}`}>{vote.decision.replace("_", " ")}</span>
                        )}
                      </div>
                      <div className="muted" style={{ fontSize: 12 }}>{vote.model}</div>
                      <p style={{ margin: "8px 0 0", fontSize: 13 }}>
                        {vote.error ? vote.error : vote.reasoning}
                      </p>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

const DEFAULT_SLOTS: ProviderSlot[] = [
  { provider: "anthropic", model: "claude-sonnet-5", base_url: null, api_key: "" },
  { provider: "openai", model: "gpt-5.1", base_url: null, api_key: "" },
  { provider: "google", model: "gemini-2.5-pro", base_url: null, api_key: "" },
];

function SlotCard({
  slot,
  isBuiltIn,
  onChange,
  onRemove,
  canRemove,
}: {
  slot: ProviderSlot;
  isBuiltIn: boolean;
  onChange: (next: ProviderSlot) => void;
  onRemove: () => void;
  canRemove: boolean;
}) {
  const [testing, setTesting] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; message: string } | null>(null);

  async function test() {
    setTesting(true);
    setResult(null);
    try {
      const r = await api.testJudgeSlot(slot);
      setResult(r);
    } catch (e) {
      setResult({ ok: false, message: String(e) });
    } finally {
      setTesting(false);
    }
  }

  return (
    <div className="card" style={{ marginBottom: 10 }}>
      <div className="field-row">
        <label>Provider id</label>
        <input
          value={slot.provider}
          placeholder="anthropic, google, openai, deepseek, xai, groq…"
          onChange={(e) => onChange({ ...slot, provider: e.target.value })}
        />
      </div>
      <div className="field-row">
        <label>Model</label>
        <input value={slot.model} onChange={(e) => onChange({ ...slot, model: e.target.value })} />
      </div>
      {!isBuiltIn && (
        <div className="field-row">
          <label>API base URL</label>
          <input
            value={slot.base_url ?? ""}
            placeholder="https://api.openai.com/v1/chat/completions (default)"
            onChange={(e) => onChange({ ...slot, base_url: e.target.value || null })}
          />
        </div>
      )}
      <div className="field-row">
        <label>API key</label>
        <input
          type="password"
          placeholder={slot.api_key ? "•••••••• (set — leave blank to keep)" : "API key"}
          onChange={(e) => onChange({ ...slot, api_key: e.target.value || null })}
        />
        <button className="btn-primary" style={{ background: "var(--bg-elev-2)", color: "var(--text)", marginTop: 0 }} onClick={test} disabled={testing}>
          {testing ? "Testing…" : "Test"}
        </button>
        {canRemove && (
          <button className="btn-primary" style={{ background: "var(--red)", marginTop: 0 }} onClick={onRemove}>
            Remove
          </button>
        )}
      </div>
      {result && (
        <p className="note" style={{ borderColor: result.ok ? undefined : "var(--red)", marginTop: 4 }}>
          {result.ok ? "✓ " : "✗ "}
          {result.message}
        </p>
      )}
    </div>
  );
}

function JudgeSettingsPanel() {
  const cfgQ = useAsync(() => api.judgeConfig());
  const [settings, setSettings] = useState<JudgeSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (cfgQ.data) setSettings(cfgQ.data.judge);
  }, [cfgQ.data]);

  if (cfgQ.loading || !settings) {
    return <Loading error={cfgQ.error} variant="cards" rows={1} />;
  }

  const slots = settings.slots.length > 0 ? settings.slots : DEFAULT_SLOTS;

  async function save() {
    if (!settings) return;
    setSaving(true);
    setSaved(false);
    try {
      const result = await api.saveJudgeConfig(settings);
      setSettings(result.judge);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="card">
      <div className="section-title" style={{ marginTop: 0 }}>
        Judge settings
      </div>

      <div className="field-row">
        <label>Mode</label>
        <select
          value={settings.mode}
          onChange={(e) => setSettings({ ...settings, mode: e.target.value as JudgeSettings["mode"] })}
        >
          <option value="disabled">Disabled — deterministic checks only</option>
          <option value="own_keys">Own keys — call providers directly from this machine</option>
          <option value="backend_proxy">Backend proxy — Trace-hosted, metered</option>
        </select>
      </div>

      {settings.mode === "own_keys" && (
        <>
          <p className="note">
            Keys are stored in <code>~/.trace/global.toml</code> or read from{" "}
            <code>TRACE_&lt;PROVIDER&gt;_API_KEY</code> environment variables (e.g.{" "}
            <code>TRACE_ANTHROPIC_API_KEY</code>, <code>TRACE_DEEPSEEK_API_KEY</code>). They
            are never sent anywhere except directly to that provider, and never
            bundled into the app itself.
          </p>
          {slots.map((slot, i) => {
            const isBuiltIn = slot.provider === "anthropic" || slot.provider === "google";
            return (
              <SlotCard
                key={i}
                slot={slot}
                isBuiltIn={isBuiltIn}
                canRemove={slots.length > 1}
                onChange={(next) => {
                  const updated = [...slots];
                  updated[i] = next;
                  setSettings({ ...settings, slots: updated });
                }}
                onRemove={() => setSettings({ ...settings, slots: slots.filter((_, j) => j !== i) })}
              />
            );
          })}
          <button
            className="btn-primary"
            style={{ background: "var(--bg-elev-2)", color: "var(--text)" }}
            onClick={() =>
              setSettings({
                ...settings,
                slots: [...slots, { provider: "", model: "", base_url: null, api_key: null }],
              })
            }
          >
            + Add a model to the panel
          </button>
          <p className="note" style={{ marginTop: 10 }}>
            Three independent models is the recommended baseline (so no single
            lab's blind spot decides alone), but the panel works with any
            number ≥ 1 — add more for a stronger vote, or point one slot at a
            model you're specifically evaluating.
          </p>
        </>
      )}

      {settings.mode === "backend_proxy" && (
        <div className="field-row">
          <label>Proxy URL</label>
          <input
            value={settings.backend_proxy_url ?? ""}
            onChange={(e) => setSettings({ ...settings, backend_proxy_url: e.target.value })}
            placeholder="https://api.trace.dev/judge"
          />
        </div>
      )}

      <div className="field-row">
        <label>
          <input
            type="checkbox"
            checked={settings.model_prompting_mode}
            onChange={(e) => setSettings({ ...settings, model_prompting_mode: e.target.checked })}
          />{" "}
          Model Prompting Mode
        </label>
      </div>
      <p className="note">
        {settings.model_prompting_mode
          ? "On — when the panel requires approval or blocks something, Trace sends a corrective instruction back to the coding agent asking it to stop and fix the issue."
          : "Off — questionable actions are only recorded here and on the dashboard. The rollback path stays available, same as today, but nothing is sent to the agent."}
      </p>

      <button className="btn-primary" onClick={save} disabled={saving}>
        {saving ? "Saving…" : saved ? "Saved" : "Save settings"}
      </button>
    </div>
  );
}

function DoctrinePanel() {
  const dashQ = useAsync(() => api.dashboard());
  const projects = dashQ.data?.projects ?? [];
  const [projectId, setProjectId] = useState<string>("");
  const [mining, setMining] = useState(false);
  const [mineNote, setMineNote] = useState<string | null>(null);

  useEffect(() => {
    if (!projectId && projects.length > 0) setProjectId(projects[0].id);
  }, [projects, projectId]);

  const rulesQ = useAsync(() => (projectId ? api.projectDoctrine(projectId) : Promise.resolve([])), [projectId]);
  const rules = rulesQ.data ?? [];

  async function mine() {
    if (!projectId) return;
    setMining(true);
    setMineNote(null);
    try {
      const result = await api.mineDoctrine(projectId);
      if (result.reason) {
        setMineNote(result.reason);
      } else {
        setMineNote(`Mined ${result.rules.length} rule(s) from ${result.prs_analyzed} merged PR(s).`);
      }
      rulesQ.reload();
    } finally {
      setMining(false);
    }
  }

  return (
    <div className="card" style={{ marginTop: 16 }}>
      <div className="section-title" style={{ marginTop: 0 }}>
        Doctrine
      </div>
      <p className="note" style={{ marginBottom: 14 }}>
        Rules mined from this project's own merged pull-request review
        comments — recurring things reviewers actually enforce. When present,
        the judge panel weighs violations of these more heavily than generic
        best practices.
      </p>

      {projects.length === 0 ? (
        <div className="empty">No projects registered yet.</div>
      ) : (
        <>
          <div className="field-row">
            <label>Project</label>
            <select value={projectId} onChange={(e) => setProjectId(e.target.value)}>
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
            <button className="btn-primary" onClick={mine} disabled={mining}>
              {mining ? "Mining…" : "Mine doctrine"}
            </button>
          </div>
          {mineNote && <p className="note">{mineNote}</p>}

          {rulesQ.loading ? (
            <Loading error={rulesQ.error} variant="table" rows={2} />
          ) : rules.length === 0 ? (
            <div className="empty">
              No doctrine mined yet for this project — click "Mine doctrine"
              (needs a GitHub remote, a readable token, and at least one
              judge provider configured above).
            </div>
          ) : (
            <table>
              <thead>
                <tr>
                  <th>Rule</th>
                  <th>Category</th>
                  <th>Strength</th>
                  <th>Confidence</th>
                </tr>
              </thead>
              <tbody>
                {rules.map((r, i) => (
                  <tr key={r.id} className="enter" style={stagger(i, 15, 160)}>
                    <td>{r.rule_text}</td>
                    <td className="muted">{r.category}</td>
                    <td>
                      <span className={`pill ${r.strength === "hard-rule" ? "block" : r.strength === "soft-norm" ? "require_approval" : "allow"}`}>
                        {r.strength.replace("-", " ")}
                      </span>
                    </td>
                    <td>{Math.round(r.confidence * 100)}%</td>
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
