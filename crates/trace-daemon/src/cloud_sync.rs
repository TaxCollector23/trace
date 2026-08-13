//! Opt-in cloud sync: POST a completed run + its events to trace-cloud-api
//! so the hosted dashboard can show it.
//!
//! Configuration (env vars, no on-disk config yet):
//!   TRACE_CLOUD_URL    e.g. "https://trace-cloud-api.onrender.com"
//!   TRACE_CLOUD_TOKEN  opaque bearer token, paste from web dashboard
//!
//! Both unset ⇒ sync is a no-op and no network call is made. Set both ⇒
//! every completed run is fire-and-forget POSTed after `finish_run`.
//!
//! Design constraints:
//!   - **Never blocks the run.** Sync runs on a spawn_blocking task after
//!     the caller has already gotten its 200 back — a slow or dead backend
//!     never delays the local daemon or the CLI's exit.
//!   - **Best effort.** A failed sync logs at INFO and moves on; there's
//!     no retry queue yet. Losing telemetry on the cloud side is strictly
//!     better than delaying the user's local workflow.
//!   - **Privacy first.** Only run metadata + event summaries are sent —
//!     never file contents, never diffs, never prompts unless the user
//!     supplied them via `--user-prompt` explicitly.

use serde::Serialize;
use serde_json::json;
use trace_core::{Event, Run, Store};

#[derive(Debug, Serialize)]
struct CloudEvent<'a> {
    event_type: &'a str,
    message: &'a str,
    metadata_json: Option<&'a str>,
    created_at: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct CloudRun<'a> {
    id: &'a str,
    project_name: &'a str,
    agent_name: Option<&'a str>,
    command: &'a str,
    user_prompt: Option<&'a str>,
    status: &'a str,
    exit_code: Option<i64>,
    created_at: &'a str,
    completed_at: Option<&'a str>,
    event_count: i64,
}

/// Enqueue a sync on a background thread. Returns immediately.
pub fn enqueue(run_id: String, store: std::sync::Arc<std::sync::Mutex<Store>>) {
    let (url, token) = match (
        std::env::var("TRACE_CLOUD_URL"),
        std::env::var("TRACE_CLOUD_TOKEN"),
    ) {
        (Ok(u), Ok(t)) if !u.is_empty() && !t.is_empty() => (u, t),
        _ => return, // opt-in — silent no-op when not configured
    };
    std::thread::spawn(move || {
        if let Err(e) = sync_one(&url, &token, &run_id, &store) {
            tracing::info!("cloud sync skipped for {run_id}: {e}");
        }
    });
}

fn sync_one(
    url: &str,
    token: &str,
    run_id: &str,
    store: &std::sync::Mutex<Store>,
) -> anyhow::Result<()> {
    let (run, events, project_name) = {
        let s = store.lock().expect("store mutex poisoned");
        let run: Run = s
            .run_by_id(run_id)?
            .ok_or_else(|| anyhow::anyhow!("run {run_id} not found in store"))?;
        let events: Vec<Event> = s.list_events(run_id)?;
        // Best-effort project name lookup — fall back to "(unknown)" so a
        // stale project id doesn't kill the sync.
        let project_name = s
            .project_by_id(&run.project_id)?
            .map(|p| p.name)
            .unwrap_or_else(|| "(unknown)".into());
        (run, events, project_name)
    };

    let cloud_events: Vec<CloudEvent> = events
        .iter()
        .map(|e| CloudEvent {
            event_type: &e.event_type,
            message: &e.message,
            metadata_json: e.metadata_json.as_deref(),
            created_at: Some(&e.created_at),
        })
        .collect();

    let cloud_run = CloudRun {
        id: &run.id,
        project_name: &project_name,
        agent_name: run.agent_name.as_deref(),
        command: &run.command,
        user_prompt: run.user_prompt.as_deref(),
        status: &run.status,
        exit_code: None, // trace-core Run doesn't carry exit_code; the finish payload does
        created_at: &run.started_at,
        completed_at: run.ended_at.as_deref(),
        event_count: events.len() as i64,
    };

    let body = json!({ "run": cloud_run, "events": cloud_events });
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build();
    let endpoint = format!("{}/v1/runs", url.trim_end_matches('/'));
    let resp = agent
        .post(&endpoint)
        .set("Authorization", &format!("Bearer {token}"))
        .set("content-type", "application/json")
        .send_json(body);

    match resp {
        Ok(_) => {
            tracing::info!("cloud sync ok: {run_id}");
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("POST {endpoint} failed: {e}")),
    }
}
