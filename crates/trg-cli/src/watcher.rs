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
use traceguard_core::models::EventType;

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
    ".traceguard",
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
    let body = traceguard_core::models::NewEvent {
        event_type: event_type.as_str().to_string(),
        message: format!("{} {}", verb(event_type), rel),
        metadata_json: Some(format!("{{\"path\":{:?}}}", rel)),
    };
    // Best-effort: a dropped timeline event must never break a run.
    let _ = client.post(&format!("/api/runs/{run_id}/events"), &body);
}

fn verb(event_type: EventType) -> &'static str {
    match event_type {
        EventType::FileCreated => "created",
        EventType::FileDeleted => "deleted",
        _ => "modified",
    }
}
