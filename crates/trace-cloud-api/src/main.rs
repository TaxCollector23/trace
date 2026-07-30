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
mod routes;

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use routes::{ApiDoc, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let port: u16 = std::env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(10000);
    let db_path = std::env::var("TRACE_CLOUD_DB").unwrap_or_else(|_| "/tmp/trace-cloud.db".into());

    let store = db::Store::open(&db_path)?;
    let state = AppState::new(store);

    // CORS: allow the Vercel dashboards (both Trace's landing and Ratify)
    // to hit this API from a browser. Explicit allow-list would be safer
    // long-term, but the endpoints are token-gated regardless — a browser
    // origin without a valid bearer token gets a 401 no matter what.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .merge(routes::router(state))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("trace-cloud-api listening on http://{addr}  (swagger: /docs)");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
