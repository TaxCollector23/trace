//! Daemon Health Center + Integration Coverage + Data-Integrity routes.
//!
//! Everything here is **read-only** and **evidence-backed**: every status is
//! either the result of a real check performed at request time (a DB query,
//! a filesystem write probe, running `git --version`, reading an agent's own
//! config file) or an honest `null`/`"unknown"`/`"not_instrumented"` when no
//! such check exists yet. Nothing here is a hardcoded green badge.
//!
//! Mounted into the daemon's `/api` router from `server.rs` with a single
//! `.merge(health_routes::router())` — see that file for why the
//! pre-existing placeholder `/health` route in `api.rs` was removed rather
//! than left in place (axum panics on two routers registering the same
//! path+method, so the old stub and this real one cannot coexist).
//!
//! Detection logic that must never diverge from the CLI (`trc integrations
//! status`) or duplicate the guard/policy engine lives in `trace_core`
//! (`trace_core::integrations`, `trace_core::agents`, `Store::agent_activity`,
//! `Store::last_ingested_at`, `Store::integrity_scan`) — this file only
//! assembles those real checks into HTTP responses.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use serde_json::json;
use std::sync::MutexGuard;
use trace_core::Store;

use crate::state::AppState;

/// Build the health-center routes: `/health`, `/integrations/coverage`, and
/// `/runs/:id/integrity`. Merged into `api::router()` under the `/api` prefix.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/integrations/coverage", get(coverage))
        .route("/runs/:id/integrity", get(integrity))
}

// --- Local error handling (self-contained: api.rs's `ApiError` keeps its
//     fields private, and this module intentionally never edits api.rs) ----

struct RouteError {
    status: StatusCode,
    message: String,
}

impl From<anyhow::Error> for RouteError {
    fn from(e: anyhow::Error) -> Self {
        RouteError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: e.to_string(),
        }
    }
}

impl IntoResponse for RouteError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

fn not_found(what: &str) -> RouteError {
    RouteError {
        status: StatusCode::NOT_FOUND,
        message: format!("{what} not found"),
    }
}

type RouteResult<T> = Result<T, RouteError>;

/// Lock the store, recovering the guard if a previous handler panicked while
/// holding it (mirrors `api::store` — see that function's doc comment).
fn store(state: &AppState) -> MutexGuard<'_, Store> {
    state.store.lock().unwrap_or_else(|e| e.into_inner())
}

// --- /health ---------------------------------------------------------------

#[derive(Serialize, Clone)]
struct HealthCheck {
    name: &'static str,
    /// "healthy" | "degraded" | "failed"
    status: &'static str,
    detail: String,
}

fn healthy(name: &'static str, detail: impl Into<String>) -> HealthCheck {
    HealthCheck {
        name,
        status: "healthy",
        detail: detail.into(),
    }
}

fn degraded(name: &'static str, detail: impl Into<String>) -> HealthCheck {
    HealthCheck {
        name,
        status: "degraded",
        detail: detail.into(),
    }
}

fn failed(name: &'static str, detail: impl Into<String>) -> HealthCheck {
    HealthCheck {
        name,
        status: "failed",
        detail: detail.into(),
    }
}

/// Worst-of across every check: any `failed` wins, else any `degraded`, else
/// `healthy`. This is the single top-level `status` field.
fn worst_status(checks: &[HealthCheck]) -> &'static str {
    if checks.iter().any(|c| c.status == "failed") {
        "failed"
    } else if checks.iter().any(|c| c.status == "degraded") {
        "degraded"
    } else {
        "healthy"
    }
}

/// Seconds between an RFC3339 timestamp and now. `None` if the timestamp
/// can't be parsed (never fabricated as 0).
fn seconds_since(ts: &str) -> Option<i64> {
    let parsed = chrono::DateTime::parse_from_rfc3339(ts).ok()?;
    Some(
        (chrono::Utc::now() - parsed.with_timezone(&chrono::Utc))
            .num_seconds()
            .max(0),
    )
}

