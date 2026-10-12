import { fetchResource, type IntegrationCoverage, type Resource } from "./data";
import { isDevFixtureMode } from "./usage";

export type CapabilityStatus = "available" | "partial" | "not_available" | "unknown";

export interface CapabilityMatrix {
  tokens: CapabilityStatus;
  cost: CapabilityStatus;
  subagents: CapabilityStatus;
  tool_calls: CapabilityStatus;
  command_guard: CapabilityStatus;
}

export interface ConnectionRow {
  id: string;
  name: string;
  installed: boolean;
  path: string | null;
  version: string | null;
  connected: boolean;
  session_paths: string[];
  session_data_found: boolean;
  last_event_at: string | null;
  events_imported: number;
  parse_errors: number;
  capabilities: CapabilityMatrix;
  status: string;
  action: string;
  install_hint: string | null;
  note: string;
  importer: string | null;
}

export async function loadConnections(signal?: AbortSignal): Promise<Resource<ConnectionRow[]>> {
  if (isDevFixtureMode()) {
    try {
      const response = await fetch("/__trace-fixtures/connections", { signal });
      if (!response.ok) {
        return { state: "unavailable", kind: "server", reason: `Fixture server returned ${response.status}.` };
      }
      const rows = (await response.json()) as unknown;
      if (!Array.isArray(rows)) {
        return { state: "unavailable", kind: "corrupt", reason: "Fixture connections were not an array." };
      }
      return { state: "ok", data: rows.map((row) => normalizeFixtureRow(row as IntegrationCoverage)) };
    } catch (error) {
      if ((error as { name?: string })?.name === "AbortError") return { state: "loading" };
      return { state: "unavailable", kind: "server", reason: `Fixture data could not be loaded: ${String(error)}` };
    }
  }

  return fetchResource<ConnectionRow[]>("/connections", {
    validate: (value: unknown): value is ConnectionRow[] =>
      Array.isArray(value) && value.every(isConnectionRow),
    signal,
  });
}

function normalizeFixtureRow(row: IntegrationCoverage): ConnectionRow {
  const status = row.status ?? "unknown";
  return {
    id: row.agent,
    name: row.display_name ?? row.agent,
    installed: row.installed ?? false,
    path: null,
    version: null,
    connected: row.connected ?? false,
    session_paths: row.session_data_path ? [row.session_data_path] : [],
    session_data_found: Boolean(row.session_data_path),
    last_event_at: row.last_event_at ?? null,
    events_imported: row.events_imported ?? 0,
    parse_errors: row.parse_errors ?? 0,
    capabilities: {
      tokens: legacyCapability(row.capabilities?.tokens),
      cost: legacyCapability(row.capabilities?.cost),
      subagents: legacyCapability(row.capabilities?.subagents),
      tool_calls: legacyCapability(row.capabilities?.tool_calls),
      command_guard: legacyCapability(row.capabilities?.command_guard ?? row.command_enforcement),
    },
    status,
    action: row.note ?? "Synthetic development fixture",
    install_hint: null,
    note: row.note ?? "Synthetic development fixture",
    importer: null,
  };
}

function isConnectionRow(value: unknown): value is ConnectionRow {
  if (typeof value !== "object" || value === null) return false;
  const row = value as Partial<ConnectionRow>;
  return (
    typeof row.id === "string" &&
    typeof row.name === "string" &&
    typeof row.installed === "boolean" &&
    typeof row.connected === "boolean" &&
    Array.isArray(row.session_paths) &&
    typeof row.events_imported === "number" &&
    typeof row.parse_errors === "number" &&
    typeof row.action === "string" &&
    typeof row.capabilities === "object" &&
    row.capabilities !== null
  );
}

function legacyCapability(value: boolean | null | undefined): CapabilityStatus {
  if (value === true) return "available";
  if (value === false) return "not_available";
  return "unknown";
}
