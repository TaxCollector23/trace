//! Loopback provider proxy and metadata-only usage capture.
//!
//! The parent daemon can mount [`router`] below `/proxy`.  This module never
//! logs or persists request/response content or provider credentials.  It
//! buffers only bounded bytes long enough to pass the provider response back
//! to the caller and extract provider-reported usage metadata.

use std::{io::Read, time::Instant};

use axum::{
    body::{Body, Bytes},
    extract::{OriginalUri, Path, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use trace_core::models::NewApiUsage;

use crate::state::AppState;

const OPENAI_ORIGIN: &str = "https://api.openai.com";
const ANTHROPIC_ORIGIN: &str = "https://api.anthropic.com";
const GOOGLE_ORIGIN: &str = "https://generativelanguage.googleapis.com";

/// Request and response limits keep the local proxy bounded.  These are large
/// enough for normal chat requests while ensuring a provider cannot make the
/// daemon retain an unbounded body in memory.
pub const MAX_REQUEST_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
const MAX_RUN_ID_BYTES: usize = 256;

/// A provider name accepted by the proxy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    OpenAi,
    Anthropic,
    Google,
}

impl Provider {
    /// Parse the intentionally small provider allowlist.  `gemini` is an
    /// alias for Google's Generative Language API, not a user-controlled host.
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "openai" => Some(Self::OpenAi),
            "anthropic" => Some(Self::Anthropic),
            "google" | "gemini" => Some(Self::Google),
            _ => None,
        }
    }

    fn origin(self) -> &'static str {
        match self {
            Self::OpenAi => OPENAI_ORIGIN,
            Self::Anthropic => ANTHROPIC_ORIGIN,
            Self::Google => GOOGLE_ORIGIN,
        }
    }

    fn canonical_name(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Google => "google",
        }
    }
}

/// Provider-reported token usage plus Trace's local price-table estimate.
///
/// Provider-reported usage is telemetry, not an invoice.  The estimate is
/// calculated locally from `trace_core::cost::estimate_cost`; it is not a
/// provider bill, subscription charge, or guarantee of what was charged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageCapture {
    pub provider: String,
    pub model: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cached_tokens: Option<i64>,
    pub trace_estimated_cost_usd: Option<f64>,
}

/// Build the router to mount below `/proxy` in the daemon's loopback router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/:provider/*path", any(handler))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_REQUEST_BYTES))
}

/// Proxy one provider request and return the provider status/body unchanged
/// (within [`MAX_RESPONSE_BYTES`]).  This handler deliberately does not expose
/// upstream error details in locally generated errors.
pub async fn handler(
    State(state): State<AppState>,
    Path((provider_name, path)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(provider) = Provider::parse(&provider_name) else {
        return json_error(StatusCode::NOT_FOUND, "provider_not_allowed");
    };

    if body.len() > MAX_REQUEST_BYTES {
        return json_error(StatusCode::PAYLOAD_TOO_LARGE, "request_too_large");
    }

    let url = match upstream_url(provider, &path, &uri) {
        Some(url) => url,
        None => return json_error(StatusCode::BAD_REQUEST, "invalid_upstream_path"),
    };
    let forwarded_headers = safe_request_headers(&headers);
    let run_id = trace_run_id(&headers);
    let request_body = body.to_vec();
    let upstream_body = request_body.clone();
    let request_method = method.clone();
    let started = Instant::now();

    let upstream = tokio::task::spawn_blocking(move || {
        send_upstream(request_method, &url, forwarded_headers, upstream_body)
    })
    .await;

    let upstream = match upstream {
        Ok(Ok(response)) => response,
        _ => return json_error(StatusCode::BAD_GATEWAY, "provider_unavailable"),
    };

    let capture = capture_usage(
        provider,
        &request_body,
        &upstream.body,
        upstream.content_type.as_deref(),
    );
    if let (Some(run_id), Some(capture)) = (run_id, capture.as_ref()) {
        persist_capture(&state, run_id, capture, started.elapsed().as_millis());
    }

    provider_response(upstream)
}

#[derive(Debug)]
struct UpstreamResponse {
    status: u16,
    content_type: Option<String>,
    body: Vec<u8>,
}

fn send_upstream(
    method: Method,
    url: &str,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
) -> Result<UpstreamResponse, ()> {
    let agent = ureq::AgentBuilder::new().build();
    let mut request = agent.request(method.as_str(), url);
    for (name, value) in headers {
        request = request.set(&name, &value);
    }

    match request.send_bytes(&body) {
        Ok(response) => read_upstream_response(response),
        Err(ureq::Error::Status(_, response)) => read_upstream_response(response),
        Err(_) => Err(()),
    }
}

fn read_upstream_response(response: ureq::Response) -> Result<UpstreamResponse, ()> {
    let status = response.status();
    let content_type = response.header("content-type").map(str::to_owned);
    let mut body = Vec::new();
    let mut reader = response
        .into_reader()
        .take((MAX_RESPONSE_BYTES as u64).saturating_add(1));
    reader.read_to_end(&mut body).map_err(|_| ())?;
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(());
    }
    Ok(UpstreamResponse {
        status,
        content_type,
        body,
    })
}

