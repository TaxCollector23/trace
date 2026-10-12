//! HTTP routes + OpenAPI doc.
//!
//! Every route is utoipa-annotated so `/openapi.json` and `/docs` (Swagger
//! UI) reflect exactly what the code does. Adding a route means adding it
//! both to `router()` and to the `paths(...)` list in `ApiDoc` — one file,
//! one place to keep in sync.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::OpenApi;

use crate::auth::AuthedUser;
use crate::db::{CloudEvent, RunSummary, RunUpload, Store};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
}

impl AppState {
    pub fn new(store: Store) -> Self {
        AppState {
            store: Arc::new(store),
        }
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Health {
    pub service: &'static str,
    pub status: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UploadReceipt {
    pub run_id: String,
    pub ok: bool,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListQuery {
    /// Max rows to return, default 50, capped at 500.
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RunDetail {
    pub run: RunSummary,
    pub events: Vec<CloudEvent>,
}

// ---- handlers -------------------------------------------------------------

/// Liveness probe. Never touches the DB, so a poisoned SQLite lock can't
/// take the /healthz check with it — Render depends on this returning 200
/// for a healthy deploy.
#[utoipa::path(get, path = "/healthz", responses((status = 200, body = Health)))]
async fn healthz() -> impl IntoResponse {
    Json(Health {
        service: "trace-cloud-api",
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Upload a run + its timeline events. Idempotent by `run.id`: re-uploading
/// the same run replaces its events (safe to retry).
#[utoipa::path(
    post, path = "/v1/runs",
    request_body = RunUpload,
    responses(
        (status = 200, body = UploadReceipt),
        (status = 401, description = "missing or invalid Bearer token"),
    ),
    security(("bearer_auth" = []))
)]
async fn upload_run(
    State(state): State<AppState>,
    user: AuthedUser,
    Json(body): Json<RunUpload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state
        .store
        .insert_run(&user.user_id, &body)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(UploadReceipt {
        run_id: body.run.id,
        ok: true,
    }))
}

/// List the current user's runs, most recent first.
#[utoipa::path(
    get, path = "/v1/runs",
    params(ListQuery),
    responses((status = 200, body = Vec<RunSummary>)),
    security(("bearer_auth" = []))
)]
async fn list_runs(
    State(state): State<AppState>,
    user: AuthedUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let limit = q.limit.unwrap_or(50).min(500);
    let rows = state
        .store
        .list_runs(&user.user_id, limit)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(rows))
}

/// Fetch one run with its full event timeline. Returns 404 if the run
/// exists but belongs to a different user — the same status as "doesn't
/// exist," so a token holder can never probe for other users' run IDs.
#[utoipa::path(
    get, path = "/v1/runs/{run_id}",
    responses(
        (status = 200, body = RunDetail),
        (status = 404, description = "not found or not yours"),
    ),
    security(("bearer_auth" = []))
)]
async fn get_run(
    State(state): State<AppState>,
    user: AuthedUser,
    Path(run_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let out = state
        .store
        .get_run(&user.user_id, &run_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    match out {
        Some((run, events)) => Ok(Json(RunDetail { run, events })),
        None => Err((StatusCode::NOT_FOUND, "not found".into())),
    }
}

// ---- router ---------------------------------------------------------------

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/runs", post(upload_run).get(list_runs))
        .route("/v1/runs/:run_id", get(get_run))
        .with_state(state)
}

// ---- OpenAPI --------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Trace Cloud API",
        description = "Hosted backend for Trace. Local daemons POST completed runs here; the web dashboard reads them back. Auth is opaque bearer tokens.",
        version = "1.3.0",
    ),
    paths(healthz, upload_run, list_runs, get_run),
    components(
        schemas(Health, UploadReceipt, RunUpload, RunSummary, CloudEvent, RunDetail),
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "runs", description = "Sync + read Trace runs from any authorized daemon."),
    ),
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("opaque")
                    .build(),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt; // for `oneshot`

    fn app() -> Router {
        router(AppState::new(Store::open(":memory:").unwrap()))
    }

    async fn body_json(resp: axum::response::Response) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    }

    fn get(path: &str) -> Request<Body> {
        Request::get(path).body(Body::empty()).unwrap()
    }

    fn get_auth(path: &str, bearer: &str) -> Request<Body> {
        Request::get(path)
            .header(header::AUTHORIZATION, bearer)
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn healthz_is_public_and_ok() {
        let resp = app().oneshot(get("/healthz")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_json(resp).await["status"], "ok");
    }

    #[tokio::test]
    async fn missing_authorization_header_is_401() {
        let resp = app().oneshot(get("/v1/runs")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn wrong_scheme_is_401() {
        // Not a `Bearer` token (e.g. Basic / Token) must be rejected.
        let resp = app()
            .oneshot(get_auth("/v1/runs", "Token abc123"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn empty_bearer_token_is_401() {
        let resp = app()
            .oneshot(get_auth("/v1/runs", "Bearer    "))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn valid_token_registers_user_and_returns_empty_list() {
        let resp = app()
            .oneshot(get_auth("/v1/runs", "Bearer trace_opaque_123"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_json(resp).await.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn upload_list_get_and_cross_user_isolation() {
        // One app instance -> one shared in-memory store across requests.
        let app = app();
        let upload = serde_json::json!({
            "run": {
                "id": "r1", "project_name": "p", "agent_name": null, "command": "echo",
                "user_prompt": null, "status": "completed", "exit_code": 0,
                "created_at": "2026-01-01T00:00:00Z", "completed_at": null, "event_count": 0
            },
            "events": [
                { "event_type": "run_created", "message": "start", "metadata_json": null, "created_at": null }
            ]
        });

        // Upload as user A.
        let resp = app
            .clone()
            .oneshot(
                Request::post("/v1/runs")
                    .header(header::AUTHORIZATION, "Bearer tok-A")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(upload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // A sees exactly one run.
        let resp = app
            .clone()
            .oneshot(get_auth("/v1/runs", "Bearer tok-A"))
            .await
            .unwrap();
        assert_eq!(body_json(resp).await.as_array().unwrap().len(), 1);

        // A can fetch it; B gets 404 (isolation, not 403 that would leak existence).
        let resp = app
            .clone()
            .oneshot(get_auth("/v1/runs/r1", "Bearer tok-A"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let resp = app
            .clone()
            .oneshot(get_auth("/v1/runs/r1", "Bearer tok-B"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
