//! Bearer-token auth.
//!
//! Two paths, chosen at request time based on which header shape shows up:
//!
//! 1. **Daemon path** — `Authorization: Bearer trace_<opaque>`. The token
//!    is issued to the local daemon (paste from web dashboard), stored in
//!    ~/.trace/global.toml under [cloud], and never rotates automatically.
//!    First use of a token registers a user row; subsequent requests reuse
//!    the same user_id. Simple, no external service dependency.
//!
//! 2. **Browser path (future)** — Firebase ID token verification, sharing
//!    the same Firebase project as Ratify. For now the browser flow is
//!    stubbed to reuse the opaque-token path so the UI can be exercised
//!    end-to-end without pulling in the Firebase Admin SDK at cold start.
//!
//! Deliberately not JWT-issuing the daemon tokens ourselves: an opaque
//! token that lives in a database is easier to revoke (delete the row)
//! than a JWT with an unrevoked-issuer problem.

use axum::extract::FromRequestParts;
use axum::http::{request::Parts, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::routes::AppState;

pub struct AuthedUser {
    pub user_id: String,
}

pub struct AuthError(pub StatusCode, pub &'static str);

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (self.0, self.1).into_response()
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthedUser {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(AuthError(
                StatusCode::UNAUTHORIZED,
                "missing Authorization header",
            ))?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AuthError(
                StatusCode::UNAUTHORIZED,
                "expected `Bearer <token>`",
            ))?
            .trim();
        if token.is_empty() {
            return Err(AuthError(StatusCode::UNAUTHORIZED, "empty token"));
        }
        let user_id = state
            .store
            .upsert_user_by_token(token)
            .map_err(|_| AuthError(StatusCode::INTERNAL_SERVER_ERROR, "auth store failure"))?;
        Ok(AuthedUser { user_id })
    }
}
