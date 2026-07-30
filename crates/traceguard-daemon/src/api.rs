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
use traceguard_core::prompt::{self, CompressionMode, OutputBudgetPreset};
use traceguard_core::{git, models::*, Store};

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
        // Prompt hardening + agent hub
        .route("/guard", post(guard_prompt))
        .route("/agents", get(list_agents))
        .route("/launch", post(launch_agent))
        .route("/scan", post(scan_project))
        .route("/doctor", get(doctor))
        // GitHub (reads directly from the repo, including private)
        .route("/github/status", get(gh_status))
        .route("/github/commits", get(gh_commits))
        .route("/github/pulls", get(gh_pulls))
        .route("/github/file", get(gh_file))
        // TraceCompress
        .route("/prompt-compressor/compress", post(pc_compress))
        .route("/prompt-compressor/record", post(pc_record))
        .route("/prompt-compressor/output-budget", post(pc_output_budget))
        .route("/prompt-compressor/history", get(pc_history))
        .route("/prompt-compressor/:id", get(pc_get).delete(pc_delete))
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
        "service": "traceguard-daemon",
        "version": traceguard_core::VERSION,
    }))
}

async fn state_info(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    let s = store(&state);
    let projects = s.list_projects()?;
    Ok(Json(json!({
        "version": traceguard_core::VERSION,
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
    let s = store(&state);
    s.finish_run(
        &id,
        RunStatus::from_str(&body.status),
        body.exit_code,
        body.ending_commit.as_deref(),
    )?;
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
        .join(".traceguard")
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
    let result = traceguard_core::guard::classify(&body.command);
    Json(json!({
        "decision": result.decision.as_str(),
        "reason": result.reason,
    }))
}

// --- Prompt hardening + agent hub -----------------------------------------

#[derive(Deserialize)]
struct GuardBody {
    prompt: String,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    coding: bool,
    #[serde(default)]
    project_context: Option<String>,
    #[serde(default)]
    custom_rules: Vec<String>,
}

/// Harden a prompt (clean + rules + score + lint). Stateless; local.
async fn guard_prompt(Json(body): Json<GuardBody>) -> impl IntoResponse {
    use traceguard_core::harden::{harden, HardenMode};
    let mode = match (body.mode.as_deref(), body.coding) {
        (Some(m), _) => HardenMode::parse(m),
        (None, true) => HardenMode::Coding,
        (None, false) => HardenMode::Balanced,
    };
    let result = harden(
        &body.prompt,
        mode,
        body.project_context.as_deref(),
        &body.custom_rules,
    );
    Json(result)
}

/// List supported AI tools and their install status.
async fn list_agents() -> impl IntoResponse {
    Json(traceguard_core::agents::detect_all())
}

#[derive(Deserialize)]
struct LaunchBody {
    target: String,
    prompt: String,
    #[serde(default)]
    cwd: Option<String>,
}

/// Launch a prompt into an agent. `auto` routes to the best installed tool.
/// Web tools open + copy; CLI tools (Claude, etc.) open a new terminal running
/// them with the prompt. Secrets are scanned first.
async fn launch_agent(Json(body): Json<LaunchBody>) -> impl IntoResponse {
    let findings = traceguard_core::secrets::scan_text(&body.prompt);
    if !findings.is_empty() {
        return Json(json!({
            "method": "blocked",
            "launched": false,
            "message": "Prompt may contain secrets — remove them before launching.",
            "secrets": findings.iter().map(|f| f.secret_type.clone()).collect::<Vec<_>>(),
        }));
    }
    let target = if body.target == "auto" {
        traceguard_core::agents::route(&body.prompt).to_string()
    } else {
        body.target.clone()
    };
    let outcome =
        traceguard_core::agents::launch_detached(&target, &body.prompt, body.cwd.as_deref());
    Json(serde_json::to_value(outcome).unwrap_or(json!({"method":"error"})))
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
    Json(traceguard_core::scan::scan(&root))
}

/// Lightweight doctor: version, agents detected, clipboard availability.
async fn doctor(State(state): State<AppState>) -> impl IntoResponse {
    let agents = traceguard_core::agents::detect_all();
    let installed = agents.iter().filter(|a| a.installed).count();
    let clipboard = traceguard_core::agents::copy_to_clipboard("").is_ok();
    Json(json!({
        "version": traceguard_core::VERSION,
        "port": state.port,
        "db_path": state.db_path,
        "clipboard": clipboard,
        "agents_total": agents.len(),
        "agents_installed": installed,
        "agents": agents,
    }))
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
    traceguard_core::github::RepoRef,
    Option<String>,
)> {
    let project = store(state)
        .project_by_id(project_id)?
        .ok_or_else(|| ApiError::not_found("project"))?;
    let path = std::path::PathBuf::from(&project.path);
    let repo_ref = traceguard_core::git::remote_url(&path)
        .and_then(|u| traceguard_core::github::parse_remote(&u))
        .ok_or_else(|| ApiError {
            status: StatusCode::BAD_REQUEST,
            message: "project has no GitHub origin remote".into(),
        })?;
    let (token, _) = traceguard_core::github::resolve_token();
    Ok((path, repo_ref, token))
}

async fn gh_status(
    State(state): State<AppState>,
    Query(q): Query<GhQuery>,
) -> ApiResult<impl IntoResponse> {
    let project = store(&state)
        .project_by_id(&q.project_id)?
        .ok_or_else(|| ApiError::not_found("project"))?;
    let status = traceguard_core::github::status_for_path(std::path::Path::new(&project.path));
    Ok(Json(status))
}

async fn gh_commits(
    State(state): State<AppState>,
    Query(q): Query<GhQuery>,
) -> ApiResult<impl IntoResponse> {
    let (_p, r, token) = gh_repo_ref(&state, &q.project_id)?;
    let commits =
        traceguard_core::github::list_commits(&r, token.as_deref(), q.limit.unwrap_or(20))
            .map_err(ApiError::from)?;
    Ok(Json(commits))
}

async fn gh_pulls(
    State(state): State<AppState>,
    Query(q): Query<GhQuery>,
) -> ApiResult<impl IntoResponse> {
    let (_p, r, token) = gh_repo_ref(&state, &q.project_id)?;
    let pulls =
        traceguard_core::github::list_pulls(&r, token.as_deref()).map_err(ApiError::from)?;
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
    let content =
        traceguard_core::github::get_file(&r, &path, q.r#ref.as_deref(), token.as_deref())
            .map_err(ApiError::from)?;
    Ok(Json(json!({ "path": path, "content": content })))
}

// --- TraceCompress (prompt compressor + output budget) -------------------

#[derive(Deserialize)]
struct CompressBody {
    prompt: String,
    #[serde(default)]
    mode: Option<String>,
}

/// Compute a deterministic compression. Does not store anything. Runs locally.
async fn pc_compress(Json(body): Json<CompressBody>) -> impl IntoResponse {
    let mode = CompressionMode::parse(body.mode.as_deref().unwrap_or("concise"));
    Json(prompt::compress_with_mode(&body.prompt, mode))
}

#[derive(Deserialize)]
struct OutputBudgetBody {
    preset: String,
}

/// Render an output-budget instruction block for a named preset.
async fn pc_output_budget(Json(body): Json<OutputBudgetBody>) -> impl IntoResponse {
    let preset = OutputBudgetPreset::parse(&body.preset);
    Json(json!({
        "preset": preset.as_str(),
        "instruction_block": preset.to_instruction_block(),
    }))
}

/// Store a compression record (honoring prompt-history settings on the caller).
async fn pc_record(
    State(state): State<AppState>,
    Json(body): Json<NewPromptCompression>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).add_prompt_compression(&body)?))
}

async fn pc_history(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).list_prompt_compressions(100)?))
}

async fn pc_get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    store(&state)
        .prompt_compression_by_id(&id)?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("compression"))
}

async fn pc_delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let deleted = store(&state).delete_prompt_compression(&id)?;
    Ok(Json(json!({ "ok": deleted })))
}
