//! Local JSON API. All routes are mounted under `/api` and bound to 127.0.0.1.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::MutexGuard;
use trace_core::{git, models::*, Store};

use crate::state::AppState;

/// Build the `/api` router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/state", get(state_info))
        .route("/dashboard", get(dashboard))
        .route("/projects", get(list_projects).post(create_project))
        .route("/projects/:id", get(get_project))
        .route("/runs", get(list_runs).post(create_run))
        .route("/runs/:id", get(get_run))
        .route("/runs/:id/finish", post(finish_run))
        .route("/runs/:id/events", get(timeline).post(add_event))
        .route("/runs/:id/timeline", get(timeline))
        .route(
            "/runs/:id/file-changes",
            get(file_changes).post(set_file_changes),
        )
        .route("/runs/:id/diff", get(run_diff))
        .route("/runs/:id/commands", get(commands).post(add_command))
        .route("/runs/:id/secrets", get(secrets).post(add_secret))
        .route("/runs/:id/cost", get(cost).post(add_cost))
        .route(
            "/runs/:id/checkpoints",
            get(checkpoints).post(add_checkpoint),
        )
        .route(
            "/runs/:id/test-results",
            get(test_results).post(add_test_result),
        )
        .route("/runs/:id/rollback", post(rollback))
        .route("/check-command", post(check_command))
        .route("/scan", post(scan_project))
        .route("/doctor", get(doctor))
        .route("/analytics", get(analytics))
        .route("/benchmarks", get(benchmarks))
        .route("/benchmarks/redteam", get(redteam_benchmarks))
        // Deterministic policy-engine review (no API key required)
        .route("/runs/:id/analyze", post(analyze_run))
        .route("/runs/:id/hook-check", post(hook_check))
        .route("/runs/:id/policy", get(list_run_policy_findings))
        // GitHub (reads directly from the repo, including private)
        .route("/github/status", get(gh_status))
        .route("/github/commits", get(gh_commits))
        .route("/github/pulls", get(gh_pulls))
        .route("/github/file", get(gh_file))
        // Ratify: deterministic policy review of a connected repo's PR
        .route("/github/ratify", get(gh_ratify))
}

// --- Error handling -------------------------------------------------------

/// Wraps any error into a JSON 500/404 response.
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn not_found(what: &str) -> Self {
        ApiError {
            status: StatusCode::NOT_FOUND,
            message: format!("{what} not found"),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: e.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

type ApiResult<T> = Result<T, ApiError>;

/// Lock the store, recovering the guard if a previous handler panicked while
/// holding the lock. Poisoning means some earlier request died mid-critical
/// section; the SQLite connection itself is still usable, so we take the guard
/// back and let the daemon keep serving instead of wedging every future request.
fn store(state: &AppState) -> MutexGuard<'_, Store> {
    state.store.lock().unwrap_or_else(|e| e.into_inner())
}

// --- Health / state -------------------------------------------------------

async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "trace-daemon",
        "version": trace_core::VERSION,
    }))
}

async fn state_info(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    let s = store(&state);
    let projects = s.list_projects()?;
    Ok(Json(json!({
        "version": trace_core::VERSION,
        "port": state.port,
        "started_at": state.started_at,
        "db_path": state.db_path,
        "bind": "127.0.0.1",
        "active_projects": projects.len(),
        "projects": projects,
    })))
}

#[derive(Deserialize)]
struct LimitQuery {
    limit: Option<i64>,
}

async fn dashboard(
    State(state): State<AppState>,
    Query(q): Query<LimitQuery>,
) -> ApiResult<impl IntoResponse> {
    let s = store(&state);
    let summaries = s.recent_run_summaries(q.limit.unwrap_or(50))?;
    let projects = s.list_projects()?;
    Ok(Json(json!({ "runs": summaries, "projects": projects })))
}

// --- Projects -------------------------------------------------------------

async fn list_projects(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).list_projects()?))
}

async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    store(&state)
        .project_by_id(&id)?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("project"))
}

async fn create_project(
    State(state): State<AppState>,
    Json(body): Json<NewProject>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).upsert_project(&body)?))
}

// --- Runs -----------------------------------------------------------------

async fn list_runs(
    State(state): State<AppState>,
    Query(q): Query<LimitQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        store(&state).recent_run_summaries(q.limit.unwrap_or(50))?,
    ))
}

