import { api, type PromptPattern } from "../api";
import { Loading, fmtTime, stagger, useAsync } from "../components";

const PATTERN_LABELS: Record<PromptPattern, string> = {
  too_short: "Too short",
  vague: "Vague",
  open_ended: "Open-ended / hedging",
  conflicting: "Conflicting constraints",
  no_acceptance_criteria: "No acceptance criteria",
  well_scoped: "Well-scoped",
};

const PATTERN_ADVICE: Record<PromptPattern, string> = {
  too_short:
    "Give the agent more to work with: what file or feature, what the current behavior is, and what you want instead.",
  vague:
    'Replace placeholders like "fix it" or "handle this" with the specific symptom and the specific file/function.',
  open_ended:
    "Pick one approach yourself, or explicitly ask the agent to propose options before implementing — don't leave it to guess which you meant.",
  conflicting: "Two parts of this prompt appear to pull in different directions — reread it for contradicting constraints.",
  no_acceptance_criteria:
    "State how you'll know it worked: a test that should pass, a behavior to verify, or an example input/output.",
  well_scoped: "Good — this prompt names concrete files/functions, which sharply narrows what the agent has to guess.",
};

const GOOD_PATTERN: PromptPattern = "well_scoped";

function parsePatterns(json: string): PromptPattern[] {
  try {
    return JSON.parse(json) as PromptPattern[];
  } catch {
    return [];
  }
}

export default function PromptCoach() {
  const q = useAsync(() => api.recentPrompts(200));
  const events = q.data ?? [];

  const withPatterns = events.map((e) => ({ ...e, patterns: parsePatterns(e.patterns_json) }));

  const patternCounts: Partial<Record<PromptPattern, number>> = {};
  for (const e of withPatterns) {
    for (const p of e.patterns) {
      patternCounts[p] = (patternCounts[p] ?? 0) + 1;
    }
  }

  const avgClarity =
    events.length > 0 ? Math.round(events.reduce((s, e) => s + e.clarity_score, 0) / events.length) : null;

  const habitsToImprove = (Object.entries(patternCounts) as [PromptPattern, number][])
    .filter(([p]) => p !== GOOD_PATTERN)
    .sort((a, b) => b[1] - a[1]);

  return (
    <div>
      <h1 className="page-title">Prompting Coach</h1>
      <p className="page-sub">
        Every prompt you send to an agent is scored locally for clarity. This
        surfaces the habits worth fixing — nothing here is sent anywhere.
      </p>

      {q.loading ? (
        <Loading error={q.error} variant="kpis" />
      ) : events.length === 0 ? (
        <div className="empty">
          No prompts recorded yet — this fills in as you run agents through
          Trace.
        </div>
      ) : (
        <>
          <div className="kpis">
            <div className="kpi">
              <div className="k-val">{avgClarity ?? "—"}</div>
              <div className="k-label">Avg clarity score</div>
            </div>
            <div className="kpi">
              <div className="k-val">{events.length}</div>
              <div className="k-label">Prompts analyzed</div>
            </div>
            <div className="kpi">
              <div className="k-val">{patternCounts.well_scoped ?? 0}</div>
              <div className="k-label">Well-scoped prompts</div>
            </div>
            <div className="kpi">
              <div className="k-val">{habitsToImprove.reduce((s, [, c]) => s + c, 0)}</div>
              <div className="k-label">Improvable prompts</div>
            </div>
          </div>

          <div className="section-title" style={{ marginTop: 30 }}>
            Habits to work on
          </div>
          {habitsToImprove.length === 0 ? (
            <div className="empty">No recurring issues detected — nice work.</div>
          ) : (
            <div className="habit-list">
              {habitsToImprove.map(([pattern, count]) => (
                <div key={pattern} className="card">
                  <div className="run-head">
                    <b>{PATTERN_LABELS[pattern]}</b>
                    <span className="muted">{count} prompt{count === 1 ? "" : "s"}</span>
                  </div>
                  <p style={{ margin: "8px 0 0" }}>{PATTERN_ADVICE[pattern]}</p>
                </div>
              ))}
            </div>
          )}

          <div className="section-title" style={{ marginTop: 30 }}>
            Recent prompts
          </div>
          <table>
            <thead>
              <tr>
                <th>Prompt</th>
                <th>Clarity</th>
                <th>Patterns</th>
                <th>When</th>
              </tr>
            </thead>
            <tbody>
              {withPatterns.slice(0, 30).map((e, i) => (
                <tr key={e.id} className="enter" style={stagger(i, 15, 160)}>
                  <td className="mono" style={{ maxWidth: 360, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {e.prompt_text}
                  </td>
                  <td>{Math.round(e.clarity_score)}</td>
                  <td>
                    {e.patterns.map((p) => (
                      <span key={p} className={`pill ${p === GOOD_PATTERN ? "allow" : "warn"}`} style={{ marginRight: 4 }}>
                        {PATTERN_LABELS[p]}
                      </span>
                    ))}
                  </td>
                  <td className="muted">{fmtTime(e.created_at)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
    </div>
  );
}
