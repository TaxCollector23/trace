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
        .route("/analytics/coaching", get(coaching_report))
        .route("/benchmarks", get(benchmarks))
        // Policy engine + judge (merged in from Ratify's review pipeline)
        .route("/runs/:id/prompt", post(record_prompt))
        .route("/runs/:id/analyze", post(analyze_run))
        .route("/runs/:id/hook-check", post(hook_check))
        .route("/runs/:id/policy", get(list_run_policy_findings))
        .route("/runs/:id/judge", get(list_run_judge_verdicts))
        .route("/judge/recent", get(recent_judge))
        .route("/prompts/recent", get(recent_prompts))
        .route("/config/judge", get(get_judge_config).put(put_judge_config))
        .route("/config/judge/test", post(test_judge_slot))
        .route("/projects/:id/doctrine", get(list_project_doctrine))
        .route("/projects/:id/doctrine/mine", post(mine_project_doctrine))
        // GitHub (reads directly from the repo, including private)
        .route("/github/status", get(gh_status))
        .route("/github/commits", get(gh_commits))
        .route("/github/pulls", get(gh_pulls))
        .route("/github/file", get(gh_file))
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

/// Lock the store, treating poisoning as a fatal server error.
fn store(state: &AppState) -> MutexGuard<'_, Store> {
    state.store.lock().expect("store mutex poisoned")
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

/// Personalized prompt-coaching feedback derived from the user's own recent
/// prompts — which patterns show up most, which correlate with flagged runs,
/// and one concrete example from their own history. Deterministic aggregation,
/// no LLM call; the semantic pass batches through the judge panel elsewhere.
async fn coaching_report(
    State(state): State<AppState>,
    Query(q): Query<LimitQuery>,
) -> ApiResult<impl IntoResponse> {
    let events = store(&state).recent_prompt_events(q.limit.unwrap_or(200) as i64)?;
    Ok(Json(trace_core::build_coaching_report(&events)))
}

/// Runs the policy engine's own labeled-fixture benchmark fresh on every
/// call — it's pure computation over in-memory fixtures (no I/O), so
/// there's nothing to cache or go stale. See `trace-core::eval`.
async fn benchmarks() -> impl IntoResponse {
    Json(trace_core::run_policy_eval())
}

// --- Prompt quality ---------------------------------------------------

#[derive(Deserialize)]
struct PromptBody {
    prompt_text: String,
}

/// Records a prompt sent to the agent and scores it with the heuristic
/// prompt-quality analyzer. Called by the CLI adapter at the start of a run,
/// where the user's instruction is already available.
async fn record_prompt(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PromptBody>,
) -> ApiResult<impl IntoResponse> {
    let analysis = trace_core::analyze_prompt(&body.prompt_text);
    let patterns_json = serde_json::to_string(&analysis.patterns).unwrap_or_else(|_| "[]".into());
    store(&state).add_prompt_event(
        &id,
        &NewPromptEvent {
            prompt_text: body.prompt_text,
            word_count: analysis.word_count,
            patterns_json,
            clarity_score: analysis.clarity_score,
            led_to_flag: false,
        },
    )?;
    Ok(Json(analysis))
}

async fn recent_prompts(
    State(state): State<AppState>,
    Query(q): Query<LimitQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).recent_prompt_events(q.limit.unwrap_or(100))?))
}

// --- Policy engine + 3-LLM judge (merged in from Ratify) ---------------

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