async fn get_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let s = store(&state);
    let run = s
        .run_by_id(&id)?
        .ok_or_else(|| ApiError::not_found("run"))?;
    let summary = s.run_summary(&run)?;
    Ok(Json(summary))
}

async fn create_run(
    State(state): State<AppState>,
    Json(body): Json<NewRun>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).create_run(&body)?))
}

#[derive(Deserialize)]
struct FinishBody {
    status: String,
    exit_code: Option<i64>,
    ending_commit: Option<String>,
}

async fn finish_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<FinishBody>,
) -> ApiResult<impl IntoResponse> {
    {
        let s = store(&state);
        s.finish_run(
            &id,
            RunStatus::from_str(&body.status),
            body.exit_code,
            body.ending_commit.as_deref(),
        )?;
    }
    // Opt-in cloud sync (fires only when TRACE_CLOUD_URL + TRACE_CLOUD_TOKEN
    // are set). Runs on its own thread; never blocks this response.
    crate::cloud_sync::enqueue(id.clone(), state.store.clone());
    Ok(Json(json!({ "ok": true })))
}

// --- Events / timeline ----------------------------------------------------

async fn add_event(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<NewEvent>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).add_event(&id, &body)?))
}

async fn timeline(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).list_events(&id)?))
}

// --- File changes ---------------------------------------------------------

async fn file_changes(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).list_file_changes(&id)?))
}

/// Serve the full unified diff captured for a run (read from the run log dir).
async fn run_diff(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let s = store(&state);
    let run = s
        .run_by_id(&id)?
        .ok_or_else(|| ApiError::not_found("run"))?;
    let project = s
        .project_by_id(&run.project_id)?
        .ok_or_else(|| ApiError::not_found("project"))?;
    let patch = std::path::Path::new(&project.path)
        .join(".trace")
        .join("runs")
        .join(&id)
        .join("diff.patch");
    let diff = std::fs::read_to_string(&patch).unwrap_or_default();
    Ok(Json(json!({ "diff": diff })))
}

async fn set_file_changes(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<Vec<NewFileChange>>,
) -> ApiResult<impl IntoResponse> {
    store(&state).replace_file_changes(&id, &body)?;
    Ok(Json(json!({ "ok": true, "count": body.len() })))
}

// --- Commands -------------------------------------------------------------

async fn commands(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).list_commands(&id)?))
}

async fn add_command(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<NewCommand>,
) -> ApiResult<impl IntoResponse> {
    store(&state).add_command(&id, &body)?;
    Ok(Json(json!({ "ok": true })))
}

// --- Secrets --------------------------------------------------------------

async fn secrets(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).list_secrets(&id)?))
}

async fn add_secret(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<NewSecret>,
) -> ApiResult<impl IntoResponse> {
    store(&state).add_secret(&id, &body)?;
    Ok(Json(json!({ "ok": true })))
}

// --- Cost / API usage -----------------------------------------------------

#[derive(Serialize)]
struct CostResponse {
    usage: Vec<ApiUsage>,
    total_estimated: Option<f64>,
    has_unavailable: bool,
}

async fn cost(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let usage = store(&state).list_api_usage(&id)?;
    let has_unavailable = usage.iter().any(|u| u.estimated_cost.is_none());
    let total: f64 = usage.iter().filter_map(|u| u.estimated_cost).sum();
    let total_estimated = if usage.iter().any(|u| u.estimated_cost.is_some()) {
        Some(total)
    } else {
        None
    };
    Ok(Json(CostResponse {
        usage,
        total_estimated,
        has_unavailable,
    }))
}

async fn add_cost(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<NewApiUsage>,
) -> ApiResult<impl IntoResponse> {
    store(&state).add_api_usage(&id, &body)?;
    Ok(Json(json!({ "ok": true })))
}

// --- Checkpoints ----------------------------------------------------------

async fn checkpoints(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).list_checkpoints(&id)?))
}

async fn add_checkpoint(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<NewCheckpoint>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).add_checkpoint(&id, &body)?))
}

// --- Test results ---------------------------------------------------------

async fn test_results(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).list_test_results(&id)?))
}

async fn add_test_result(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<NewTestResult>,
) -> ApiResult<impl IntoResponse> {
    store(&state).add_test_result(&id, &body)?;
    Ok(Json(json!({ "ok": true })))
}

