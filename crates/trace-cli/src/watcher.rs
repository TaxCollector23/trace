//! Project file watcher used during a run.
//!
//! The watcher provides timeline detail (created/modified/deleted events). The
//! final git diff remains the source of truth for the patch review — see
//! `run.rs`. Events are debounced per path so a burst of editor saves does not
//! spam the database.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecursiveMode, Watcher};
use trace_core::models::EventType;

use crate::client::Client;

/// Folders that are noisy or generated and should never be reported.
const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    ".next",
    "coverage",
    "vendor",
    ".turbo",
    ".cache",
    "__pycache__",
    ".venv",
    "venv",
    ".trace",
];

fn is_ignored(path: &Path, root: &Path) -> bool {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map(|s| IGNORED_DIRS.contains(&s))
            .unwrap_or(false)
    })
}

fn classify_kind(kind: &EventKind) -> Option<EventType> {
    match kind {
        EventKind::Create(_) => Some(EventType::FileCreated),
        EventKind::Modify(_) => Some(EventType::FileModified),
        EventKind::Remove(_) => Some(EventType::FileDeleted),
        _ => None,
    }
}

/// A running watcher. Dropping/`stop()` ends the background thread.
pub struct RunWatcher {
    _watcher: notify::RecommendedWatcher,
    handle: Option<JoinHandle<()>>,
    stop_tx: std::sync::mpsc::Sender<()>,
}

impl RunWatcher {
    /// Start watching `root`, posting debounced timeline events for `run_id`.
    pub fn start(root: PathBuf, client: Arc<Client>, run_id: String) -> notify::Result<Self> {
        let (event_tx, event_rx) = channel::<Event>();
        let (stop_tx, stop_rx) = channel::<()>();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = event_tx.send(event);
            }
        })?;
        watcher.watch(&root, RecursiveMode::Recursive)?;

        let handle = std::thread::spawn(move || {
            debounce_loop(root, client, run_id, event_rx, stop_rx);
        });

        Ok(RunWatcher {
            _watcher: watcher,
            handle: Some(handle),
            stop_tx,
        })
    }

    /// Stop the watcher and flush any pending debounced events.
    pub fn stop(mut self) {
        let _ = self.stop_tx.send(());
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Debounce window: collapse repeated events on the same path within this span.
const DEBOUNCE: Duration = Duration::from_millis(600);

fn debounce_loop(
    root: PathBuf,
    client: Arc<Client>,
    run_id: String,
    event_rx: Receiver<Event>,
    stop_rx: Receiver<()>,
) {
    // path -> (event type, last time we recorded it)
    let mut last_emit: HashMap<PathBuf, Instant> = HashMap::new();

    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }
        match event_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(event) => {
                let Some(event_type) = classify_kind(&event.kind) else {
                    continue;
                };
                for path in event.paths {
                    if is_ignored(&path, &root) {
                        continue;
                    }
                    let now = Instant::now();
                    if let Some(prev) = last_emit.get(&path) {
                        if now.duration_since(*prev) < DEBOUNCE {
                            continue;
                        }
                    }
                    last_emit.insert(path.clone(), now);
                    emit(&client, &run_id, event_type, &root, &path);
                }
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn emit(client: &Client, run_id: &str, event_type: EventType, root: &Path, path: &Path) {
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string();
    let body = trace_core::models::NewEvent {
        event_type: event_type.as_str().to_string(),
        message: format!("{} {}", verb(event_type), rel),
        metadata_json: Some(serde_json::json!({ "path": rel }).to_string()),
    };
    // Best-effort: a dropped timeline event must never break a run.
    let _ = client.post(&format!("/api/runs/{run_id}/events"), &body);

    // Live review, for every wrapped agent — not just Claude Code's hooks.
    // This is the CLI-wrapper path's equivalent of trace-hook.sh's
    // PostToolUse call: same daemon endpoint, same deterministic policy
    // engine. What it can't do that the Claude Code hook *can* is inject
    // feedback into the agent's own context — Trace is watching the
    // filesystem from outside here, not sitting in the tool-call path — so a
    // "block" response becomes a loud terminal alert for the human at the
    // keyboard plus a dashboard flag, not a message the agent itself sees.
    // See ARCHITECTURE.md.
    if matches!(event_type, EventType::FileCreated | EventType::FileModified) {
        review_live(client, run_id, path, &rel);
    }
}

/// Cap on what gets sent for a live review pass — this runs on every
/// debounced file-save, so it needs to stay cheap. Binary/huge files are
/// skipped entirely rather than truncated silently mid-content.
const MAX_REVIEW_BYTES: u64 = 200 * 1024;

fn review_live(client: &Client, run_id: &str, path: &Path, rel: &str) {
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    if !meta.is_file() || meta.len() == 0 || meta.len() > MAX_REVIEW_BYTES {
        return;
    }
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    }; // binary or non-UTF8: skip

    // The policy engine's checks match on unified-diff-style added lines
    // (see policy.rs::added_lines). The watcher only has "the file's
    // current content", not a real diff against what it looked like a
    // moment ago — treating every line as "added" is an approximation, but
    // a correct one for this engine's checks (secrets, TODOs, swallowed
    // catches): it's asking "does a line that currently exists in this
    // file match a risky pattern", and that's true regardless of whether
    // the line is new in this exact debounce window.
    let synthetic_patch: String = content.lines().map(|l| format!("+{l}\n")).collect();

    #[derive(serde::Serialize)]
    struct HookCheckBody<'a> {
        tool_name: &'a str,
        file_path: &'a str,
        diff_summary: String,
    }
    #[derive(serde::Deserialize)]
    struct HookCheckResp {
        block: bool,
        message: Option<String>,
    }

    let body = HookCheckBody {
        tool_name: "watcher",
        file_path: rel,
        diff_summary: synthetic_patch,
    };
    let resp: Result<HookCheckResp, _> =
        client.post_json(&format!("/api/runs/{run_id}/hook-check"), &body);
    if let Ok(HookCheckResp {
        block: true,
        message: Some(msg),
    }) = resp
    {
        eprintln!(
            "\n{}\n  {}\n",
            crate::colors::yellow(&format!("⚠ Trace flagged a change in {rel}:")),
            msg
        );
    }
    // Any other outcome (allowed, judge disabled, request failed) is
    // silent here by design — findings still landed via add_policy_findings
    // on the daemon side and show up on the dashboard either way.
}

fn verb(event_type: EventType) -> &'static str {
    match event_type {
        EventType::FileCreated => "created",
        EventType::FileDeleted => "deleted",
        _ => "modified",
    }
}