fn provider_response(upstream: UpstreamResponse) -> Response {
    let status = StatusCode::from_u16(upstream.status).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut response = Response::new(Body::from(upstream.body));
    *response.status_mut() = status;
    if let Some(content_type) = upstream.content_type {
        if let Ok(value) = HeaderValue::from_str(&content_type) {
            response.headers_mut().insert(header::CONTENT_TYPE, value);
        }
    }
    response
}

fn json_error(status: StatusCode, code: &'static str) -> Response {
    (status, axum::Json(json!({ "error": code }))).into_response()
}

fn upstream_url(provider: Provider, path: &str, uri: &Uri) -> Option<String> {
    if path.bytes().any(|byte| byte.is_ascii_control()) {
        return None;
    }
    let path = path.trim_start_matches('/');
    if path.is_empty() {
        return None;
    }
    let mut url = format!("{}/{}", provider.origin(), path);
    if let Some(query) = uri.query() {
        if query.bytes().any(|byte| byte.is_ascii_control()) {
            return None;
        }
        url.push('?');
        url.push_str(query);
    }
    Some(url)
}

/// Return only headers required for provider protocol and authentication.
/// Hop-by-hop headers, cookies, trace attribution, and arbitrary user headers
/// are intentionally excluded.  Credential values are forwarded but never
/// logged or stored by this module.
pub fn safe_request_headers(headers: &HeaderMap) -> Vec<(String, String)> {
    const ALLOWED: &[&str] = &[
        "accept",
        "accept-encoding",
        "anthropic-beta",
        "anthropic-version",
        "authorization",
        "content-type",
        "x-api-key",
        "x-goog-api-key",
    ];

    ALLOWED
        .iter()
        .filter_map(|name| {
            headers
                .get(*name)
                .and_then(|value| value.to_str().ok())
                .map(|value| ((*name).to_string(), value.to_string()))
        })
        .collect()
}

fn trace_run_id(headers: &HeaderMap) -> Option<String> {
    let value = headers.get("x-trace-run-id")?.to_str().ok()?;
    if value.is_empty() || value.len() > MAX_RUN_ID_BYTES {
        return None;
    }
    Some(value.to_string())
}

fn persist_capture(state: &AppState, run_id: String, capture: &UsageCapture, latency_ms: u128) {
    let Ok(store) = state.store.lock() else {
        return;
    };
    let Ok(Some(_run)) = store.run_by_id(&run_id) else {
        return;
    };

    // `unknown` means the provider omitted a model identifier.  It is not
    // used to invent a price because estimate_cost returns None for it.
    let new_usage = NewApiUsage {
        provider: capture.provider.clone(),
        model: capture
            .model
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        input_tokens: capture.input_tokens,
        output_tokens: capture.output_tokens,
        cached_tokens: capture.cached_tokens,
        estimated_cost: capture.trace_estimated_cost_usd,
        latency_ms: Some(latency_ms.min(i64::MAX as u128) as i64),
    };
    let _ = store.add_api_usage(&run_id, &new_usage);
}

