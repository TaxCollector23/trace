// ---------------------------------------------------------------------------
// Trace v4 typed data layer (prompt §111).
//
// A single place that talks to the daemon's read endpoints. Every fetch returns
// a discriminated `Resource<T>` — it NEVER throws and NEVER fabricates data. A
// missing endpoint, a 404, malformed JSON, or an unreachable daemon each map to
// an explicit, honest state the UI can render truthfully:
//
//   ok               – real data arrived and validated
//   empty            – the endpoint answered, but there is nothing to show
//   not_instrumented – this data source is not wired up in the backend yet
//   unavailable      – daemon unreachable / server error / malformed payload
//
// Several endpoints in the v4 contract (events-as-NormalizedEvent, signals,
// incidents, integrations/coverage) are being built in parallel. This layer is
// written so that the app degrades to an honest "unavailable"/"not instrumented"
// panel the moment any of them 404s or changes shape — a partial backend can
// never crash the dashboard and can never make it invent numbers.
// ---------------------------------------------------------------------------

// --- v4 contract types -----------------------------------------------------

export type Severity = "low" | "medium" | "high";
export type RiskLevel = "none" | "low" | "medium" | "high" | "critical";
export type Decision = "allow" | "warn" | "require_approval" | "block";

export type EventStatus =
  | "started"
  | "running"
  | "ok"
  | "warn"
  | "blocked"
  | "failed"
  | "pending_approval"
  | "unknown";

/** GET /api/runs/:id/events → NormalizedEvent[] */
export interface NormalizedEvent {
  id: string;
  run_id: string;
  parent_id?: string | null;
  ts_start: string;
  ts_end?: string | null;
  kind: string;
  actor: string;
  source: string;
  status: EventStatus | string;
  risk: RiskLevel | string;
  target: string | null;
  evidence: unknown;
  metadata: Record<string, unknown> | null;
}

/** GET /api/runs/:id/signals → Signal[] */
export interface SignalExplanation {
  what: string;
  why: string;
  evidence: string;
  impact: string;
  action: string;
}

export interface Signal {
  id: string;
  run_id: string;
  kind: string;
  severity: Severity | string;
  confidence: number;
  algorithm_id: string;
  algorithm_version: string;
  evidence_event_ids: string[];
  explanation: SignalExplanation;
  observed: string | number | null;
  baseline: string | number | null;
  deviation: string | number | null;
  data_window: string | null;
}

/** GET /api/runs/:id/incidents → Incident[] */
export interface Incident {
  id: string;
  run_id: string;
  severity: Severity | string;
  status: string;
  title: string;
  summary: string;
  signal_ids: string[];
  first_seen: string;
  last_seen: string;
}

/** GET /api/health */
export interface Health {
  status: string;
  service?: string;
  version?: string;
}

/** GET /api/integrations/coverage */
export interface IntegrationCoverage {
  agent: string;
  command_enforcement: boolean | null;
  file_review: boolean | null;
  status: string;
  note?: string | null;
}

// --- Resource envelope -----------------------------------------------------

export type ResourceKind = "disconnected" | "server" | "corrupt" | "not_found";

export type Resource<T> =
  | { state: "loading" }
  | { state: "ok"; data: T }
  | { state: "empty" }
  | { state: "not_instrumented"; reason: string }
  | { state: "unavailable"; kind: ResourceKind; reason: string };

export const LOADING: Resource<never> = { state: "loading" };

/** True when the resource has resolved (success or a definitive failure). */
export function isSettled<T>(r: Resource<T>): boolean {
  return r.state !== "loading";
}

/** Narrow to the payload when present, else null — for callers that only care
 * about the happy path but must still handle absence without fabricating. */
export function dataOf<T>(r: Resource<T>): T | null {
  return r.state === "ok" ? r.data : null;
}

// --- Defensive fetch -------------------------------------------------------

/** How to interpret a 404 for this endpoint. A run-scoped resource that 404s
 * means the run is gone (`not_found`); a whole endpoint that 404s means the
 * backend has not shipped it yet (`not_instrumented`). */
type AbsentMeaning = "not_instrumented" | "not_found";