// --- Rollback -------------------------------------------------------------

#[derive(Deserialize, Default)]
struct RollbackBody {
    /// Optional explicit checkpoint ref. Defaults to the run's latest checkpoint.
    git_ref: Option<String>,
}

async fn rollback(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<RollbackBody>>,
) -> ApiResult<impl IntoResponse> {
    let body = body.map(|Json(b)| b).unwrap_or_default();
    let s = store(&state);
    let run = s
        .run_by_id(&id)?
        .ok_or_else(|| ApiError::not_found("run"))?;
    let project = s
        .project_by_id(&run.project_id)?
        .ok_or_else(|| ApiError::not_found("project"))?;

    let git_ref = match body.git_ref {
        Some(r) => r,
        None => s
            .list_checkpoints(&id)?
            .into_iter()
            .rev()
            .find_map(|c| c.git_ref)
            .ok_or_else(|| ApiError::not_found("checkpoint with git ref"))?,
    };

    let project_path = std::path::PathBuf::from(&project.path);
    s.add_event(
        &id,
        &NewEvent {
            event_type: EventType::RollbackCreated.as_str().to_string(),
            message: format!("Rolling back to {git_ref}"),
            metadata_json: Some(json!({ "git_ref": git_ref }).to_string()),
        },
    )?;

    git::rollback_to(&project_path, &git_ref).map_err(ApiError::from)?;

    s.set_run_status(&id, RunStatus::RolledBack)?;
    s.add_event(
        &id,
        &NewEvent {
            event_type: EventType::RollbackCompleted.as_str().to_string(),
            message: "Rollback completed".to_string(),
            metadata_json: None,
        },
    )?;

    Ok(Json(json!({ "ok": true, "git_ref": git_ref })))
}

// --- Command guard (shared by Claude hooks, Cursor MCP, CI) ---------------

#[derive(Deserialize)]
struct CheckCommandBody {
    command: String,
}

/// Classify a command with the shared guard rules. Stateless; no storage.
async fn check_command(Json(body): Json<CheckCommandBody>) -> impl IntoResponse {
    let result = trace_core::guard::classify(&body.command);
    Json(json!({
        "decision": result.decision.as_str(),
        "reason": result.reason,
    }))
}

#[derive(Deserialize)]
struct ScanBody {
    /// Project directory to scan. Defaults to the daemon's working directory.
    #[serde(default)]
    path: Option<String>,
}

async fn scan_project(Json(body): Json<ScanBody>) -> impl IntoResponse {
    let root = body
        .path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    Json(trace_core::scan::scan(&root))
}

/// Lightweight doctor: version, agents detected, clipboard availability.
async fn doctor(State(state): State<AppState>) -> impl IntoResponse {
    let agents = trace_core::agents::detect_all();
    let installed = agents.iter().filter(|a| a.installed).count();
    let clipboard = trace_core::agents::copy_to_clipboard("").is_ok();
    Json(json!({
        "version": trace_core::VERSION,
        "port": state.port,
        "db_path": state.db_path,
        "clipboard": clipboard,
        "agents_total": agents.len(),
        "agents_installed": installed,
        "agents": agents,
    }))
}

/// Usage analytics across every run: run-frequency averages and per-agent
/// token totals. Pure aggregate SQL over the local database — nothing sent
/// anywhere.
async fn analytics(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).analytics_summary()?))
}

/// Runs the policy engine's own labeled-fixture benchmark fresh on every
/// call — it's pure computation over in-memory fixtures (no I/O), so
/// there's nothing to cache or go stale. See `trace-core::eval`.
async fn benchmarks() -> impl IntoResponse {
    Json(trace_core::run_policy_eval())
}

/// Runs the adversarial red-team detection benchmark fresh on every call:
/// dangerous commands (incl. evasions), planted secrets, and unsafe prompts
/// through the real guard / secret / prompt engines. Pure computation, no I/O.
/// See `trace-core::redteam`.
async fn redteam_benchmarks() -> impl IntoResponse {
    Json(trace_core::run_redteam_eval())
}

// --- Policy engine review -------------------------------------------------

