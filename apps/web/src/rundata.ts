// Resource-envelope accessors for the run-scoped endpoints that already exist
// today (the legacy `api.*` helpers throw on error; these degrade honestly via
// the v4 data layer instead). Types are reused from api.ts.

import { fetchResource, type Resource } from "./data";
import type {
  RunSummary,
  FileChange,
  CommandRecord,
  SecretRecord,
  TestResult,
  Checkpoint,
  CostResponse,
  DiffResponse,
  PolicyFindingRecord,
} from "./api";

const runScoped = "not_found" as const;

export const runApi = {
  runs: (signal?: AbortSignal) =>
    fetchResource<RunSummary[]>("/runs", { signal, emptyIsEmpty: true }),
  run: (id: string, signal?: AbortSignal) =>
    fetchResource<RunSummary>(`/runs/${id}`, { emptyIsEmpty: false, absent: runScoped, signal }),
  files: (id: string, signal?: AbortSignal) =>
    fetchResource<FileChange[]>(`/runs/${id}/file-changes`, { absent: runScoped, signal }),
  commands: (id: string, signal?: AbortSignal) =>
    fetchResource<CommandRecord[]>(`/runs/${id}/commands`, { absent: runScoped, signal }),
  secrets: (id: string, signal?: AbortSignal) =>
    fetchResource<SecretRecord[]>(`/runs/${id}/secrets`, { absent: runScoped, signal }),
  tests: (id: string, signal?: AbortSignal) =>
    fetchResource<TestResult[]>(`/runs/${id}/test-results`, { absent: runScoped, signal }),
  checkpoints: (id: string, signal?: AbortSignal) =>
    fetchResource<Checkpoint[]>(`/runs/${id}/checkpoints`, { absent: runScoped, signal }),
  cost: (id: string, signal?: AbortSignal) =>
    fetchResource<CostResponse>(`/runs/${id}/cost`, { emptyIsEmpty: false, absent: runScoped, signal }),
  diff: (id: string, signal?: AbortSignal) =>
    fetchResource<DiffResponse>(`/runs/${id}/diff`, { emptyIsEmpty: false, absent: runScoped, signal }),
  policy: (id: string, signal?: AbortSignal) =>
    fetchResource<PolicyFindingRecord[]>(`/runs/${id}/policy`, { absent: runScoped, signal }),
};

export type { Resource };
