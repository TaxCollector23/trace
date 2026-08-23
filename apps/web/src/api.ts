// Typed client for the local Trace daemon API. All paths are relative so
// the same code works behind the Vite dev proxy and when served by the daemon.

export type RunStatus =
  | "running"
  | "completed"
  | "failed"
  | "blocked"
  | "rolled_back";

export interface Project {
  id: string;
  name: string;
  path: string;
  config_path: string;
  created_at: string;
  updated_at: string;
}

export interface Run {
  id: string;
  project_id: string;
  command: string;
  agent_name: string | null;
  user_prompt: string | null;
  started_at: string;
  ended_at: string | null;
  starting_commit: string | null;
  ending_commit: string | null;
  status: RunStatus;
  exit_code: number | null;
  created_at: string;
}

export interface RunSummary extends Run {
  project_name: string;
  files_changed: number;
  command_count: number;
  secret_warnings: number;
  estimated_cost: number | null;
  checks_status: string | null;
}

export interface TimelineEvent {
  id: string;
  run_id: string;
  type: string;
  message: string;
  metadata_json: string | null;
  created_at: string;
}

export interface FileChange {
  id: string;
  run_id: string;
  path: string;
  change_type: "created" | "modified" | "deleted" | "renamed";
  diff_summary: string | null;
  created_at: string;
}

export interface CommandRecord {
  id: string;
  run_id: string;
  command: string;
  decision: string;
  exit_code: number | null;
  stdout_path: string | null;
  stderr_path: string | null;
  created_at: string;
}

export interface SecretRecord {
  id: string;
  run_id: string;
  file_path: string | null;
  secret_type: string;
  redacted_value: string;
  action_taken: string;
  created_at: string;
}

export interface ApiUsage {
  id: string;
  run_id: string;
  provider: string;
  model: string;
  input_tokens: number | null;
  output_tokens: number | null;
  cached_tokens: number | null;
  estimated_cost: number | null;
  latency_ms: number | null;
  created_at: string;
}

export interface CostResponse {
  usage: ApiUsage[];
  total_estimated: number | null;
  has_unavailable: boolean;
}

export interface Checkpoint {
  id: string;
  run_id: string;
  project_id: string;
  git_ref: string | null;
  checkpoint_type: string;
  created_at: string;
}

export interface TestResult {
  id: string;
  run_id: string;
  command: string;
  status: string;
  output_summary: string | null;
  created_at: string;
}

export interface DashboardData {
  runs: RunSummary[];
  projects: Project[];
}

export interface CompressionInfo {
  original_bytes: number;
  compressed_bytes: number;
}

export interface DiffResponse {
  diff: string;
  /** Present when the diff was stored compressed; null otherwise. */
  compression: CompressionInfo | null;
}

export interface GithubRepoInfo {
  full_name: string;
  private: boolean;
  default_branch: string;
  description: string | null;
  html_url: string;
  stargazers_count: number;
  open_issues_count: number;
}

export interface GithubStatus {
  authenticated: boolean;
  token_source: string;
  login: string | null;
  repo: GithubRepoInfo | null;
  repo_ref: { owner: string; repo: string } | null;
  error: string | null;
}

export interface GithubCommit {
  sha: string;
  message: string;
  author: string;
  date: string;
}

export interface GithubPull {
  number: number;
  title: string;
  state: string;
  user: string;
  html_url: string;
}

export interface AgentTokenStats {
  agent_name: string;
  run_count: number;
  input_tokens: number;
  output_tokens: number;
  estimated_cost: number;
}

export interface AnalyticsSummary {
  total_runs: number;
  first_run_at: string | null;
  avg_per_hour: number | null;
  avg_per_day: number | null;
  avg_per_week: number | null;
  avg_per_month: number | null;
  by_agent: AgentTokenStats[];
}

// --- Deterministic policy engine review -----------------------------------

export type Severity = "low" | "medium" | "high";
export type Decision = "allow" | "warn" | "require_approval" | "block";

export interface PolicyFindingRecord {
  id: string;
  run_id: string;
  rule_key: string;
  title: string;
  description: string;
  file_path: string | null;
  severity: Severity;
  confidence: number;
  source: string;
  created_at: string;
}