/// Real check: can we write to and read back from Trace's own home directory?
/// A silent filesystem permission problem (readonly disk, sandboxed home,
/// full disk) shows up here instead of surfacing later as a confusing 500
/// deep inside some unrelated handler.
fn check_filesystem_access() -> HealthCheck {
    match trace_core::paths::ensure_global_dir() {
        Ok(dir) => {
            let probe = dir.join(format!(".health-probe-{}", std::process::id()));
            let result = std::fs::write(&probe, b"ok").and_then(|_| std::fs::remove_file(&probe));
            match result {
                Ok(()) => healthy(
                    "filesystem_access",
                    format!("read/write verified under {}", dir.display()),
                ),
                Err(e) => failed(
                    "filesystem_access",
                    format!("write probe failed under {}: {e}", dir.display()),
                ),
            }
        }
        Err(e) => failed(
            "filesystem_access",
            format!("could not resolve/create the Trace home directory: {e}"),
        ),
    }
}

/// Real check: is `git` on PATH and runnable? Rollback, checkpoints, ratify,
/// and review-diff all shell out to it — if it's missing, those silently
/// degrade rather than erroring loudly, so surface it here.
fn check_git_availability() -> HealthCheck {
    match std::process::Command::new("git").arg("--version").output() {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            healthy("git_availability", version)
        }
        Ok(out) => degraded(
            "git_availability",
            format!("`git --version` exited with status {:?}", out.status.code()),
        ),
        Err(e) => failed("git_availability", format!("git not found on PATH: {e}")),
    }
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let mut checks = Vec::new();

    // 1. daemon — reaching this handler at all is itself the evidence; report
    //    real process/uptime facts alongside it rather than a bare "ok".
    let uptime = seconds_since(&state.started_at)
        .map(|s| format!("{s}s"))
        .unwrap_or_else(|| "unknown".to_string());
    checks.push(healthy(
        "daemon",
        format!(
            "pid {} listening on 127.0.0.1:{}, up {uptime}",
            std::process::id(),
            state.port
        ),
    ));

    // 2. database — a real query against the live connection, not a ping.
    checks.push(match store(&state).list_projects() {
        Ok(projects) => healthy(
            "database",
            format!(
                "{} project(s) queryable at {}",
                projects.len(),
                state.db_path
            ),
        ),
        Err(e) => failed("database", format!("query failed: {e}")),
    });

    // 3. event ingestion — can we read the tables telemetry lands in? Data
    //    freshness itself (last_event_at / ingestion_delay_seconds) is
    //    reported separately below rather than folded into healthy/degraded,
    //    since "no telemetry yet" on a fresh install is not a fault.
    let last_ingested = store(&state).last_ingested_at();
    let (ingestion_check, last_event_at) = match last_ingested {
        Ok(Some(ts)) => (
            healthy("event_ingestion", format!("last telemetry write at {ts}")),
            Some(ts),
        ),
        Ok(None) => (
            degraded(
                "event_ingestion",
                "no telemetry has been recorded yet (fresh install, or no runs since reset)",
            ),
            None,
        ),
        Err(e) => (
            failed("event_ingestion", format!("query failed: {e}")),
            None,
        ),
    };
    checks.push(ingestion_check);

    // 4. integration hooks — at least one agent actually wired up right now.
    let connections = trace_core::integrations::detect_connections();
    let connected_count = connections.iter().filter(|c| c.connected).count();
    checks.push(if connected_count > 0 {
        healthy(
            "integration_hooks",
            format!(
                "{connected_count}/{} agent integration(s) wired on this machine",
                connections.len()
            ),
        )
    } else {
        degraded(
            "integration_hooks",
            "no coding-agent integration is wired into its config on this machine",
        )
    });

    // 5. filesystem access.
    checks.push(check_filesystem_access());

    // 6. git availability.
    checks.push(check_git_availability());

    // 7. dashboard api — the embedded UI bundle is actually present, not just
    //    this JSON API responding.
    checks.push(if crate::assets::dashboard_asset_present() {
        healthy(
            "dashboard_api",
            "API router serving this request; embedded dashboard bundle present",
        )
    } else {
        degraded(
            "dashboard_api",
            "API is serving, but the embedded dashboard bundle (index.html) is missing",
        )
    });

    let overall = worst_status(&checks);
    let ingestion_delay_seconds = last_event_at.as_deref().and_then(seconds_since);

    Json(json!({
        "status": overall,
        "service": "trace-daemon",
        "version": trace_core::VERSION,
        "checks": checks,
        "last_event_at": last_event_at,
        "ingestion_delay_seconds": ingestion_delay_seconds,
    }))
}

