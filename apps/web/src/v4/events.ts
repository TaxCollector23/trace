import type { NormalizedEvent } from "../data";

// The /runs/:id/events endpoint is transitioning from the legacy timeline shape
// to NormalizedEvent (built in parallel). This normalizes EITHER shape into one
// display model so the Run Page timeline works during the migration without
// crashing and without inventing fields.

export interface DisplayEvent {
  id: string;
  ts: string | null;
  kind: string;
  actor: string | null;
  status: string | null;
  risk: string | null;
  target: string | null;
  message: string;
}

interface LegacyEvent {
  id?: string;
  type?: string;
  message?: string;
  created_at?: string;
}

export function toDisplayEvents(raw: NormalizedEvent[]): DisplayEvent[] {
  return raw.map((e, i) => {
    const legacy = e as unknown as LegacyEvent;
    const ts = e.ts_start ?? legacy.created_at ?? null;
    const kind = e.kind ?? legacy.type ?? "event";
    return {
      id: e.id ?? `${i}`,
      ts,
      kind,
      actor: e.actor ?? null,
      status: (e.status as string) ?? null,
      risk: (e.risk as string) ?? null,
      target: e.target ?? null,
      message: legacy.message ?? e.target ?? kind,
    };
  });
}
