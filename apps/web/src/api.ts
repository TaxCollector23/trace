// Typed client for the local TraceGuard daemon API. All paths are relative so
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

export interface CompressionResult {
  mode: string;
  original: string;
  compressed: string;
  original_tokens: number;
  compressed_tokens: number;
  reduction_pct: number;
  estimated: boolean;
  preserved_constraints: string[];
  removed_redundancy: string[];
  conflicts: string[];
  response_rules: string;
}

export interface CompressionRecord {
  id: string;
  run_id: string | null;
  project_id: string | null;
  mode: string;
  original_token_estimate: number;
  compressed_token_estimate: number;
  estimated_reduction_percent: number;
  compressed_prompt_hash: string;
  original_prompt_stored: boolean;
  compressed_prompt_stored: boolean;
  original_prompt: string | null;
  compressed_prompt: string | null;
  created_at: string;
}

export interface AgentStatus {
  id: string;
  name: string;
  surface: string;
  category: string;
  installed: boolean;
  version: string | null;
  url: string | null;
  install_hint: string | null;
}

export interface LaunchOutcome {
  method: string;
  agent?: string;
  launched: boolean;
  copied?: boolean;
  url?: string | null;
  command?: string | null;
  message: string;
  secrets?: string[];
}

export const api = {
  dashboard: () => get<DashboardData>("/dashboard"),
  diff: (id: string) => get<{ diff: string }>(`/runs/${id}/diff`),
  agents: () => get<AgentStatus[]>("/agents"),
  launch: (target: string, prompt: string) =>
    post<LaunchOutcome>("/launch", { target, prompt }),
  compressPrompt: (prompt: string, mode: string) =>
    post<CompressionResult>("/prompt-compressor/compress", { prompt, mode }),
  outputBudget: (preset: string) =>
    post<{ preset: string; instruction_block: string }>(
      "/prompt-compressor/output-budget",
      { preset }
    ),
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
  compressionHistory: () =>
    get<CompressionRecord[]>("/prompt-compressor/history"),
  deleteCompression: (id: string) =>
    fetch(`/api/prompt-compressor/${id}`, { method: "DELETE" }).then((r) =>
      r.json()
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
};