/// Runs the deterministic policy engine over a run's recorded file changes,
/// then — if the judge is enabled — runs the 3-LLM consensus panel with
/// those findings as context. Persists both and returns a combined result.
///
/// When `model_prompting_mode` is on and the consensus lands on
/// `require_approval` or `block`, the response includes `agent_instruction`:
/// a ready-to-relay message the CLI adapter sends back to the coding agent
/// asking it to stop and address the issue. When it's off, the verdict is
/// only recorded (`action_taken: "flagged_only"`) and the existing
/// rollback path remains the way to undo the change.
/// `file_changes.change_type` is stored as `ChangeType::as_str()`
/// ("created"/"deleted") for display purposes, but `policy.rs`'s checks use
/// GitHub's vocabulary ("added"/"removed") — see `ChangeType::as_diff_status`.
fn normalize_change_status(s: &str) -> String {
    match s {
        "created" => "added".to_string(),
        "deleted" => "removed".to_string(),
        other => other.to_string(),
    }
}

/// Count added/removed lines in a unified diff patch, excluding the `+++`/`---`
/// file headers. Returns `(additions, deletions)`. Used to populate `FileDiff`
/// on the live-review paths so policy's large-file check has real counts to
/// work with instead of hardcoded zeros. `None`/empty patches yield `(0, 0)`.
fn count_diff_lines(patch: Option<&str>) -> (i64, i64) {
    let Some(patch) = patch else {
        return (0, 0);
    };
    let mut additions = 0i64;
    let mut deletions = 0i64;
    for line in patch.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if line.starts_with('+') {
            additions += 1;
        } else if line.starts_with('-') {
            deletions += 1;
        }
    }
    (additions, deletions)
}

async fn analyze_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let (run, file_changes, project_path) = {
        let s = store(&state);
        let run = s
            .run_by_id(&id)?
            .ok_or_else(|| ApiError::not_found("run"))?;
        let file_changes = s.list_file_changes(&id)?;
        let project_path = s.project_by_id(&run.project_id)?.map(|p| p.path);
        (run, file_changes, project_path)
    };

    // The policy engine's checks (secret scanning, TODO detection, etc.) are
    // regex matches against real added/removed lines — a "+12 -3" stat
    // summary has nothing for them to match. Pull the actual patch text per
    // file from git when we have a starting commit and a project path;
    // fall back to the stat summary only if that's genuinely unavailable
    // (e.g. the project isn't a git repo), so this still degrades instead
    // of erroring.
    let patches: std::collections::HashMap<String, String> =
        match (&project_path, &run.starting_commit) {
            (Some(path), Some(from_ref)) => {
                trace_core::git::patches_by_file(std::path::Path::new(path), from_ref)
                    .unwrap_or_default()
            }
            _ => Default::default(),
        };

    let diffs: Vec<trace_core::FileDiff> = file_changes
        .iter()
        .map(|f| {
            let patch = patches
                .get(&f.path)
                .cloned()
                .or_else(|| f.diff_summary.clone());
            let (additions, deletions) = count_diff_lines(patch.as_deref());
            trace_core::FileDiff {
                filename: f.path.clone(),
                status: normalize_change_status(&f.change_type),
                additions,
                deletions,
                patch,
            }
        })
        .collect();

    let policy_findings = trace_core::run_policy_checks(&diffs);
    store(&state).add_policy_findings(&id, &policy_findings)?;

    // Deterministic review only — the policy engine runs with no API key. The
    // `judge_verdict`/`agent_instruction` fields are retained as `null` for
    // wire-compatibility with older adapters that still read them.
    Ok(Json(json!({
        "policy_findings": policy_findings,
        "judge_verdict": null,
        "agent_instruction": null,
    })))
}

#[derive(Deserialize)]
struct HookCheckBody {
    /// Accepted for wire-compat with existing agent hooks; no longer used now
    /// that live review is deterministic-only.
    #[allow(dead_code)]
    tool_name: String,
    file_path: Option<String>,
    /// Best-effort diff/content snippet for the edit. Optional: some tools
    /// (e.g. Write with a full file body) may only give the file path.
    diff_summary: Option<String>,
}

