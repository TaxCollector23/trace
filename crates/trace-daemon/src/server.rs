//! Server assembly: port selection, router wiring, and the run loop.
//!
//! The daemon binds to 127.0.0.1 only. It never listens on 0.0.0.0 — this is a
//! local-only tool and must not be reachable from the local network.

use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use axum::http::{HeaderValue, Method};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::extract::Request;
use axum::Router;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use trace_core::{paths, Store};

use crate::api;
use crate::assets;
use crate::state::{AppState, DaemonState};

/// The preferred first port. Falls back to the next free port if busy.
pub const PREFERRED_PORT: u16 = 8757;
const LOOPBACK: Ipv4Addr = Ipv4Addr::LOCALHOST; // 127.0.0.1

/// Bind a TCP listener on 127.0.0.1, starting at `start_port` and trying the
/// next ports until one is free. Returns the listener and the chosen port.
async fn bind_available(start_port: u16) -> Result<(TcpListener, u16)> {
    // Try up to 100 sequential ports before giving up.
    for port in start_port..start_port.saturating_add(100) {
        match TcpListener::bind((LOOPBACK, port)).await {
            Ok(listener) => return Ok((listener, port)),
            Err(_) => continue,
        }
    }
    anyhow::bail!(
        "no free port found in range {start_port}..{}",
        start_port + 100
    )
}

/// Security response headers, applied to everything the daemon serves.
///
/// The desktop shell points its webview at this server via
/// `WebviewUrl::External("http://127.0.0.1:<port>")` rather than Tauri's own
/// bundled-asset protocol — which means `tauri.conf.json`'s `csp` setting
/// (Tauri only injects that into content it serves itself) never actually
/// applies to this app. The real place to set it is here, as a response
/// header, which has the added benefit of covering the *other* legitimate
/// way to view the dashboard: a plain browser pointed at the daemon
/// (`trace dashboard`). Since the dashboard is entirely self-contained (own
/// bundled JS, no CDN/external script dependencies), a strict same-origin
/// policy costs nothing functionally.
async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'",
        ),
    );
    headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response
}

/// Explicit origin allow-list, deliberately *not* `CorsLayer::permissive()`.
///
/// The packaged app (Tauri desktop shell, or the daemon serving its own
/// built dashboard) loads the UI from the same origin as the API
/// (`http://127.0.0.1:<port>`), so same-origin requests need no CORS grant
/// at all — browsers don't apply CORS to same-origin fetches. The only
/// legitimate cross-origin caller is the Vite dev server during local
/// development, on its own well-known port.
///
/// A wildcard/permissive policy here would mean *any webpage the user has
/// open in a normal browser tab* — not just this app — could script a fetch
/// against this daemon: read judge/provider configuration, trigger a git
/// rollback, kick off doctrine mining against the user's GitHub token, or
/// spam `/config/judge/test` to burn API credits. Restricting to known
/// origins makes the browser's CORS preflight fail for anything else,
/// which closes that off for the practical (browser-based) case.
fn dev_origins() -> Vec<HeaderValue> {
    ["http://localhost:5173", "http://127.0.0.1:5173"]
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect()
}

/// Build the full application router (API + embedded dashboard).
fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::list(dev_origins()))
        .allow_methods(tower_http::cors::AllowMethods::list([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
        ]))
        .allow_headers(tower_http::cors::AllowHeaders::any());
    Router::new()
        .nest("/api", api::router())
        .fallback(assets::static_handler)
        .layer(cors)
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

/// Run the daemon until Ctrl-C (or SIGTERM). Writes `daemon.json` on start and
/// clears it on shutdown.
pub async fn serve(preferred_port: u16) -> Result<()> {
    let db_path = paths::database_path()?;
    let store = Store::open(&db_path).context("opening database")?;
    let global_config = trace_core::GlobalConfig::load().context("loading global config")?;

    let (listener, port) = bind_available(preferred_port).await?;
    let started_at = trace_core::time::now_rfc3339();

    let state = AppState {
        store: Arc::new(Mutex::new(store)),
        global_config: Arc::new(Mutex::new(global_config)),
        judge_cooldown: Arc::new(Mutex::new(std::collections::HashMap::new())),
        port,
        started_at: started_at.clone(),
        db_path: db_path.display().to_string(),
    };

    // Record where we are so the CLI can find us.
    DaemonState {
        pid: std::process::id(),
        port,
        started_at: started_at.clone(),
    }
    .write()?;

    let app = build_router(state);

    tracing::info!("Trace daemon listening on http://127.0.0.1:{port}");
    println!("Trace daemon listening on http://127.0.0.1:{port}");

    let result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error");

    // Best-effort cleanup of the state file.
    let _ = DaemonState::clear();
    result
}

/// Resolve when the process receives Ctrl-C or SIGTERM.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