/// Extract metadata from a buffered provider response.  The request bytes are
/// inspected only for a model id; prompt, message, and response content are
/// never returned or persisted.
pub fn capture_usage(
    provider: Provider,
    request_body: &[u8],
    response_body: &[u8],
    content_type: Option<&str>,
) -> Option<UsageCapture> {
    let request_json = serde_json::from_slice::<Value>(request_body).ok();
    let request_model = request_json
        .as_ref()
        .and_then(|json| json.get("model"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let is_sse = content_type
        .map(|value| value.to_ascii_lowercase().starts_with("text/event-stream"))
        .unwrap_or_else(|| looks_like_sse(response_body));

    let values: Vec<Value> = if is_sse {
        parse_sse_values(response_body)
    } else {
        serde_json::from_slice(response_body)
            .ok()
            .into_iter()
            .collect()
    };
    if values.is_empty() {
        return None;
    }

    let mut capture = UsageCapture {
        provider: provider.canonical_name().to_string(),
        model: request_model,
        input_tokens: None,
        output_tokens: None,
        cached_tokens: None,
        trace_estimated_cost_usd: None,
    };

    for value in &values {
        if capture.model.is_none() {
            capture.model = model_from_response(provider, value);
        }
        match provider {
            Provider::OpenAi => apply_openai_usage(&mut capture, value),
            Provider::Anthropic => apply_anthropic_usage(&mut capture, value),
            Provider::Google => apply_google_usage(&mut capture, value),
        }
    }

    if capture.input_tokens.is_none()
        && capture.output_tokens.is_none()
        && capture.cached_tokens.is_none()
    {
        return None;
    }
    capture.trace_estimated_cost_usd = trace_core::cost::estimate_cost(
        &capture.provider,
        capture.model.as_deref().unwrap_or("unknown"),
        capture.input_tokens,
        capture.output_tokens,
        capture.cached_tokens,
    );
    Some(capture)
}

fn model_from_response(provider: Provider, value: &Value) -> Option<String> {
    match provider {
        Provider::OpenAi => value.get("model").and_then(Value::as_str),
        Provider::Anthropic => value
            .get("model")
            .or_else(|| {
                value
                    .get("message")
                    .and_then(|message| message.get("model"))
            })
            .and_then(Value::as_str),
        Provider::Google => value
            .get("modelVersion")
            .or_else(|| value.get("model"))
            .and_then(Value::as_str),
    }
    .map(str::to_owned)
}

fn apply_openai_usage(capture: &mut UsageCapture, value: &Value) {
    let Some(usage) = value.get("usage") else {
        return;
    };
    capture.input_tokens = number(usage, "prompt_tokens").or_else(|| number(usage, "input_tokens"));
    capture.output_tokens =
        number(usage, "completion_tokens").or_else(|| number(usage, "output_tokens"));
    capture.cached_tokens = usage
        .get("prompt_tokens_details")
        .and_then(|details| number(details, "cached_tokens"))
        .or_else(|| {
            usage
                .get("input_token_details")
                .and_then(|details| number(details, "cached_tokens"))
        });
}

fn apply_anthropic_usage(capture: &mut UsageCapture, value: &Value) {
    let usage = value.get("usage").or_else(|| {
        value
            .get("message")
            .and_then(|message| message.get("usage"))
    });
    let Some(usage) = usage else {
        return;
    };
    capture.input_tokens = number(usage, "input_tokens").or(capture.input_tokens);
    capture.output_tokens = number(usage, "output_tokens").or(capture.output_tokens);
    // Cache creation is intentionally not treated as cache-read usage: the
    // cost table has one cached-input rate and cannot honestly price creation
    // separately.
    capture.cached_tokens = number(usage, "cache_read_input_tokens").or(capture.cached_tokens);
}

fn apply_google_usage(capture: &mut UsageCapture, value: &Value) {
    let Some(usage) = value.get("usageMetadata") else {
        return;
    };
    capture.input_tokens = number(usage, "promptTokenCount").or(capture.input_tokens);
    capture.output_tokens = number(usage, "candidatesTokenCount").or(capture.output_tokens);
    capture.cached_tokens = number(usage, "cachedContentTokenCount").or(capture.cached_tokens);
}

fn number(value: &Value, key: &str) -> Option<i64> {
    value.get(key)?.as_i64().filter(|number| *number >= 0)
}

fn looks_like_sse(body: &[u8]) -> bool {
    body.windows(5).any(|window| window == b"data:")
}

fn parse_sse_values(body: &[u8]) -> Vec<Value> {
    let text = String::from_utf8_lossy(body);
    let mut values = Vec::new();
    let mut data = String::new();

    for line in text.lines() {
        if line.is_empty() {
            append_sse_value(&mut values, &mut data);
        } else if let Some(value) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(value.trim_start());
        }
    }
    append_sse_value(&mut values, &mut data);
    values
}

fn append_sse_value(values: &mut Vec<Value>, data: &mut String) {
    if data.is_empty() {
        return;
    }
    if data != "[DONE]" {
        if let Ok(value) = serde_json::from_str(data) {
            values.push(value);
        }
    }
    data.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE};

    #[test]
    fn provider_allowlist_has_fixed_origins() {
        assert_eq!(Provider::parse("openai"), Some(Provider::OpenAi));
        assert_eq!(Provider::parse("ANTHROPIC"), Some(Provider::Anthropic));
        assert_eq!(Provider::parse("gemini"), Some(Provider::Google));
        assert!(Provider::parse("example.com").is_none());
        assert_eq!(Provider::OpenAi.origin(), OPENAI_ORIGIN);
        assert_eq!(Provider::Google.origin(), GOOGLE_ORIGIN);
    }

    #[test]
    fn header_filtering_drops_credentials_metadata_and_hop_by_hop_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer secret"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(CONTENT_LENGTH, HeaderValue::from_static("42"));
        headers.insert("x-trace-run-id", HeaderValue::from_static("run-1"));
        headers.insert("cookie", HeaderValue::from_static("session=secret"));

        let filtered = safe_request_headers(&headers);
        assert!(filtered
            .iter()
            .any(|(name, value)| { name == "authorization" && value == "Bearer secret" }));
        assert!(filtered.iter().any(|(name, _)| name == "content-type"));
        assert!(!filtered.iter().any(|(name, _)| name == "content-length"));
        assert!(!filtered.iter().any(|(name, _)| name == "x-trace-run-id"));
        assert!(!filtered.iter().any(|(name, _)| name == "cookie"));
    }

    #[test]
    fn captures_openai_json_usage_and_estimate() {
        let capture = capture_usage(
            Provider::OpenAi,
            br#"{"model":"gpt-4o-mini","messages":[]}"#,
            br#"{"model":"gpt-4o-mini","usage":{"prompt_tokens":1000,"completion_tokens":500,"prompt_tokens_details":{"cached_tokens":200}}}"#,
            Some("application/json"),
        )
        .expect("usage should be captured");

        assert_eq!(capture.input_tokens, Some(1000));
        assert_eq!(capture.output_tokens, Some(500));
        assert_eq!(capture.cached_tokens, Some(200));
        assert!(capture.trace_estimated_cost_usd.is_some());
    }

    #[test]
    fn captures_anthropic_json_usage() {
        let capture = capture_usage(
            Provider::Anthropic,
            br#"{"model":"claude-sonnet-4"}"#,
            br#"{"model":"claude-sonnet-4","usage":{"input_tokens":1200,"output_tokens":300,"cache_read_input_tokens":100}}"#,
            Some("application/json"),
        )
        .expect("usage should be captured");

        assert_eq!(capture.input_tokens, Some(1200));
        assert_eq!(capture.output_tokens, Some(300));
        assert_eq!(capture.cached_tokens, Some(100));
        assert_eq!(capture.model.as_deref(), Some("claude-sonnet-4"));
    }

    #[test]
    fn captures_sse_usage_for_openai_and_anthropic() {
        let openai = b"data: {\"id\":\"1\",\"model\":\"gpt-4o-mini\"}\n\ndata: {\"usage\":{\"prompt_tokens\":9,\"completion_tokens\":4}}\n\ndata: [DONE]\n\n";
        let openai_capture = capture_usage(Provider::OpenAi, b"{}", openai, None).unwrap();
        assert_eq!(openai_capture.input_tokens, Some(9));
        assert_eq!(openai_capture.output_tokens, Some(4));
        assert_eq!(openai_capture.model.as_deref(), Some("gpt-4o-mini"));

        let anthropic = b"event: message_start\ndata: {\"message\":{\"model\":\"claude-haiku\",\"usage\":{\"input_tokens\":8,\"cache_read_input_tokens\":2}}}\n\nevent: message_delta\ndata: {\"usage\":{\"output_tokens\":3}}\n\n";
        let anthropic_capture = capture_usage(
            Provider::Anthropic,
            b"{}",
            anthropic,
            Some("text/event-stream"),
        )
        .unwrap();
        assert_eq!(anthropic_capture.input_tokens, Some(8));
        assert_eq!(anthropic_capture.output_tokens, Some(3));
        assert_eq!(anthropic_capture.cached_tokens, Some(2));
        assert_eq!(anthropic_capture.model.as_deref(), Some("claude-haiku"));
    }
}
