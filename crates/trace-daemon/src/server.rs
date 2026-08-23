//! Server assembly: port selection, router wiring, and the run loop.
//!
//! The daemon binds to 127.0.0.1 only. It never listens on 0.0.0.0 — this is a
//! local-only tool and must not be reachable from the local network.

use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use axum::extract::Request;
use axum::http::{HeaderValue, Method};
use axum::middleware::{self, Next};
use axum::response::Response;
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
/// (`trc dashboard`). Since the dashboard is entirely self-contained (own
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
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response
}

/// Explicit origin allow-list, deliberately *not* `CorsLayer::permissive()`.
///
/// Three trusted origins are allowed to hit this daemon from a browser:
///   1. Vite dev server on 5173 (local dev of the embedded dashboard).
///   2. The Trace landing site's hosted /dashboard page, so a visitor
///      signed into nothing can still open <landing>/dashboard, have
///      their browser fetch from their own 127.0.0.1 daemon, and see
///      every local run — no cloud round-trip.
///   3. Ratify's dashboard, for the "Local Trace runs" tab.
///
/// A wildcard/permissive policy would let *any webpage the user has open
/// in a normal browser tab* script a fetch against this daemon — read a
/// user's run history and project paths, or trigger a git rollback. This
/// allow-list closes that.
///
/// Override the hosted origins with `TRACE_ALLOWED_ORIGINS` (comma-
/// separated) when running against a preview deployment or a custom
/// domain like trace.dev.
fn dev_origins() -> Vec<HeaderValue> {
    let mut origins: Vec<&str> = vec![
        "http://localhost:5173",
        "http://127.0.0.1:5173",
        "https://landing-one-hazel-88.vercel.app",
        "https://ratify-zeta-dusky.vercel.app",
    ];
    let extra: Vec<String> = std::env::var("TRACE_ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    for e in &extra {
        origins.push(e.as_str());
    }
    origins
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

    let (listener, port) = bind_available(preferred_port).await?;
    let started_at = trace_core::time::now_rfc3339();

    let state = AppState {
        store: Arc::new(Mutex::new(store)),
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
