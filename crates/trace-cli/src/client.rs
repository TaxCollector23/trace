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
            .with_context(|| format!("GET {path}"))?;
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

    /// Health check; returns Ok(()) only when the daemon answers.
    pub fn health(&self) -> Result<()> {
        let v: Value = self.get_json("/api/health")?;
        if v.get("status").and_then(|s| s.as_str()) == Some("ok") {
            Ok(())
        } else {
            Err(anyhow!("unexpected health response"))
        }
    }
}
