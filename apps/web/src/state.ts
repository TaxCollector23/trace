// ---------------------------------------------------------------------------
// Dashboard state machine (prompt §2).
//
// The dashboard is modelled as ONE explicit state, not a pile of components.
// The active state decides the whole layout: what the primary visual is, what
// the one obvious action is, what to announce to a screen reader, and which
// data is expected to be present vs unavailable. The Run Page and Control Room
// read `describe(state)` and adapt — the user never hunts for the current run.
// ---------------------------------------------------------------------------

import type { RunSummary } from "./api";
import type { Resource, Signal, Incident, NormalizedEvent, Health } from "./data";

export type DashboardState =
  | "IDLE"
  | "STARTING"
  | "RUNNING"
  | "WAITING_FOR_APPROVAL"
  | "BLOCKED"
  | "FAILED"
  | "COMPLETED"
  | "ROLLED_BACK"
  | "DISCONNECTED"
  | "NO_DATA"
  | "CORRUPT_DATA";

export type Tone = "neutral" | "info" | "active" | "attention" | "danger" | "success" | "muted";

export interface Action {
  label: string;
  /** A route to navigate to, or a well-known command id handled by the shell. */
  to?: string;
  command?: string;
  tone?: "primary" | "default" | "danger";
}

export interface StateDescriptor {
  state: DashboardState;
  /** Short human label shown in the status strip. */
  label: string;
  tone: Tone;
  /** One-line explanation of *why* the dashboard is in this state (§19). */
  explanation: string;
  /** The single most useful thing to do next. */
  primaryAction: Action | null;
  secondaryActions: Action[];
  /** Which data panels are meaningful in this state. */
  availableData: string[];
  unavailableData: string[];
  /** Which layout the Run Page / Control Room should render. */
  layout: "recent" | "live" | "intervention" | "failure" | "outcome" | "system" | "onboarding";
  /** Polite (default) vs assertive live-region announcement. */
  announce: { text: string; assertive: boolean };
  /** Keyboard hint surfaced in the status strip / help. */
  keyboard: string;
}

const ICON_TONE: Record<DashboardState, Tone> = {
  IDLE: "muted",
  STARTING: "info",
  RUNNING: "active",
  WAITING_FOR_APPROVAL: "attention",
  BLOCKED: "danger",
  FAILED: "danger",
  COMPLETED: "success",
  ROLLED_BACK: "info",
  DISCONNECTED: "danger",
  NO_DATA: "muted",
  CORRUPT_DATA: "danger",
};

// --- Inputs the machine reasons over -------------------------------------

export interface StateInputs {
  health: Resource<Health>;
  /** All runs the daemon knows about (may be empty / unavailable). */
  runs: Resource<RunSummary[]>;
  /** The run currently in focus, if one is selected/derivable. */
  run: RunSummary | null;
  events: Resource<NormalizedEvent[]>;
  signals: Resource<Signal[]>;
  incidents: Resource<Incident[]>;
}

/** A run is "active" if the backend still considers it running. Per the
 * recovery audit the daemon coerces unknown states to `running`, so RUNNING is
 * treated as provisional and cross-checked against event freshness. */
function isTerminal(status: string): boolean {
  return ["completed", "failed", "blocked", "rolled_back", "aborted", "interrupted"].includes(
    status
  );
}

function hasPendingApproval(
  events: Resource<NormalizedEvent[]>,
  signals: Resource<Signal[]>,
  incidents: Resource<Incident[]>
): boolean {
  if (signals.state === "ok" && signals.data.some((s) => s.kind === "approval_required"))
    return true;
  if (
    incidents.state === "ok" &&
    incidents.data.some((i) => i.status === "awaiting_approval" || i.status === "needs_approval")
  )
    return true;
  if (
    events.state === "ok" &&
    events.data.some(
      (e) => e.status === "pending_approval" || e.risk === "critical" && e.status === "warn"
    )
  )
    return true;
  return false;
}

function hasBlockingIncident(incidents: Resource<Incident[]>): boolean {
  return (
    incidents.state === "ok" &&
    incidents.data.some(
      (i) => i.status === "open" && (i.severity === "high" || i.severity === "critical")
    )
  );
}

/** Derive the single dashboard state from all available inputs. This function
 * is total: every combination resolves to exactly one state, and missing data
 * degrades to an honest state rather than an optimistic guess. */