/** A policy finding as produced fresh by the engine (before it's stored). */
export interface PolicyFinding {
  rule_key: string;
  title: string;
  description: string;
  file_path: string | null;
  severity: Severity;
  confidence: number;
  source: string;
}

export interface AnalyzeRunResponse {
  policy_findings: PolicyFinding[];
  judge_verdict: null;
  agent_instruction: null;
}

// --- Ratify (deterministic policy review of a GitHub pull request) ---------

export type RatifyVerdict = "pass" | "review" | "block";

export interface RatifyReport {
  pr: number;
  files_reviewed: number;
  findings: PolicyFinding[];
  counts: { high: number; medium: number; low: number };
  verdict: RatifyVerdict;
}

// --- Benchmarks --------------------------------------------------------

export interface FixtureResult {
  name: string;
  expected_rule: string | null;
  fired_rules: string[];
  passed: boolean;
}

export interface PolicyEvalReport {
  total: number;
  passed: number;
  precision: number;
  recall: number;
  results: FixtureResult[];
}

export interface RedTeamEngineScore {
  name: string;
  threats: number;
  caught: number;
  downgraded: number;
  missed: number;
  benign: number;
  false_positives: number;
  recall: number;
}

export interface RedTeamReport {
  engines: RedTeamEngineScore[];
  pack_version: string;
  injection_phrases: number;
  command_rules: number;
  secret_patterns: number;
  passed: boolean;
}

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`/api${path}`);
  if (!res.ok) throw new Error(`GET ${path} failed: ${res.status}`);
  return res.json() as Promise<T>;
}

async function post<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`/api${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`POST ${path} failed: ${res.status}`);
  return res.json() as Promise<T>;
}

export const api = {
  dashboard: () => get<DashboardData>("/dashboard"),
  diff: (id: string) => get<DiffResponse>(`/runs/${id}/diff`),
  githubStatus: (projectId: string) =>
    get<GithubStatus>(`/github/status?project_id=${projectId}`),
  githubCommits: (projectId: string, limit = 20) =>
    get<GithubCommit[]>(`/github/commits?project_id=${projectId}&limit=${limit}`),
  githubPulls: (projectId: string) =>
    get<GithubPull[]>(`/github/pulls?project_id=${projectId}`),
  githubFile: (projectId: string, path: string, ref?: string) =>
    get<{ path: string; content: string }>(
      `/github/file?project_id=${projectId}&path=${encodeURIComponent(path)}${
        ref ? `&ref=${encodeURIComponent(ref)}` : ""
      }`
    ),
  state: () => get<Record<string, unknown>>("/state"),
  runs: () => get<RunSummary[]>("/runs"),
  run: (id: string) => get<RunSummary>(`/runs/${id}`),
  timeline: (id: string) => get<TimelineEvent[]>(`/runs/${id}/timeline`),
  fileChanges: (id: string) => get<FileChange[]>(`/runs/${id}/file-changes`),
  commands: (id: string) => get<CommandRecord[]>(`/runs/${id}/commands`),
  secrets: (id: string) => get<SecretRecord[]>(`/runs/${id}/secrets`),
  cost: (id: string) => get<CostResponse>(`/runs/${id}/cost`),
  checkpoints: (id: string) => get<Checkpoint[]>(`/runs/${id}/checkpoints`),
  testResults: (id: string) => get<TestResult[]>(`/runs/${id}/test-results`),
  rollback: (id: string) => post<{ ok: boolean; git_ref: string }>(`/runs/${id}/rollback`, {}),
  analytics: () => get<AnalyticsSummary>("/analytics"),
  // Deterministic policy engine review (no API key)
  policyFindings: (id: string) => get<PolicyFindingRecord[]>(`/runs/${id}/policy`),
  analyzeRun: (id: string) => post<AnalyzeRunResponse>(`/runs/${id}/analyze`, {}),
  // Ratify: deterministic policy review of a connected repo's pull request
  ratifyPull: (projectId: string, pr: number) =>
    get<RatifyReport>(`/github/ratify?project_id=${projectId}&pr=${pr}`),
  benchmarks: () => get<PolicyEvalReport>("/benchmarks"),
  redteamBenchmarks: () => get<RedTeamReport>("/benchmarks/redteam"),
};