async fn analyze_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let (run, file_changes, judge_settings, project_path) = {
        let s = store(&state);
        let run = s.run_by_id(&id)?.ok_or_else(|| ApiError::not_found("run"))?;
        let file_changes = s.list_file_changes(&id)?;
        let judge_settings = state.global_config.lock().expect("config mutex poisoned").judge.clone();
        let project_path = s.project_by_id(&run.project_id)?.map(|p| p.path);
        (run, file_changes, judge_settings, project_path)
    };

    // The policy engine's checks (secret scanning, TODO detection, etc.) are
    // regex matches against real added/removed lines — a "+12 -3" stat
    // summary has nothing for them to match. Pull the actual patch text per
    // file from git when we have a starting commit and a project path;
    // fall back to the stat summary only if that's genuinely unavailable
    // (e.g. the project isn't a git repo), so this still degrades instead
    // of erroring.
    let patches: std::collections::HashMap<String, String> = match (&project_path, &run.starting_commit) {
        (Some(path), Some(from_ref)) => {
            trace_core::git::patches_by_file(std::path::Path::new(path), from_ref).unwrap_or_default()
        }
        _ => Default::default(),
    };

    let diffs: Vec<trace_core::FileDiff> = file_changes
        .iter()
        .map(|f| trace_core::FileDiff {
            filename: f.path.clone(),
            status: normalize_change_status(&f.change_type),
            additions: 0,
            deletions: 0,
            patch: patches.get(&f.path).cloned().or_else(|| f.diff_summary.clone()),
        })
        .collect();

    let policy_findings = trace_core::run_policy_checks(&diffs);
    store(&state).add_policy_findings(&id, &policy_findings)?;

    let mut response = json!({
        "policy_findings": policy_findings,
        "judge_verdict": null,
        "agent_instruction": null,
    });

    if judge_settings.mode != trace_core::JudgeMode::Disabled {
        let doctrine_rules = doctrine_lines(&state, &run.project_id);
        let ctx = trace_core::JudgeContext {
            subject: format!("Agent run on {}", run.project_id),
            agent_name: run.agent_name.clone(),
            user_prompt: run.user_prompt.clone(),
            command: Some(run.command.clone()),
            files: diffs,
            policy_findings: policy_findings.clone(),
            doctrine_rules,
        };

        let model_prompting_mode = judge_settings.model_prompting_mode;

        // Blocking network calls to up to three providers — run off the
        // async runtime so we don't stall other requests while waiting.
        let verdict = tokio::task::spawn_blocking(move || trace_core::run_judge(&judge_settings, &ctx))
            .await
            .map_err(|e| ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: format!("judge task panicked: {e}"),
            })?;

        let should_prompt_agent = model_prompting_mode
            && matches!(verdict.consensus, trace_core::Decision::RequireApproval | trace_core::Decision::Block);
        let action_taken = if should_prompt_agent { "agent_prompted" } else { "flagged_only" };

        store(&state).save_judge_verdict(&id, "run-analysis", &verdict, action_taken)?;

        let agent_instruction = should_prompt_agent.then(|| {
            format!(
                "Trace's review panel flagged this action ({}). {} Please stop, re-examine the last change, and address this before continuing.",
                verdict.consensus.as_str().replace('_', " "),
                verdict.summary
            )
        });

        response = json!({
            "policy_findings": policy_findings,
            "judge_verdict": verdict,
            "agent_instruction": agent_instruction,
        });
    }

    Ok(Json(response))
}

