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
use traceguard_core::prompt::{self, OutputBudget};
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
        .route("/prompt-compress", post(prompt_compress))
        .route("/output-budget", post(output_budget))
        .route(
            "/prompt-compressions",
            get(list_compressions).post(add_compression),
        )
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
    Json(json!({ "status": "ok", "service": "traceguard-daemon" }))
}

async fn state_info(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    let s = store(&state);
    let projects = s.list_projects()?;
    Ok(Json(json!({
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

// --- Prompt Compressor / output budget -----------------------------------

#[derive(Deserialize)]
struct CompressBody {
    prompt: String,
}

/// Compute a deterministic compression. Does not store anything. Runs locally.
async fn prompt_compress(Json(body): Json<CompressBody>) -> impl IntoResponse {
    Json(prompt::compress(&body.prompt))
}

/// Render an output-budget instruction block from selected options.
async fn output_budget(Json(budget): Json<OutputBudget>) -> impl IntoResponse {
    Json(json!({ "instruction_block": budget.to_instruction_block() }))
}

async fn list_compressions(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).list_prompt_compressions(100)?))
}

async fn add_compression(
    State(state): State<AppState>,
    Json(body): Json<NewPromptCompression>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(store(&state).add_prompt_compression(&body)?))
}
