//! Read-only routes for the deterministic intelligence spine
//! (`trace_core::intel`): normalized events, signals, correlated incidents,
//! and causal links for a run. Mounted into `api::router()` via a single
//! `.merge(...)` line.
//!
//! Every handler here is read-only against the existing `Store` — nothing in
//! this file executes a command, writes a file, or mutates any Trace state.
//! `events`/`signals`/`incidents` shapes match `apps/web/src/data.ts`'s
//! `NormalizedEvent`, `Signal`, and `Incident` interfaces (the `Incident`
//! shape has one additive `evidence` field beyond that interface — extra JSON
//! fields are ignored by consumers that don't know about them yet), so the v4
//! dashboard lights up with no further UI changes. `causality` is a new
//! Wave 2 endpoint with no `apps/web` consumer yet (see `intel::causality`'s
//! module docs for why it is its own endpoint).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::sync::MutexGuard;
use trace_core::Store;

use crate::state::AppState;

/// Build the intel-spine routes, to be merged into the main `/api` router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/runs/:id/events", get(normalized_events))
        .route("/runs/:id/signals", get(signals))
        .route("/runs/:id/incidents", get(incidents))
        .route("/runs/:id/causality", get(causality))
}

// Mirrors `api::ApiError` exactly (kept local so this module has no
// dependency on `api.rs`'s private error type — see module docs on why this
// file must not otherwise touch `api.rs`).
struct ApiError {
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

fn store(state: &AppState) -> MutexGuard<'_, Store> {
    state.store.lock().unwrap_or_else(|e| e.into_inner())
}

/// `GET /api/runs/:id/events` -> `NormalizedEvent[]`. 404 when the run itself
/// does not exist (the web layer's `v4.events()` call passes `absent:
/// "not_found"` for exactly this reason).
async fn normalized_events(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let bundle = trace_core::intel::run_intel_pipeline(&store(&state), &id)?
        .ok_or_else(|| ApiError::not_found("run"))?;
    Ok(Json(bundle.events))
}

/// `GET /api/runs/:id/signals` -> `Signal[]`.
async fn signals(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let bundle = trace_core::intel::run_intel_pipeline(&store(&state), &id)?
        .ok_or_else(|| ApiError::not_found("run"))?;
    Ok(Json(bundle.signals))
}

/// `GET /api/runs/:id/incidents` -> `Incident[]`.
async fn incidents(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let bundle = trace_core::intel::run_intel_pipeline(&store(&state), &id)?
        .ok_or_else(|| ApiError::not_found("run"))?;
    Ok(Json(bundle.incidents))
}

