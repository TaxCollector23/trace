//! trace-cloud-api — hosted backend for Trace.
//!
//! Deployed as a Render web service (see render.yaml at the repo root).
//! Local daemons opt in via `TRACE_CLOUD_URL` + `TRACE_CLOUD_TOKEN` env
//! vars (see `trace-daemon::cloud_sync`) and POST completed runs here so
//! they show up in the web dashboard at traceurl.vercel.app/dashboard.
//!
//! Design goals:
//!   1. **Zero surprise privacy:** default token auth is per-user opaque,
//!      never global. A daemon with no token configured never syncs.
//!   2. **Discoverable API:** every route is annotated with utoipa;
//!      /openapi.json + /docs (Swagger UI) are always live so any client
//!      (browser, curl, generated SDK) can inspect the surface.
//!   3. **Runnable everywhere:** SQLite by default so a fresh Render
//!      deploy works with no DB provisioning. Postgres migration path is
//!      an env-var swap when someone actually needs it.

mod auth;
mod db;
mod ratelimit;
mod routes;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::{HeaderValue, Method};
use axum::Router;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use routes::{ApiDoc, AppState};

/// Browser origins allowed to call this API. Deliberately an explicit
/// allow-list, not `Any`: the endpoints are token-gated, but a wildcard would
/// still let any page the user has open script cross-origin reads of the
/// response for a token it somehow obtained. Mirrors the daemon's `dev_origins`.
/// Extend for a preview/custom deploy with `TRACE_ALLOWED_ORIGINS` (comma-sep).
fn allowed_origins() -> Vec<HeaderValue> {
    let mut origins: Vec<String> = vec![
        "http://localhost:5173".into(),
        "http://127.0.0.1:5173".into(),
        "https://landing-one-hazel-88.vercel.app".into(),
        "https://ratify-zeta-dusky.vercel.app".into(),
    ];
    origins.extend(
        std::env::var("TRACE_ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    );
    origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10000);
    let db_path = std::env::var("TRACE_CLOUD_DB").unwrap_or_else(|_| "/tmp/trace-cloud.db".into());

    let store = db::Store::open(&db_path)?;
    let state = AppState::new(store);

    // CORS: explicit origin allow-list (see `allowed_origins`). Only the known
    // dashboards may read responses cross-origin; everything else is a network
    // error in the browser even before auth runs.
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins()))
        .allow_methods(AllowMethods::list([Method::GET, Method::POST]))
        .allow_headers(AllowHeaders::any());

    // Per-token rate limiting (default 120 req/token/min; TRACE_RATE_LIMIT_PER_MIN).
    let limiter = Arc::new(ratelimit::RateLimit::from_env());

    let app = Router::new()
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .merge(routes::router(state))
        .layer(axum::middleware::from_fn_with_state(
            limiter,
            ratelimit::layer,
        ))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("trace-cloud-api listening on http://{addr}  (swagger: /docs)");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