#[derive(Deserialize)]
struct HookCheckBody {
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
/// Minimum time between judge panel invocations for the same run. A rapid
/// save-loop (autosave, a formatter, an aggressive "format on keystroke"
/// setup) could otherwise trigger three paid model calls per debounced
/// file event — this caps the real-money cost of that without turning the
/// judge off. Policy-engine findings are never throttled: they're free,
/// local, and instant, so every edit still gets that layer of review even
/// during a judge cooldown window.
const JUDGE_COOLDOWN: std::time::Duration = std::time::Duration::from_secs(15);

/// Returns `true` and marks the run as "just judged" if enough time has
/// passed since the last judge call for this run — false if still cooling
/// down, in which case the caller should skip the judge and rely on the
/// (unthrottled) policy engine for this particular edit.
fn judge_cooldown_elapsed(state: &AppState, run_id: &str) -> bool {
    let mut cooldowns = state.judge_cooldown.lock().expect("cooldown mutex poisoned");
    let now = std::time::Instant::now();

    // Opportunistic eviction: this map has no natural upper bound otherwise
    // (one entry per run id, ever) and the daemon is meant to run for
    // days/weeks. Piggybacking cleanup on every call keeps it bounded
    // without a background task — cheap since it only runs on judge calls,
    // not every request.
    cooldowns.retain(|_, last| now.duration_since(*last) < JUDGE_COOLDOWN * 4);

    match cooldowns.get(run_id) {
        Some(last) if now.duration_since(*last) < JUDGE_COOLDOWN => false,
        _ => {
            cooldowns.insert(run_id.to_string(), now);
            true
        }
    }
}

async fn hook_check(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<HookCheckBody>,
) -> ApiResult<impl IntoResponse> {
    let (run, judge_settings) = {
        let s = store(&state);
        let run = s.run_by_id(&id)?.ok_or_else(|| ApiError::not_found("run"))?;
        let judge_settings = state.global_config.lock().expect("config mutex poisoned").judge.clone();
        (run, judge_settings)
    };

    let diff = trace_core::FileDiff {
        filename: body.file_path.clone().unwrap_or_else(|| "(unknown file)".to_string()),
        status: "modified".to_string(),
        additions: 0,
        deletions: 0,
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

    // Even before the judge runs, deterministic policy findings are worth
    // echoing to the agent as advisory. The hook script surfaces this at
    // exit 0 (no block) if the judge is off; if the judge later escalates,
    // this text becomes part of the blocking message too.
    if !policy_findings.is_empty() {
        response.agent_feedback = Some(format_policy_advisory(&policy_findings));
    }

    let judge_wanted = judge_settings.model_prompting_mode && judge_settings.mode != trace_core::JudgeMode::Disabled;
    // Only consult (and thereby reset) the cooldown clock when the judge
    // would actually run otherwise — an unrelated Bash-only run with the
    // judge off shouldn't touch this run's cooldown state at all.
    let judge_allowed_now = judge_wanted && judge_cooldown_elapsed(&state, &id);
    response.judge_on_cooldown = judge_wanted && !judge_allowed_now;

    if judge_allowed_now {
        let doctrine_rules = doctrine_lines(&state, &run.project_id);
        let ctx = trace_core::JudgeContext {
            subject: format!("{} on {}", body.tool_name, diffs[0].filename),
            agent_name: run.agent_name.clone(),
            user_prompt: run.user_prompt.clone(),
            command: None,
            files: diffs,
            policy_findings: policy_findings.clone(),
            doctrine_rules,
        };
        let verdict = tokio::task::spawn_blocking(move || trace_core::run_judge(&judge_settings, &ctx))
            .await
            .map_err(|e| ApiError { status: StatusCode::INTERNAL_SERVER_ERROR, message: format!("judge task panicked: {e}") })?;

        let should_block =
            matches!(verdict.consensus, trace_core::Decision::RequireApproval | trace_core::Decision::Block);
        let action_taken = if should_block { "agent_prompted" } else { "flagged_only" };
        store(&state).save_judge_verdict(&id, &format!("live-edit: {}", body.tool_name), &verdict, action_taken)?;

        response.consensus = Some(verdict.consensus.as_str().to_string());
        // Always fold the panel's reasoning into agent_feedback (not just
        // on block). A `warn` verdict still contains useful "here's what
        // could be better" that the hook echoes without exit 2, giving
        // the agent a self-correction chance before the next edit.
        if verdict.consensus != trace_core::Decision::Allow {
            response.agent_feedback = Some(format_agent_feedback(
                &policy_findings,
                &verdict,
                &body.tool_name,
                body.file_path.as_deref(),
            ));
        }

        if should_block {
            response.block = true;
            // Legacy `message` field mirrors agent_feedback so older shell
            // hooks that only read `message` still get the rich text.
            response.message = response.agent_feedback.clone();
        }
    }

    Ok(Json(response))
}

/// Produce the multi-line message that the coding agent actually reads
/// back — no dashboard visit needed. Concrete: every deterministic
/// finding, every disagreeing reviewer's own reasoning, and a directive
/// on what to do next.
fn format_agent_feedback(
    policy_findings: &[trace_core::PolicyFinding],
    verdict: &trace_core::JudgeVerdict,
    tool_name: &str,
    file_path: Option<&str>,
) -> String {
    use trace_core::Decision;
    let file = file_path.unwrap_or("(unknown file)");
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "[Trace] {} on {} → panel consensus: {} ({:.0}% confidence, {:.0}% agreement).",
        tool_name,
        file,
        verdict.consensus.as_str().replace('_', " "),
        verdict.confidence * 100.0,
        verdict.agreement * 100.0
    ));