export function deriveState(inp: StateInputs): DashboardState {
  // 1. Can we talk to the daemon at all?
  if (inp.health.state === "unavailable" && inp.health.kind === "disconnected") {
    return "DISCONNECTED";
  }

  // 2. Do we have a usable run list?
  if (inp.runs.state === "unavailable") {
    return inp.runs.kind === "corrupt" ? "CORRUPT_DATA" : "DISCONNECTED";
  }
  if (inp.runs.state === "empty") {
    return "NO_DATA";
  }

  const run = inp.run;

  // 3. If a specific run is in focus, its own data may be corrupt.
  if (run) {
    if (inp.events.state === "unavailable" && inp.events.kind === "corrupt") {
      return "CORRUPT_DATA";
    }

    if (!isTerminal(run.status)) {
      // The run is (provisionally) active. Prioritise intervention states.
      if (hasBlockingIncident(inp.incidents)) return "BLOCKED";
      if (hasPendingApproval(inp.events, inp.signals, inp.incidents))
        return "WAITING_FOR_APPROVAL";
      // Just started and no events yet → STARTING.
      const noEventsYet =
        inp.events.state === "empty" ||
        (inp.events.state === "ok" && inp.events.data.length === 0);
      if (noEventsYet) return "STARTING";
      return "RUNNING";
    }

    // Terminal run → outcome-oriented state.
    switch (run.status) {
      case "completed":
        return "COMPLETED";
      case "failed":
        return "FAILED";
      case "blocked":
        return "BLOCKED";
      case "rolled_back":
        return "ROLLED_BACK";
      default:
        return "COMPLETED";
    }
  }

  // 4. Runs exist but none is in focus / active → idle control room.
  if (inp.runs.state === "ok") {
    const active = inp.runs.data.find((r) => !isTerminal(r.status));
    if (active) return "RUNNING";
    return "IDLE";
  }

  return "IDLE";
}

// --- Descriptors -----------------------------------------------------------

const A = (label: string, extra: Partial<Action> = {}): Action => ({ label, ...extra });

