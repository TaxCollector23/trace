//! Server assembly: port selection, router wiring, and the run loop.
//!
//! The daemon binds to 127.0.0.1 only. It never listens on 0.0.0.0 — this is a
//! local-only tool and must not be reachable from the local network.

use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
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

/// Build the full application router (API + embedded dashboard).
fn build_router(state: AppState) -> Router {
    // Local-only tool; allow any localhost origin (e.g. Vite dev server).
    let cors = CorsLayer::permissive();
    Router::new()
        .nest("/api", api::router())
        .fallback(assets::static_handler)
        .layer(cors)
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