    if !policy_findings.is_empty() {
        lines.push(String::new());
        lines.push("Deterministic checks found:".into());
        for f in policy_findings.iter().take(6) {
            lines.push(format!("  • [{}] {}: {}", f.severity.as_str(), f.title, f.description));
        }
        if policy_findings.len() > 6 {
            lines.push(format!("  … and {} more", policy_findings.len() - 6));
        }
    }

    // Each reviewer's actual reasoning — this is what makes the feedback
    // *actionable* for the agent rather than a generic "please fix it."
    let successful: Vec<_> = verdict.votes.iter().filter(|v| v.error.is_none()).collect();
    if !successful.is_empty() {
        lines.push(String::new());
        lines.push("Reviewers said:".into());
        for v in &successful {
            let short = v.reasoning.trim();
            let short: String = short.chars().take(280).collect();
            if !short.is_empty() {
                lines.push(format!("  • {} ({:.0}%): {}", v.model, v.confidence * 100.0, short));
            }
        }
    }

    lines.push(String::new());
    lines.push(match verdict.consensus {
        Decision::Block => "→ Do not continue with this edit. Revert or rework the change to address the highest-severity issue above, then explain what you changed.".into(),
        Decision::RequireApproval => "→ Pause here. Explain your rationale in one sentence and wait for confirmation before continuing.".into(),
        Decision::Warn => "→ Not blocking, but before your next edit, address the issue above or note explicitly why it's acceptable.".into(),
        Decision::Allow => "→ No action required.".into(),
    });

    lines.join("\n")
}

/// Advisory formatting used when the judge didn't run but the deterministic
/// policy engine found things worth telling the agent about. Kept short —
/// the agent will still be moving forward, so we prioritize signal density.
fn format_policy_advisory(findings: &[trace_core::PolicyFinding]) -> String {
    let mut out = String::from("[Trace] Deterministic checks:");
    for f in findings.iter().take(6) {
        out.push_str(&format!("\n  • [{}] {}: {}", f.severity.as_str(), f.title, f.description));
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
fn doctrine_lines(state: &AppState, project_id: &str) -> Vec<String> {
    store(state)
        .list_doctrine_rules(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|r| format!("[{} · {}] {}", r.strength, r.category, r.rule_text))
        .collect()
}

async fn list_project_doctrine(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).list_doctrine_rules(&id)?))
}