/// `GET /api/runs/:id/causality` -> `EventCausality[]` (one entry per event
/// that has at least one likely cause or effect — see `intel::causality`).
async fn causality(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let bundle = trace_core::intel::run_intel_pipeline(&store(&state), &id)?
        .ok_or_else(|| ApiError::not_found("run"))?;
    Ok(Json(bundle.causality))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;
    use trace_core::models::{NewCommand, NewProject, NewRun};

    fn test_state() -> AppState {
        AppState {
            store: Arc::new(Mutex::new(Store::open_in_memory().unwrap())),
            port: 0,
            started_at: "1970-01-01T00:00:00Z".to_string(),
            db_path: ":memory:".to_string(),
        }
    }

    /// Seed a project + run (+ a handful of identical commands so the
    /// retry-loop analyzer has something to say) and return the state and run id.
    fn test_state_with_run() -> (AppState, String) {
        let state = test_state();
        let s = store(&state);
        let project = s
            .upsert_project(&NewProject {
                name: "T".into(),
                path: "/tmp/intel-routes-test".into(),
                config_path: "/tmp/intel-routes-test/.trace/config.toml".into(),
            })
            .unwrap();
        let run = s
            .create_run(&NewRun {
                project_id: project.id,
                command: "run".into(),
                agent_name: Some("claude-code".into()),
                user_prompt: None,
                starting_commit: None,
            })
            .unwrap();
        for _ in 0..4 {
            s.add_command(
                &run.id,
                &NewCommand {
                    command: "npm test".into(),
                    decision: "allow".into(),
                    exit_code: Some(1),
                    stdout_path: None,
                    stderr_path: None,
                },
            )
            .unwrap();
        }
        drop(s);
        (state, run.id)
    }

    #[tokio::test]
    async fn events_route_returns_normalized_shape() {
        let (state, run_id) = test_state_with_run();
        let resp = router()
            .with_state(state)
            .oneshot(
                Request::get(format!("/runs/{run_id}/events"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 4);
        // Contract fields from apps/web/src/data.ts's NormalizedEvent.
        for field in [
            "id", "run_id", "ts_start", "kind", "actor", "source", "status", "risk",
        ] {
            assert!(arr[0].get(field).is_some(), "missing field {field}");
        }
    }

    #[tokio::test]
    async fn signals_route_surfaces_the_retry_loop() {
        let (state, run_id) = test_state_with_run();
        let resp = router()
            .with_state(state)
            .oneshot(
                Request::get(format!("/runs/{run_id}/signals"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["kind"], "retry_loop");
        assert_eq!(arr[0]["algorithm_id"], "retry_loop_v1");
    }

    /// The endpoint this Wave 2 task exists to fix: 4 identical trailing
    /// commands produce a Medium-severity `retry_loop` signal, which now
    /// correlates into a real, non-empty incident (Wave 1 only escalated
    /// `High` severity, so this run's incidents were empty before).
    #[tokio::test]
    async fn incidents_route_surfaces_a_correlated_incident() {
        let (state, run_id) = test_state_with_run();
        let resp = router()
            .with_state(state)
            .oneshot(
                Request::get(format!("/runs/{run_id}/incidents"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(
            arr.len(),
            1,
            "expected the retry_loop signal to correlate into one incident"
        );
        assert_eq!(arr[0]["status"], "open");
        assert!(arr[0]["evidence"].as_array().unwrap().len() >= 4);
        for field in [
            "id",
            "run_id",
            "severity",
            "status",
            "title",
            "summary",
            "signal_ids",
            "evidence",
            "first_seen",
            "last_seen",
        ] {
            assert!(arr[0].get(field).is_some(), "missing field {field}");
        }
    }

    #[tokio::test]
    async fn causality_route_returns_array_shape() {
        let (state, run_id) = test_state_with_run();
        let resp = router()
            .with_state(state)
            .oneshot(
                Request::get(format!("/runs/{run_id}/causality"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        // The 4 identical commands share a target and were recorded close in
        // time, so this must be a non-empty, well-shaped result.
        let arr = json.as_array().unwrap();
        assert!(!arr.is_empty());
        for field in ["event_id", "likely_causes", "likely_effects"] {
            assert!(arr[0].get(field).is_some(), "missing field {field}");
        }
    }

    #[tokio::test]
    async fn unknown_run_id_is_404_on_every_route() {
        let state = test_state();
        for path in [
            "/runs/does-not-exist/events",
            "/runs/does-not-exist/signals",
            "/runs/does-not-exist/incidents",
            "/runs/does-not-exist/causality",
        ] {
            let resp = router()
                .with_state(state.clone())
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND, "path: {path}");
        }
    }

    #[tokio::test]
    async fn incidents_route_is_empty_array_not_404_when_no_incidents() {
        let state = test_state();
        let run = {
            let s = store(&state);
            let project = s
                .upsert_project(&NewProject {
                    name: "T".into(),
                    path: "/tmp/intel-routes-test-2".into(),
                    config_path: "/tmp/intel-routes-test-2/.trace/config.toml".into(),
                })
                .unwrap();
            s.create_run(&NewRun {
                project_id: project.id,
                command: "run".into(),
                agent_name: None,
                user_prompt: None,
                starting_commit: None,
            })
            .unwrap()
        };

        let resp = router()
            .with_state(state)
            .oneshot(
                Request::get(format!("/runs/{}/incidents", run.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 0);
    }
}