const DESCRIPTORS: Record<DashboardState, Omit<StateDescriptor, "tone">> = {
  IDLE: {
    state: "IDLE",
    label: "Idle",
    explanation: "No run is active. Showing the most recent activity across your projects.",
    primaryAction: A("Watch for the next run", { command: "focus-latest", tone: "primary" }),
    secondaryActions: [A("Open command palette", { command: "palette" })],
    availableData: ["recent runs", "integration coverage", "outcomes"],
    unavailableData: ["live timeline", "in-flight risk signals"],
    layout: "recent",
    announce: { text: "Idle. No active run.", assertive: false },
    keyboard: "j / k move between recent runs · Enter opens · ⌘K palette",
  },
  STARTING: {
    state: "STARTING",
    label: "Starting",
    explanation: "A run has begun but has not emitted events yet. Waiting for the first signal.",
    primaryAction: A("Follow live", { command: "focus-latest", tone: "primary" }),
    secondaryActions: [A("View run header", { command: "focus-latest" })],
    availableData: ["run header", "command"],
    unavailableData: ["timeline (no events yet)", "risk signals", "diff", "tests"],
    layout: "live",
    announce: { text: "Run starting.", assertive: false },
    keyboard: "Esc leaves the run · ⌘K palette",
  },
  RUNNING: {
    state: "RUNNING",
    label: "Running",
    explanation: "A run is in progress. The timeline and risk signals update live.",
    primaryAction: A("Follow live", { command: "focus-latest", tone: "primary" }),
    secondaryActions: [A("Pause auto-refresh", { command: "toggle-live" })],
    availableData: ["live timeline", "risk signals", "commands", "files"],
    unavailableData: ["final diff", "final outcome"],
    layout: "live",
    announce: { text: "Run in progress.", assertive: false },
    keyboard: "l live toggle · g t timeline · ⌘K palette",
  },
  WAITING_FOR_APPROVAL: {
    state: "WAITING_FOR_APPROVAL",
    label: "Waiting for approval",
    explanation:
      "The run reached an action that needs a human decision. See who, why, and the impact before deciding.",
    primaryAction: A("Review approval request", { command: "focus-approval", tone: "primary" }),
    secondaryActions: [
      A("View evidence", { command: "focus-approval" }),
      A("Open run controls", { command: "focus-controls" }),
    ],
    availableData: ["approval request", "evidence events", "risk signals", "timeline"],
    unavailableData: ["final outcome", "final diff"],
    layout: "intervention",
    announce: {
      text: "A run is waiting for your approval.",
      assertive: true,
    },
    keyboard: "a jump to approval · e evidence · ⌘K palette",
  },
  BLOCKED: {
    state: "BLOCKED",
    label: "Blocked",
    explanation:
      "Trace blocked a dangerous action. Review what was stopped and why before continuing.",
    primaryAction: A("Review what was blocked", { command: "focus-controls", tone: "primary" }),
    secondaryActions: [A("View risk signals", { command: "focus-risk" })],
    availableData: ["blocked command", "risk signals", "incidents", "timeline"],
    unavailableData: ["completed diff"],
    layout: "intervention",
    announce: { text: "A run is blocked.", assertive: true },
    keyboard: "r risk · Esc leaves run · ⌘K palette",
  },
  FAILED: {
    state: "FAILED",
    label: "Failed",
    explanation: "The run ended in failure. The failure chain shows what led to the exit.",
    primaryAction: A("View failure chain", { command: "focus-failure", tone: "primary" }),
    secondaryActions: [
      A("Roll back", { command: "focus-controls", tone: "danger" }),
      A("View diff", { command: "focus-diff" }),
    ],
    availableData: ["failure chain", "commands", "diff", "risk signals"],
    unavailableData: ["live timeline (run ended)"],
    layout: "failure",
    announce: { text: "A run failed.", assertive: true },
    keyboard: "f failure chain · d diff · ⌘K palette",
  },
  COMPLETED: {
    state: "COMPLETED",
    label: "Completed",
    explanation: "The run finished. Review the outcome and exactly what changed.",
    primaryAction: A("Review outcome & diff", { command: "focus-diff", tone: "primary" }),
    secondaryActions: [
      A("View timeline", { command: "focus-timeline" }),
      A("Roll back", { command: "focus-controls" }),
    ],
    availableData: ["outcome", "diff", "files", "tests", "timeline"],
    unavailableData: ["live updates (run ended)"],
    layout: "outcome",
    announce: { text: "Run completed.", assertive: false },
    keyboard: "d diff · g t timeline · ⌘K palette",
  },
  ROLLED_BACK: {
    state: "ROLLED_BACK",
    label: "Rolled back",
    explanation: "This run's changes were rolled back. The timeline shows the restore point used.",
    primaryAction: A("View what was reverted", { command: "focus-diff", tone: "primary" }),
    secondaryActions: [A("View timeline", { command: "focus-timeline" })],
    availableData: ["restore point", "reverted diff", "timeline"],
    unavailableData: ["live updates (run ended)"],
    layout: "outcome",
    announce: { text: "Run rolled back.", assertive: false },
    keyboard: "d diff · g t timeline · ⌘K palette",
  },
  DISCONNECTED: {
    state: "DISCONNECTED",
    label: "Daemon unreachable",
    explanation: "The Trace daemon is not answering on 127.0.0.1. No live data can be shown.",
    primaryAction: A("Retry connection", { command: "reload", tone: "primary" }),
    secondaryActions: [A("How to start the daemon", { command: "help-daemon" })],
    availableData: [],
    unavailableData: ["everything (no daemon connection)"],
    layout: "system",
    announce: { text: "The Trace daemon is unreachable.", assertive: true },
    keyboard: "Enter retries connection",
  },
  NO_DATA: {
    state: "NO_DATA",
    label: "No runs yet",
    explanation: "Trace is connected but has recorded no runs. Connect an agent and run a task.",
    primaryAction: A("Connect an agent", { command: "help-onboarding", tone: "primary" }),
    secondaryActions: [A("Open command palette", { command: "palette" })],
    availableData: ["setup guidance", "integration coverage"],
    unavailableData: ["runs", "timeline", "risk", "diff"],
    layout: "onboarding",
    announce: { text: "No runs recorded yet.", assertive: false },
    keyboard: "⌘K palette",
  },
  CORRUPT_DATA: {
    state: "CORRUPT_DATA",
    label: "Data unreadable",
    explanation:
      "The daemon returned data in an unexpected shape. Rather than guess, Trace is not rendering it.",
    primaryAction: A("Retry", { command: "reload", tone: "primary" }),
    secondaryActions: [A("Pick another run", { command: "palette" })],
    availableData: ["run list (if readable)"],
    unavailableData: ["the malformed resource"],
    layout: "system",
    announce: { text: "A run's data could not be read.", assertive: true },
    keyboard: "Enter retries",
  },
};

export function describe(state: DashboardState): StateDescriptor {
  return { ...DESCRIPTORS[state], tone: ICON_TONE[state] };
}
