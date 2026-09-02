import type { RunSummary, CommandRecord } from "../api";
import type { NormalizedEvent, Signal } from "../data";
import { ToneIcon } from "./ui";

// ---------------------------------------------------------------------------
// WAITING_FOR_APPROVAL detail (§18/§19). An approval is never just "Approval
// required" — it must answer WHY / WHAT / WHO / CONTEXT / IMPACT / ACTION with
// concrete evidence. Every field is derived from real data; anything the
// backend did not provide is shown as "not reported", never invented.
// ---------------------------------------------------------------------------

export interface ApprovalRequest {
  id: string;
  what: string;
  why: string;
  who: string;
  context: string;
  impact: string;
  action: string;
  evidence: string | null;
  severity: string;
}

/** Derive pending-approval requests from whatever the backend actually gave us.
 * Order of preference: dedicated signals → normalized events → the legacy
 * command guard rows (which the recovery audit confirms are real today). */
export function buildApprovals(
  run: RunSummary | null,
  events: NormalizedEvent[],
  signals: Signal[],
  commands: CommandRecord[]
): ApprovalRequest[] {
  const out: ApprovalRequest[] = [];
  const ctx = run ? `${run.project_name} · ${run.agent_name ?? "unattributed agent"}` : "—";

  for (const s of signals) {
    if (s.kind !== "approval_required") continue;
    const e = s.explanation;
    out.push({
      id: s.id,
      what: e?.what || "An action requires approval.",
      why: e?.why || `Detector ${s.algorithm_id || "?"} flagged this as requiring approval.`,
      who: run?.agent_name ?? "unattributed agent",
      context: ctx,
      impact: e?.impact || "not reported",
      action: e?.action || "Review the evidence and approve or reject in your agent.",
      evidence: e?.evidence ?? null,
      severity: String(s.severity),
    });
  }

  for (const ev of events) {
    if (ev.status !== "pending_approval") continue;
    out.push({
      id: ev.id,
      what: ev.target ? `${ev.kind}: ${ev.target}` : ev.kind,
      why: "This step was paused pending a human decision.",
      who: ev.actor || run?.agent_name || "unattributed agent",
      context: ctx,
      impact: `Risk assessed as ${ev.risk || "unknown"}.`,
      action: "Approve or reject this step in your agent; Trace records the outcome.",
      evidence: ev.target,
      severity: String(ev.risk),
    });
  }

  for (const c of commands) {
    if (c.decision !== "require_approval") continue;
    out.push({
      id: c.id,
      what: "A guarded command wants to run.",
      why: "Trace's command guard classified this command as require_approval.",
      who: run?.agent_name ?? "unattributed agent",
      context: ctx,
      impact:
        "If run, this command performs an action the policy considers high-consequence (e.g. destructive or infrastructure-level).",
      action:
        "Approve only if you intended this. Note: at the agent boundary this is advisory today — verify your agent actually paused.",
      evidence: c.command,
      severity: "high",
    });
  }

  return out;
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="v4-approval-row">
      <div className="v4-approval-label">{label}</div>
      <div className="v4-approval-val">{children}</div>
    </div>
  );
}

export function ApprovalPanel({ req }: { req: ApprovalRequest }) {
  return (
    <div className="v4-approval" role="group" aria-label="Approval request">
      <div className="v4-approval-head">
        <ToneIcon tone="attention" size={18} />
        <b>Waiting for approval</b>
        <span className={`v4-tag tone-${req.severity.toLowerCase() === "high" || req.severity.toLowerCase() === "critical" ? "danger" : "attention"}`}>
          {req.severity.toUpperCase()}
        </span>
      </div>
      <Row label="What">{req.what}</Row>
      <Row label="Why">{req.why}</Row>
      <Row label="Who">{req.who}</Row>
      <Row label="Context">{req.context}</Row>
      <Row label="Impact">{req.impact}</Row>
      {req.evidence && (
        <Row label="Evidence">
          <code className="v4-approval-evidence mono">{req.evidence}</code>
        </Row>
      )}
      <Row label="Action">{req.action}</Row>
    </div>
  );
}
