//! Serves the built React dashboard embedded into the binary.
//!
//! The dashboard is a single-page app, so unknown non-API paths fall back to
//! `index.html` to let client-side routing work.

use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

/// Embeds everything under `apps/web/dist` at compile time. A placeholder
/// `index.html` is checked in so the daemon always builds; the real Vite build
/// overwrites it before release.
#[derive(RustEmbed)]
#[folder = "../../apps/web/dist"]
struct Dashboard;

fn serve_embedded(path: &str) -> Option<Response> {
    let file = Dashboard::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    Some(
        (
            [(header::CONTENT_TYPE, mime.as_ref())],
            Body::from(file.data.into_owned()),
        )
            .into_response(),
    )
}

/// Real check (no I/O beyond an in-memory lookup): is the embedded dashboard
/// bundle actually present in this binary? A placeholder `index.html` is
/// checked in so the daemon always builds even before the Vite build runs,
/// so this specifically confirms *something* is embedded to serve — used by
/// the `/api/health` "dashboard_api" check.
pub fn dashboard_asset_present() -> bool {
    Dashboard::get("index.html").is_some()
}

/// Axum fallback handler: serve the static asset for the path, or `index.html`.
pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if let Some(resp) = serve_embedded(path) {
        return resp;
    }
    // SPA fallback.
    if let Some(resp) = serve_embedded("index.html") {
        return resp;
    }
    (StatusCode::NOT_FOUND, "dashboard assets not embedded").into_response()
}
