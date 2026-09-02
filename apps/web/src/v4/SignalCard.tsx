import { useState } from "react";
import type { Signal } from "../data";
import { RiskTag, ToneIcon } from "./ui";

// ---------------------------------------------------------------------------
// Attention prioritizer card (§18) + "Why am I seeing this?" (§19).
//
// Every intelligent signal is INSPECTABLE: one click reveals the exact
// signal / observed / baseline / deviation / data-window / algorithm behind it.
// Nothing here is fabricated — a field the backend did not provide renders as
// "not reported", never as a plausible-looking number.
// ---------------------------------------------------------------------------

function fmtVal(v: string | number | null | undefined): string {
  if (v === null || v === undefined || v === "") return "not reported";
  return String(v);
}

/** Priority score for ordering (higher = more urgent). Deterministic and
 * derived only from real fields. */
export function signalPriority(s: Signal): number {
  const sev = { critical: 4, high: 3, medium: 2, low: 1 }[String(s.severity).toLowerCase()] ?? 0;
  const conf = Number.isFinite(s.confidence) ? s.confidence : 0.5;
  return sev * 10 + conf * 5;
}

export function SignalCard({ signal, onEvidence }: { signal: Signal; onEvidence?: (ids: string[]) => void }) {
  const [open, setOpen] = useState(false);
  const exp = signal.explanation ?? ({} as Signal["explanation"]);
  const title = exp?.what || signal.kind || "Signal";
  const confPct = Number.isFinite(signal.confidence)
    ? `${Math.round(signal.confidence * 100)}%`
    : "—";

  return (
    <div className={`v4-signal sev-${String(signal.severity).toLowerCase()}`}>
      <div className="v4-signal-head">
        <div className="v4-signal-title">
          <RiskTag level={String(signal.severity)} />
          <b>{title}</b>
        </div>
        <span className="muted v4-conf" title="Detector confidence">
          confidence {confPct}
        </span>
      </div>

      {exp?.why && <p className="v4-signal-why">{exp.why}</p>}

      <button
        type="button"
        className="v4-why-toggle"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        <ToneIcon tone="info" size={13} />
        {open ? "Hide the evidence" : "Why am I seeing this?"}
      </button>

      {open && (
        <div className="v4-why">
          <dl className="v4-why-grid">
            <dt>Signal</dt>
            <dd>{signal.kind || "not reported"}</dd>
            <dt>Observed</dt>
            <dd className="mono">{fmtVal(signal.observed)}</dd>
            <dt>Baseline</dt>
            <dd className="mono">{fmtVal(signal.baseline)}</dd>
            <dt>Deviation</dt>
            <dd className="mono">{fmtVal(signal.deviation)}</dd>
            <dt>Data window</dt>
            <dd>{fmtVal(signal.data_window)}</dd>
            <dt>Algorithm</dt>
            <dd className="mono">
              {signal.algorithm_id || "not reported"}
              {signal.algorithm_version ? ` @ ${signal.algorithm_version}` : ""}
            </dd>
          </dl>

          {exp?.evidence && (
            <div className="v4-why-block">
              <span className="v4-why-label">Evidence</span>
              <p>{exp.evidence}</p>
            </div>
          )}
          {exp?.impact && (
            <div className="v4-why-block">
              <span className="v4-why-label">Impact</span>
              <p>{exp.impact}</p>
            </div>
          )}
          {exp?.action && (
            <div className="v4-why-block">
              <span className="v4-why-label">Recommended action</span>
              <p>{exp.action}</p>
            </div>
          )}

          {signal.evidence_event_ids?.length > 0 && onEvidence && (
            <button className="btn-ghost" onClick={() => onEvidence(signal.evidence_event_ids)}>
              Show {signal.evidence_event_ids.length} evidence event
              {signal.evidence_event_ids.length === 1 ? "" : "s"} on the timeline
            </button>
          )}
        </div>
      )}
    </div>
  );
}