#[derive(Serialize)]
struct HookCheckResponse {
    block: bool,
    /// Blocking message when `block == true`. Kept for backward compat
    /// with older hook shells that only read this field.
    message: Option<String>,
    policy_findings: usize,
    /// True when the judge would otherwise have run (Model Prompting Mode
    /// on, judge enabled) but was skipped because this run is still inside
    /// its cooldown window since the last judge call. Distinguishes "the
    /// panel looked and allowed it" from "the panel didn't look this time."
    judge_on_cooldown: bool,
    /// The single message the hook should echo *to the coding agent* — a
    /// concrete, actionable multi-line summary of what was flagged, why,
    /// and what the agent should try next. Populated for any decision
    /// beyond `allow` (including `warn`, which the hook echoes without
    /// exit 2). This is the field a new hook/MCP surface should read;
    /// `message` is a fallback for the older shell.
    agent_feedback: Option<String>,
    /// The consensus decision itself, so MCP-driven surfaces can render
    /// the same information with their own affordances instead of a raw
    /// string. Absent when the judge didn't run.
    #[serde(skip_serializing_if = "Option::is_none")]
    consensus: Option<String>,
}

/// The live guardrail entry point: called from the coding agent's own
/// PostToolUse hook (see `integrations/claude/trace-hook.sh`) right after a
/// file edit. Always runs the fast deterministic policy engine. Only spends
/// the latency of a 3-LLM judge call when Model Prompting Mode is on —
/// that's the whole trade the toggle represents: instant edits with
/// dashboard-only flagging, or a brief pause per edit in exchange for the
/// panel being able to tell the agent to stop and fix something before it
/// keeps building on a mistake.
async fn hook_check(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<HookCheckBody>,
) -> ApiResult<impl IntoResponse> {
    {
        let s = store(&state);
        s.run_by_id(&id)?
            .ok_or_else(|| ApiError::not_found("run"))?;
    }

    let (additions, deletions) = count_diff_lines(body.diff_summary.as_deref());
    let diff = trace_core::FileDiff {
        filename: body
            .file_path
            .clone()
            .unwrap_or_else(|| "(unknown file)".to_string()),
        status: "modified".to_string(),
        additions,
        deletions,
        patch: body.diff_summary.clone(),
    };
    let diffs = vec![diff];

    let policy_findings = trace_core::run_policy_checks(&diffs);
    if !policy_findings.is_empty() {
        store(&state).add_policy_findings(&id, &policy_findings)?;
    }

    let mut response = HookCheckResponse {
        block: false,
        message: None,
        policy_findings: policy_findings.len(),
        judge_on_cooldown: false,
        agent_feedback: None,
        consensus: None,
    };

    // Deterministic policy findings are echoed to the agent as advisory. This
    // is the whole live-review surface now: fast, local, and needs no API key.
    if !policy_findings.is_empty() {
        response.agent_feedback = Some(format_policy_advisory(&policy_findings));
    }

    Ok(Json(response))
}

/// Advisory formatting for the deterministic
/// policy engine found things worth telling the agent about. Kept short —
/// the agent will still be moving forward, so we prioritize signal density.
fn format_policy_advisory(findings: &[trace_core::PolicyFinding]) -> String {
    let mut out = String::from("[Trace] Deterministic checks:");
    for f in findings.iter().take(6) {
        out.push_str(&format!(
            "\n  • [{}] {}: {}",
            f.severity.as_str(),
            f.title,
            f.description
        ));
    }
    if findings.len() > 6 {
        out.push_str(&format!("\n  … and {} more", findings.len() - 6));
    }
    out
}

/// Formats a project's stored doctrine rules as prompt-ready lines, e.g.
/// `"[hard-rule · testing] Every new endpoint needs an integration test."`
/// Shared by `analyze_run` and `hook_check` so both live-review paths give
/// the judge panel the same repo-specific context.
async fn list_run_policy_findings(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).list_policy_findings(&id)?))
}

// --- GitHub ---------------------------------------------------------------

#[derive(Deserialize)]
struct GhQuery {
    project_id: String,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    r#ref: Option<String>,
    /// Pull-request number to ratify (used by `/github/ratify`).
    #[serde(default)]
    pr: Option<i64>,
}

/// Resolve a project's path + git RepoRef + token for GitHub calls.
fn gh_repo_ref(
    state: &AppState,
    project_id: &str,
) -> ApiResult<(
    std::path::PathBuf,
    trace_core::github::RepoRef,
    Option<String>,
)> {
    let project = store(state)
        .project_by_id(project_id)?
        .ok_or_else(|| ApiError::not_found("project"))?;
    let path = std::path::PathBuf::from(&project.path);
    let repo_ref = trace_core::git::remote_url(&path)
        .and_then(|u| trace_core::github::parse_remote(&u))
        .ok_or_else(|| ApiError {
            status: StatusCode::BAD_REQUEST,
            message: "project has no GitHub origin remote".into(),
        })?;
    let (token, _) = trace_core::github::resolve_token();
    Ok((path, repo_ref, token))
}