// --- /integrations/coverage -------------------------------------------------

/// "observed" when real rows exist, "none" when the check ran but found
/// nothing, never a fabricated percentage.
fn bucket(count: i64) -> serde_json::Value {
    json!({ "status": if count > 0 { "observed" } else { "none" }, "count": count })
}

async fn coverage(State(state): State<AppState>) -> RouteResult<impl IntoResponse> {
    let installed_agents = trace_core::agents::detect_all();

    let mut out = Vec::with_capacity(trace_core::integrations::INTEGRATIONS.len());
    for def in trace_core::integrations::INTEGRATIONS {
        let connected = trace_core::integrations::is_connected(def);
        let installed = installed_agents
            .iter()
            .find(|a| a.id == def.id)
            .map(|a| a.installed)
            .unwrap_or(false);
        let activity = store(&state).agent_activity(def.id)?;

        let status = if !installed && !connected {
            "not_installed"
        } else if !connected {
            "not_connected"
        } else if def.command_enforcement == Some(true) {
            "connected"
        } else {
            "connected_advisory"
        };

        out.push(json!({
            "agent": def.id,
            "display_name": def.display_name,
            "installed": installed,
            "connected": connected,
            "receiving_events": activity.has_activity(),
            "last_event_at": activity.last_activity_at,
            "command_enforcement": def.command_enforcement,
            "file_review": def.file_review,
            "status": status,
            "note": def.capability_note,
            "coverage": {
                "commands": bucket(activity.commands_count),
                "files": bucket(activity.files_count),
                "tests": bucket(activity.tests_count),
                // No table in the schema captures process-tree data for any
                // integration today — honestly "not_instrumented" everywhere
                // rather than a fabricated "none"/"observed".
                "process_tree": { "status": "not_instrumented", "count": 0 },
            },
        }));
    }

    Ok(Json(out))
}

// --- /runs/:id/integrity -----------------------------------------------------

