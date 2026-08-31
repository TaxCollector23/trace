//! Per-token rate limiting.
//!
//! A small fixed-window limiter keyed by the opaque bearer token, so one
//! noisy or abusive token can't exhaust the service for everyone else. We key
//! by token rather than IP on purpose: many daemons behind one NAT share an IP
//! (IP keying would punish them collectively), and a token is the unit we
//! actually bill/trust. `tower_governor` keys by peer IP, so a small custom
//! layer is both lighter and a better fit here.
//!
//! Requests with no bearer token (the `/healthz` liveness probe, or anything
//! that will be rejected by auth anyway) are not counted, so Render's health
//! checks are never rate limited.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::{header::AUTHORIZATION, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Fixed-window counter per token. Simple and allocation-light; the window
/// resets lazily on the first request after it elapses.
pub struct RateLimit {
    window: Duration,
    max: u32,
    hits: Mutex<HashMap<String, (Instant, u32)>>,
}

impl RateLimit {
    pub fn new(max_per_window: u32, window: Duration) -> Self {
        RateLimit {
            window,
            max: max_per_window.max(1),
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// Build from `TRACE_RATE_LIMIT_PER_MIN` (default 120 requests/token/minute).
    pub fn from_env() -> Self {
        let max = std::env::var("TRACE_RATE_LIMIT_PER_MIN")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(120);
        RateLimit::new(max, Duration::from_secs(60))
    }

    /// Record a hit for `key`; returns `true` if it is within the limit.
    pub fn check(&self, key: &str, now: Instant) -> bool {
        // Recover the guard rather than panic if a previous holder panicked.
        let mut map = self.hits.lock().unwrap_or_else(|e| e.into_inner());

        // Opportunistic prune so the map can't grow without bound: when it gets
        // large, drop every entry whose window has already elapsed.
        if map.len() > 4096 {
            map.retain(|_, (start, _)| now.duration_since(*start) <= self.window);
        }

        let entry = map.entry(key.to_string()).or_insert((now, 0));
        if now.duration_since(entry.0) > self.window {
            *entry = (now, 0);
        }
        entry.1 += 1;
        entry.1 <= self.max
    }
}

/// Axum middleware: reject a token that has exceeded its window with 429.
pub async fn layer(
    State(limiter): State<Arc<RateLimit>>,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());

    if let Some(token) = token {
        if !limiter.check(&token, Instant::now()) {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                "rate limit exceeded; slow down",
            )
                .into_response();
        }
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_the_limit_then_blocks() {
        let rl = RateLimit::new(3, Duration::from_secs(60));
        let t = Instant::now();
        assert!(rl.check("tok", t));
        assert!(rl.check("tok", t));
        assert!(rl.check("tok", t));
        assert!(!rl.check("tok", t), "4th request in the window is blocked");
    }

    #[test]
    fn tokens_are_counted_independently() {
        let rl = RateLimit::new(1, Duration::from_secs(60));
        let t = Instant::now();
        assert!(rl.check("a", t));
        assert!(!rl.check("a", t), "a is now over its limit");
        assert!(rl.check("b", t), "b has its own separate budget");
    }

    #[test]
    fn window_resets_after_it_elapses() {
        let rl = RateLimit::new(1, Duration::from_secs(60));
        let t0 = Instant::now();
        assert!(rl.check("tok", t0));
        assert!(!rl.check("tok", t0));
        // A request just past the window resets the counter.
        let later = t0 + Duration::from_secs(61);
        assert!(rl.check("tok", later), "counter resets in the next window");
    }

    // The middleware wiring: token extraction from the header and the 429.
    mod middleware {
        use super::*;
        use axum::body::Body;
        use axum::http::{header, Request, StatusCode};
        use axum::routing::get;
        use axum::Router;
        use tower::ServiceExt; // oneshot

        fn app(max: u32) -> Router {
            let limiter = Arc::new(RateLimit::new(max, Duration::from_secs(60)));
            Router::new()
                .route("/", get(|| async { "ok" }))
                .layer(axum::middleware::from_fn_with_state(limiter, layer))
        }

        fn req(bearer: Option<&str>) -> Request<Body> {
            let mut b = Request::get("/");
            if let Some(t) = bearer {
                b = b.header(header::AUTHORIZATION, t);
            }
            b.body(Body::empty()).unwrap()
        }

        #[tokio::test]
        async fn over_limit_returns_429_for_the_same_token() {
            let app = app(1);
            let first = app.clone().oneshot(req(Some("Bearer tok"))).await.unwrap();
            assert_eq!(first.status(), StatusCode::OK);
            let second = app.clone().oneshot(req(Some("Bearer tok"))).await.unwrap();
            assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
            // A different token still has budget.
            let other = app.oneshot(req(Some("Bearer other"))).await.unwrap();
            assert_eq!(other.status(), StatusCode::OK);
        }

        #[tokio::test]
        async fn unauthenticated_requests_are_not_rate_limited() {
            // No bearer token (e.g. the health probe): never counted, even past
            // the limit, so Render's liveness checks are never throttled.
            let app = app(1);
            for _ in 0..5 {
                let resp = app.clone().oneshot(req(None)).await.unwrap();
                assert_eq!(resp.status(), StatusCode::OK);
            }
        }
    }
}