async fn gh_status(
    State(state): State<AppState>,
    Query(q): Query<GhQuery>,
) -> ApiResult<impl IntoResponse> {
    let project = store(&state)
        .project_by_id(&q.project_id)?
        .ok_or_else(|| ApiError::not_found("project"))?;
    let status = trace_core::github::status_for_path(std::path::Path::new(&project.path));
    Ok(Json(status))
}

async fn gh_commits(
    State(state): State<AppState>,
    Query(q): Query<GhQuery>,
) -> ApiResult<impl IntoResponse> {
    let (_p, r, token) = gh_repo_ref(&state, &q.project_id)?;
    let commits = trace_core::github::list_commits(&r, token.as_deref(), q.limit.unwrap_or(20))
        .map_err(ApiError::from)?;
    Ok(Json(commits))
}

async fn gh_pulls(
    State(state): State<AppState>,
    Query(q): Query<GhQuery>,
) -> ApiResult<impl IntoResponse> {
    let (_p, r, token) = gh_repo_ref(&state, &q.project_id)?;
    let pulls = trace_core::github::list_pulls(&r, token.as_deref()).map_err(ApiError::from)?;
    Ok(Json(pulls))
}

async fn gh_file(
    State(state): State<AppState>,
    Query(q): Query<GhQuery>,
) -> ApiResult<impl IntoResponse> {
    let path = q.path.clone().ok_or_else(|| ApiError {
        status: StatusCode::BAD_REQUEST,
        message: "missing ?path=".into(),
    })?;
    let (_p, r, token) = gh_repo_ref(&state, &q.project_id)?;
    let content = trace_core::github::get_file(&r, &path, q.r#ref.as_deref(), token.as_deref())
        .map_err(ApiError::from)?;
    Ok(Json(json!({ "path": path, "content": content })))
}

/// Ratify a pull request on the connected GitHub repo: fetch the PR's changed
/// files and run the same deterministic policy engine (secret scanning,
/// risky-change detection, etc.) used across Trace. No LLM, no API key — pure
/// pattern matching, so every user gets a working verdict. The verdict is
/// `block` if any high-severity finding, `review` for medium-only, else `pass`.
async fn gh_ratify(
    State(state): State<AppState>,
    Query(q): Query<GhQuery>,
) -> ApiResult<impl IntoResponse> {
    let pr = q.pr.ok_or_else(|| ApiError {
        status: StatusCode::BAD_REQUEST,
        message: "missing ?pr= (pull-request number)".into(),
    })?;
    let (_p, r, token) = gh_repo_ref(&state, &q.project_id)?;
    let files =
        trace_core::github::list_pr_files(&r, token.as_deref(), pr).map_err(ApiError::from)?;
    let findings = trace_core::run_policy_checks(&files);
    let summary = trace_core::ratify_summarize(&findings);

    Ok(Json(json!({
        "pr": pr,
        "files_reviewed": files.len(),
        "findings": findings,
        "counts": {
            "high": summary.counts.high,
            "medium": summary.counts.medium,
            "low": summary.counts.low,
        },
        "verdict": summary.verdict.as_str(),
    })))
}

// --- Tests ----------------------------------------------------------------
//
// These are the first daemon tests. They exercise pure/local handler logic
// against an in-memory store — no network, no git subprocess, no bound socket
// — so they stay deterministic and offline. Handlers reached here (hook_check,
// analyze_run) never touch GitHub or git as long as the run has no
// `starting_commit` (analyze_run only shells out to git when both a project
// path and a starting commit are present).
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use trace_core::Store;

    fn test_state() -> AppState {
        AppState {
            store: Arc::new(Mutex::new(Store::open_in_memory().unwrap())),
            port: 0,
            started_at: "1970-01-01T00:00:00Z".to_string(),
            db_path: ":memory:".to_string(),
        }
    }

    /// Insert a project + run and return the run id.
    fn seed_run(state: &AppState, starting_commit: Option<&str>) -> String {
        let s = store(state);
        let project = s
            .upsert_project(&NewProject {
                name: "T".into(),
                path: "/does/not/exist".into(),
                config_path: "/does/not/exist/c".into(),
            })
            .unwrap();
        let run = s
            .create_run(&NewRun {
                project_id: project.id,
                command: "run".into(),
                agent_name: None,
                user_prompt: None,
                starting_commit: starting_commit.map(|s| s.to_string()),
            })
            .unwrap();
        run.id
    }

    // A real AWS access-key-shaped secret, split so this source file itself
    // isn't flagged by scanners scanning the repo.
    fn planted_secret_patch() -> String {
        format!("+const key = \"{}\";", concat!("AKIA", "ABCDEFGHIJKLMNOP"))
    }

    #[test]
    fn normalize_change_status_maps_git_vocabulary() {
        assert_eq!(normalize_change_status("created"), "added");
        assert_eq!(normalize_change_status("deleted"), "removed");
        // Anything already in GitHub's vocabulary passes through unchanged.
        assert_eq!(normalize_change_status("modified"), "modified");
        assert_eq!(normalize_change_status("renamed"), "renamed");
    }

    #[test]
    fn count_diff_lines_counts_body_and_ignores_headers() {
        let patch = "--- a/f.rs\n+++ b/f.rs\n@@ -1,2 +1,3 @@\n+added one\n+added two\n-removed one\n unchanged";
        assert_eq!(count_diff_lines(Some(patch)), (2, 1));
        assert_eq!(count_diff_lines(None), (0, 0));
        assert_eq!(count_diff_lines(Some("")), (0, 0));
    }

    #[test]
    fn rollback_default_checkpoint_selection_prefers_latest_inserted() {
        // Two checkpoints inserted back-to-back (same RFC3339 second in
        // practice; the rowid tiebreak makes it deterministic regardless).
        // This mirrors the `rollback` handler's default selection expression.
        let state = test_state();
        let run_id = seed_run(&state, None);
        {
            let s = store(&state);
            let project_id = s.list_projects().unwrap()[0].id.clone();
            for git_ref in ["ref-old", "ref-new"] {
                s.add_checkpoint(
                    &run_id,
                    &NewCheckpoint {
                        project_id: project_id.clone(),
                        git_ref: Some(git_ref.to_string()),
                        checkpoint_type: "auto".into(),
                    },
                )
                .unwrap();
            }
        }

        let selected = store(&state)
            .list_checkpoints(&run_id)
            .unwrap()
            .into_iter()
            .rev()
            .find_map(|c| c.git_ref);
        assert_eq!(selected.as_deref(), Some("ref-new"));
    }

    #[tokio::test]
    async fn hook_check_persists_secret_finding() {
        let state = test_state();
        let run_id = seed_run(&state, None);

        let body = HookCheckBody {
            tool_name: "Edit".into(),
            file_path: Some("src/config.rs".into()),
            diff_summary: Some(planted_secret_patch()),
        };
        hook_check(State(state.clone()), Path(run_id.clone()), Json(body))
            .await
            .map_err(|e| e.message)
            .expect("hook_check should succeed offline");

        let findings = store(&state).list_policy_findings(&run_id).unwrap();
        assert!(
            findings.iter().any(|f| f.rule_key == "secret-in-diff"),
            "expected a secret-in-diff finding to be persisted, got: {findings:?}"
        );
    }

    #[tokio::test]
    async fn analyze_run_persists_secret_finding_from_diff_summary() {
        // No starting_commit => analyze_run never shells out to git and falls
        // back to the stored diff_summary as the patch text. Fully offline.
        let state = test_state();
        let run_id = seed_run(&state, None);
        store(&state)
            .replace_file_changes(
                &run_id,
                &[NewFileChange {
                    path: "src/config.rs".into(),
                    change_type: "created".into(),
                    diff_summary: Some(planted_secret_patch()),
                }],
            )
            .unwrap();

        analyze_run(State(state.clone()), Path(run_id.clone()))
            .await
            .map_err(|e| e.message)
            .expect("analyze_run should succeed offline");

        let findings = store(&state).list_policy_findings(&run_id).unwrap();
        assert!(
            findings.iter().any(|f| f.rule_key == "secret-in-diff"),
            "expected a secret-in-diff finding to be persisted, got: {findings:?}"
        );
    }
}