async fn integrity(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> RouteResult<impl IntoResponse> {
    let report = store(&state).integrity_scan(&id)?;
    match report {
        Some(report) => Ok(Json(report)),
        None => Err(not_found("run")),
    }
}

// --- Tests -------------------------------------------------------------------
//
// Offline: in-memory store, no socket bound, no git/filesystem/network
// assumptions baked into the assertions (this machine's actual git/home-dir
// state is not something a unit test should depend on).
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;
    use trace_core::{NewCommand, NewProject, NewRun};

    fn test_state() -> AppState {
        AppState {
            store: Arc::new(Mutex::new(Store::open_in_memory().unwrap())),
            port: 0,
            started_at: "1970-01-01T00:00:00Z".to_string(),
            db_path: ":memory:".to_string(),
        }
    }

    async fn body_json(resp: Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn app(state: AppState) -> axum::Router {
        router().with_state(state)
    }

    #[tokio::test]
    async fn health_route_reports_healthy_status_and_all_seven_checks() {
        let state = test_state();
        let resp = app(state)
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["service"], "trace-daemon");
        assert!(json["status"].is_string());
        let checks = json["checks"].as_array().unwrap();
        assert_eq!(checks.len(), 7);
        let names: Vec<&str> = checks.iter().map(|c| c["name"].as_str().unwrap()).collect();
        for expected in [
            "daemon",
            "database",
            "event_ingestion",
            "integration_hooks",
            "filesystem_access",
            "git_availability",
            "dashboard_api",
        ] {
            assert!(names.contains(&expected), "missing check {expected}");
        }
        // Every check reports one of the three real states, never a made-up one.
        for c in checks {
            let s = c["status"].as_str().unwrap();
            assert!(
                ["healthy", "degraded", "failed"].contains(&s),
                "bad status {s}"
            );
        }
    }

    #[tokio::test]
    async fn health_route_reports_no_telemetry_yet_on_a_fresh_store() {
        let state = test_state();
        let resp = app(state)
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let json = body_json(resp).await;
        assert_eq!(json["last_event_at"], serde_json::Value::Null);
        assert_eq!(json["ingestion_delay_seconds"], serde_json::Value::Null);
        let checks = json["checks"].as_array().unwrap();
        let ingestion = checks
            .iter()
            .find(|c| c["name"] == "event_ingestion")
            .unwrap();
        assert_eq!(ingestion["status"], "degraded");
    }

    #[tokio::test]
    async fn coverage_route_lists_every_integration_with_real_counts() {
        let state = test_state();
        // Seed one "claude" run with a command, so claude's coverage shows
        // real activity while every other agent stays at zero.
        {
            let s = store(&state);
            let project = s
                .upsert_project(&NewProject {
                    name: "P".into(),
                    path: "/p".into(),
                    config_path: "/p/c".into(),
                })
                .unwrap();
            let run = s
                .create_run(&NewRun {
                    project_id: project.id,
                    command: "run".into(),
                    agent_name: Some("claude".into()),
                    user_prompt: None,
                    starting_commit: None,
                })
                .unwrap();
            s.add_command(
                &run.id,
                &NewCommand {
                    command: "ls".into(),
                    decision: "allow".into(),
                    exit_code: Some(0),
                    stdout_path: None,
                    stderr_path: None,
                },
            )
            .unwrap();
        }

        let resp = app(state)
            .oneshot(
                Request::get("/integrations/coverage")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        let rows = json.as_array().unwrap();
        assert_eq!(rows.len(), trace_core::integrations::INTEGRATIONS.len());

        let claude = rows.iter().find(|r| r["agent"] == "claude").unwrap();
        assert_eq!(claude["receiving_events"], true);
        assert_eq!(claude["coverage"]["commands"]["count"], 1);
        assert_eq!(claude["coverage"]["commands"]["status"], "observed");
        // No process-tree table exists for anyone — always honest, never fabricated.
        assert_eq!(
            claude["coverage"]["process_tree"]["status"],
            "not_instrumented"
        );

        let windsurf = rows.iter().find(|r| r["agent"] == "windsurf").unwrap();
        assert_eq!(windsurf["receiving_events"], false);
        assert_eq!(windsurf["coverage"]["commands"]["count"], 0);
        // Windsurf's own architecture never blocks commands — grounded fact,
        // not a live measurement.
        assert_eq!(windsurf["command_enforcement"], false);
    }

    #[tokio::test]
    async fn integrity_route_404s_for_unknown_run_and_200s_for_known() {
        let state = test_state();
        let resp = app(state.clone())
            .oneshot(
                Request::get("/runs/does-not-exist/integrity")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let run_id = {
            let s = store(&state);
            let project = s
                .upsert_project(&NewProject {
                    name: "P".into(),
                    path: "/p".into(),
                    config_path: "/p/c".into(),
                })
                .unwrap();
            s.create_run(&NewRun {
                project_id: project.id,
                command: "run".into(),
                agent_name: None,
                user_prompt: None,
                starting_commit: None,
            })
            .unwrap()
            .id
        };

        let resp = app(state)
            .oneshot(
                Request::get(format!("/runs/{run_id}/integrity"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["ok"], true);
        assert_eq!(json["run_id"], run_id);
    }

    /// Guards against the exact failure mode this route file exists to avoid:
    /// merging `health_routes::router()` into `api::router()` must not panic
    /// on an overlapping `/health` registration. If this test compiles and
    /// passes, the merge in `server.rs::build_router` is safe.
    #[test]
    fn merges_cleanly_with_api_router_without_overlapping_routes() {
        let _combined: axum::Router<AppState> = crate::api::router().merge(router());
    }
}