export interface FetchOpts<T> {
  /** Optional runtime shape check. Returning false marks the payload corrupt. */
  validate?: (raw: unknown) => raw is T;
  /** Treat an empty array / null as the `empty` state. Default true. */
  emptyIsEmpty?: boolean;
  absent?: AbsentMeaning;
  signal?: AbortSignal;
}

function looksEmpty(v: unknown): boolean {
  if (v == null) return true;
  if (Array.isArray(v)) return v.length === 0;
  return false;
}

export async function fetchResource<T>(
  path: string,
  opts: FetchOpts<T> = {}
): Promise<Resource<T>> {
  const { validate, emptyIsEmpty = true, absent = "not_instrumented", signal } = opts;
  let res: Response;
  try {
    res = await fetch(`/api${path}`, { signal });
  } catch (e) {
    if ((e as { name?: string })?.name === "AbortError") {
      return { state: "loading" };
    }
    // Network-level failure: the daemon is not answering on 127.0.0.1.
    return {
      state: "unavailable",
      kind: "disconnected",
      reason: "The Trace daemon is not reachable on this machine.",
    };
  }

  if (res.status === 404) {
    if (absent === "not_found") {
      return { state: "unavailable", kind: "not_found", reason: "Not found." };
    }
    return {
      state: "not_instrumented",
      reason: "This data source is not instrumented by the daemon yet.",
    };
  }
  if (res.status === 501) {
    return {
      state: "not_instrumented",
      reason: "This data source is not implemented by the daemon yet.",
    };
  }
  if (res.status >= 500) {
    return {
      state: "unavailable",
      kind: "server",
      reason: `The daemon returned a server error (${res.status}). Its database may be unavailable.`,
    };
  }
  if (!res.ok) {
    return {
      state: "unavailable",
      kind: "server",
      reason: `The daemon returned an unexpected status (${res.status}).`,
    };
  }

  let raw: unknown;
  try {
    raw = await res.json();
  } catch {
    return {
      state: "unavailable",
      kind: "corrupt",
      reason: "The daemon returned a response that could not be parsed as JSON.",
    };
  }

  if (validate && !validate(raw)) {
    return {
      state: "unavailable",
      kind: "corrupt",
      reason: "The daemon returned data in an unexpected shape.",
    };
  }

  if (emptyIsEmpty && looksEmpty(raw)) {
    return { state: "empty" };
  }

  return { state: "ok", data: raw as T };
}

// --- Validators (kept intentionally lenient: we accept unknown extra fields
//     and only reject on structural surprises, so a backend that adds fields
//     never trips the "corrupt" state) --------------------------------------

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

const isArrayOfObjects = (v: unknown): v is Record<string, unknown>[] =>
  Array.isArray(v) && v.every((x) => x === null || isObject(x));

export const validators = {
  events: (v: unknown): v is NormalizedEvent[] =>
    isArrayOfObjects(v) &&
    (v as Record<string, unknown>[]).every(
      (e) => typeof e.id === "string" || typeof e.ts_start === "string"
    ),
  signals: (v: unknown): v is Signal[] => isArrayOfObjects(v),
  incidents: (v: unknown): v is Incident[] => isArrayOfObjects(v),
  coverage: (v: unknown): v is IntegrationCoverage[] => isArrayOfObjects(v),
  health: (v: unknown): v is Health => isObject(v) && typeof v.status === "string",
};

// --- v4 endpoint accessors -------------------------------------------------

export const v4 = {
  health: (signal?: AbortSignal) =>
    fetchResource<Health>("/health", {
      validate: validators.health,
      emptyIsEmpty: false,
      signal,
    }),

  events: (runId: string, signal?: AbortSignal) =>
    fetchResource<NormalizedEvent[]>(`/runs/${runId}/events`, {
      validate: validators.events,
      absent: "not_found",
      signal,
    }),

  signals: (runId: string, signal?: AbortSignal) =>
    fetchResource<Signal[]>(`/runs/${runId}/signals`, {
      validate: validators.signals,
      signal,
    }),

  incidents: (runId: string, signal?: AbortSignal) =>
    fetchResource<Incident[]>(`/runs/${runId}/incidents`, {
      validate: validators.incidents,
      signal,
    }),

  coverage: (signal?: AbortSignal) =>
    fetchResource<IntegrationCoverage[]>("/integrations/coverage", {
      validate: validators.coverage,
      signal,
    }),
};
