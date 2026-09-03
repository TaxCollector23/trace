//! Thin synchronous HTTP client the CLI uses to talk to the local daemon.
//!
//! All persistence goes through the daemon so there is a single writer to the
//! SQLite database.

use anyhow::{anyhow, Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

/// A client bound to a running daemon on 127.0.0.1:<port>.
pub struct Client {
    base: String,
}

impl Client {
    pub fn new(port: u16) -> Self {
        Client {
            base: format!("http://127.0.0.1:{port}"),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    /// GET and deserialize the JSON body.
    pub fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = ureq::get(&self.url(path))
            .call()
            .map_err(|e| translate_error("GET", path, e))?;
        resp.into_json::<T>()
            .with_context(|| format!("decoding GET {path}"))
    }

    /// POST a JSON body and deserialize the JSON response.
    pub fn post_json<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T> {
        let resp = ureq::post(&self.url(path))
            .send_json(body)
            .with_context(|| format!("POST {path}"))?;
        resp.into_json::<T>()
            .with_context(|| format!("decoding POST {path}"))
    }

    /// POST a JSON body, ignoring the response body (fire-and-forget records).
    pub fn post<B: Serialize>(&self, path: &str, body: &B) -> Result<()> {
        ureq::post(&self.url(path))
            .send_json(body)
            .with_context(|| format!("POST {path}"))?;
        Ok(())
    }

    /// Readiness check; returns Ok(()) once the daemon's HTTP API answers at
    /// all. `/api/health` now reports real per-subsystem status
    /// (healthy/degraded/failed — see `trace-daemon::health_routes`), so this
    /// deliberately does NOT require overall `status == "healthy"`: a
    /// daemon that's up but missing `git`, say, is still a daemon this CLI
    /// can talk to and should not be reported as "failed to start". It only
    /// confirms the response actually came from `trace-daemon`.
    pub fn health(&self) -> Result<()> {
        let v: Value = self.get_json("/api/health")?;
        if v.get("service").and_then(|s| s.as_str()) == Some("trace-daemon") {
            Ok(())
        } else {
            Err(anyhow!("unexpected health response"))
        }
    }
}

/// Turn a raw transport/status error into something a user can act on.
///
/// Left as-is, a failed request surfaces as `error: GET /api/runs/<id>`
/// (raw HTTP verb + path) with the real cause hidden behind
/// `TRACE_DEBUG=1`. A 404 on a `/api/runs/<id>`-shaped path is common and
/// has one obvious cause — a typo'd or stale run id — so it's translated
/// into a direct message instead. Every other error keeps the original
/// `<method> <path>` context.
fn translate_error(method: &str, path: &str, err: ureq::Error) -> anyhow::Error {
    if let ureq::Error::Status(404, _) = &err {
        if let Some(run_id) = run_id_from_runs_path(path) {
            return anyhow!("run {run_id} not found — see `trc runs`");
        }
    }
    anyhow::Error::new(err).context(format!("{method} {path}"))
}

/// Extract `<id>` from a `/api/runs/<id>` or `/api/runs/<id>/...` path.
/// Returns `None` for paths that don't address a single run (e.g. the
/// `/api/runs` list endpoint), so those keep the generic error message.
fn run_id_from_runs_path(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("/api/runs/")?;
    let id = rest.split(['/', '?']).next()?;
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_run_id_from_bare_path() {
        assert_eq!(run_id_from_runs_path("/api/runs/abc123"), Some("abc123"));
    }

    #[test]
    fn extracts_run_id_from_nested_path() {
        assert_eq!(
            run_id_from_runs_path("/api/runs/abc123/timeline"),
            Some("abc123")
        );
        assert_eq!(
            run_id_from_runs_path("/api/runs/abc123/file-changes"),
            Some("abc123")
        );
    }

    #[test]
    fn does_not_match_the_runs_list_endpoint() {
        assert_eq!(run_id_from_runs_path("/api/runs"), None);
        assert_eq!(run_id_from_runs_path("/api/runs?limit=50"), None);
    }

    #[test]
    fn does_not_match_unrelated_paths() {
        assert_eq!(run_id_from_runs_path("/api/health"), None);
        assert_eq!(run_id_from_runs_path("/api/projects"), None);
    }
}