/// Mines doctrine for a project from its GitHub PR review history and
/// replaces whatever was previously stored. Entirely server-side: the
/// project's path is already known (it's registered), so this resolves the
/// GitHub remote and a read-only token the same way `/api/github/*` does,
/// then runs the mining pass using one of the judge panel's configured
/// providers. Returns a clear reason rather than an error when mining can't
/// produce rules (no remote, no token, no judge provider, no PR history) —
/// none of those are failures, they're just "nothing to mine yet."
async fn mine_project_doctrine(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let (project, judge_settings) = {
        let s = store(&state);
        let project = s.project_by_id(&id)?.ok_or_else(|| ApiError::not_found("project"))?;
        let judge_settings = state.global_config.lock().expect("config mutex poisoned").judge.clone();
        (project, judge_settings)
    };

    let repo_ref = trace_core::git::remote_url(std::path::Path::new(&project.path))
        .and_then(|u| trace_core::github::parse_remote(&u));
    let Some(repo_ref) = repo_ref else {
        return Ok(Json(json!({
            "rules": [],
            "prs_analyzed": 0,
            "reason": "project has no GitHub origin remote",
        })));
    };

    let result = tokio::task::spawn_blocking(move || trace_core::mine_doctrine(&repo_ref, &judge_settings, 12))
        .await
        .map_err(|e| ApiError { status: StatusCode::INTERNAL_SERVER_ERROR, message: format!("mining task panicked: {e}") })?;

    match result {
        Ok(mined) => {
            store(&state).replace_doctrine_rules(&id, &mined.rules)?;
            let rules = store(&state).list_doctrine_rules(&id)?;
            Ok(Json(json!({ "rules": rules, "prs_analyzed": mined.prs_analyzed, "reason": null })))
        }
        Err(e) => Ok(Json(json!({ "rules": [], "prs_analyzed": 0, "reason": e.to_string() }))),
    }
}

async fn list_run_policy_findings(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).list_policy_findings(&id)?))
}

async fn list_run_judge_verdicts(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).list_judge_verdicts(&id)?))
}

async fn recent_judge(
    State(state): State<AppState>,
    Query(q): Query<LimitQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).recent_judge_verdicts(q.limit.unwrap_or(50))?))
}

#[derive(Serialize)]
struct TestSlotResponse {
    ok: bool,
    message: String,
}

/// Verifies one provider slot actually works — real network call, real
/// response check — before the user commits to saving it. If the request
/// doesn't include a key (testing an already-saved slot rather than a
/// freshly-typed one), falls back to whatever's currently stored for that
/// provider so "test" works without having to retype the key.
async fn test_judge_slot(
    State(state): State<AppState>,
    Json(mut slot): Json<trace_core::judge::ProviderSlot>,
) -> ApiResult<impl IntoResponse> {
    if slot.api_key.is_none() {
        let cfg = state.global_config.lock().expect("config mutex poisoned");
        slot.api_key = cfg
            .judge
            .slots
            .iter()
            .find(|s| s.provider == slot.provider)
            .and_then(|s| s.api_key.clone());
    }

    let result = tokio::task::spawn_blocking(move || {
        let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(15)).build();
        trace_core::judge::call_provider_raw(&slot, "Respond with exactly one word: ok", &agent)
    })
    .await
    .map_err(|e| ApiError { status: StatusCode::INTERNAL_SERVER_ERROR, message: format!("test task panicked: {e}") })?;

    Ok(Json(match result {
        Ok(content) if !content.trim().is_empty() => {
            TestSlotResponse { ok: true, message: format!("Connected — model responded: \"{}\"", content.trim().chars().take(60).collect::<String>()) }
        }
        Ok(_) => TestSlotResponse { ok: false, message: "Connected, but the model returned an empty response.".to_string() },
        Err(e) => TestSlotResponse { ok: false, message: e.to_string() },
    }))
}

async fn get_judge_config(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = state.global_config.lock().expect("config mutex poisoned");
    Json(cfg.redacted())
}

/// Update judge settings. Accepts a full `JudgeSettings` body; a slot with
/// `api_key: null` keeps its previously stored key (so the redacted GET
/// response can safely be edited and PUT back without clobbering keys).
async fn put_judge_config(
    State(state): State<AppState>,
    Json(mut body): Json<trace_core::JudgeSettings>,
) -> ApiResult<impl IntoResponse> {
    let mut cfg = state.global_config.lock().expect("config mutex poisoned");
    for (i, slot) in body.slots.iter_mut().enumerate() {
        if slot.api_key.is_none() {
            if let Some(existing) = cfg.judge.slots.get(i) {
                slot.api_key = existing.api_key.clone();
            }
        }
    }
    cfg.judge = body;
    cfg.save().map_err(|e| ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: e.to_string(),
    })?;
    Ok(Json(cfg.redacted()))
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
