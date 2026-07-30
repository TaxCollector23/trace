//! SQLite persistence layer and schema for TraceGuard.
//!
//! The daemon owns the database in normal operation; this module is the single
//! place that knows the schema so the CLI and daemon never diverge.

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::ids::new_id;
use crate::models::*;
use crate::time::now_rfc3339;

/// Embedded schema. Applied idempotently on every open.
const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    path        TEXT NOT NULL UNIQUE,
    config_path TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS runs (
    id              TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL REFERENCES projects(id),
    command         TEXT NOT NULL,
    agent_name      TEXT,
    user_prompt     TEXT,
    started_at      TEXT NOT NULL,
    ended_at        TEXT,
    starting_commit TEXT,
    ending_commit   TEXT,
    status          TEXT NOT NULL,
    exit_code       INTEGER,
    created_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
    id            TEXT PRIMARY KEY,
    run_id        TEXT NOT NULL REFERENCES runs(id),
    type          TEXT NOT NULL,
    message       TEXT NOT NULL,
    metadata_json TEXT,
    created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS file_changes (
    id           TEXT PRIMARY KEY,
    run_id       TEXT NOT NULL REFERENCES runs(id),
    path         TEXT NOT NULL,
    change_type  TEXT NOT NULL,
    diff_summary TEXT,
    created_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS commands (
    id          TEXT PRIMARY KEY,
    run_id      TEXT NOT NULL REFERENCES runs(id),
    command     TEXT NOT NULL,
    decision    TEXT NOT NULL,
    exit_code   INTEGER,
    stdout_path TEXT,
    stderr_path TEXT,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS secrets (
    id            TEXT PRIMARY KEY,
    run_id        TEXT NOT NULL REFERENCES runs(id),
    file_path     TEXT,
    secret_type   TEXT NOT NULL,
    redacted_value TEXT NOT NULL,
    action_taken  TEXT NOT NULL,
    created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS api_usage (
    id             TEXT PRIMARY KEY,
    run_id         TEXT NOT NULL REFERENCES runs(id),
    provider       TEXT NOT NULL,
    model          TEXT NOT NULL,
    input_tokens   INTEGER,
    output_tokens  INTEGER,
    cached_tokens  INTEGER,
    estimated_cost REAL,
    latency_ms     INTEGER,
    created_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS checkpoints (
    id              TEXT PRIMARY KEY,
    run_id          TEXT NOT NULL REFERENCES runs(id),
    project_id      TEXT NOT NULL REFERENCES projects(id),
    git_ref         TEXT,
    checkpoint_type TEXT NOT NULL,
    created_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS test_results (
    id             TEXT PRIMARY KEY,
    run_id         TEXT NOT NULL REFERENCES runs(id),
    command        TEXT NOT NULL,
    status         TEXT NOT NULL,
    output_summary TEXT,
    created_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS prompt_compressions (
    id                          TEXT PRIMARY KEY,
    run_id                      TEXT REFERENCES runs(id),
    project_id                  TEXT,
    mode                        TEXT NOT NULL,
    original_token_estimate     INTEGER NOT NULL,
    compressed_token_estimate   INTEGER NOT NULL,
    estimated_reduction_percent REAL NOT NULL,
    compressed_prompt_hash      TEXT NOT NULL,
    original_prompt_stored      INTEGER NOT NULL,
    compressed_prompt_stored    INTEGER NOT NULL,
    preserved_constraints_json  TEXT,
    removed_redundancy_json     TEXT,
    original_prompt             TEXT,
    compressed_prompt           TEXT,
    created_at                  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_runs_project ON runs(project_id);
CREATE INDEX IF NOT EXISTS idx_prompt_compressions_run ON prompt_compressions(run_id);
CREATE INDEX IF NOT EXISTS idx_events_run ON events(run_id);
CREATE INDEX IF NOT EXISTS idx_file_changes_run ON file_changes(run_id);
CREATE INDEX IF NOT EXISTS idx_commands_run ON commands(run_id);
CREATE INDEX IF NOT EXISTS idx_secrets_run ON secrets(run_id);
CREATE INDEX IF NOT EXISTS idx_api_usage_run ON api_usage(run_id);
CREATE INDEX IF NOT EXISTS idx_checkpoints_run ON checkpoints(run_id);
CREATE INDEX IF NOT EXISTS idx_test_results_run ON test_results(run_id);
"#;

/// Thin wrapper around a SQLite connection that exposes typed operations.
pub struct Store {
    conn: Connection,
}

/// Apply schema migrations that `CREATE TABLE IF NOT EXISTS` cannot express.
///
/// The `prompt_compressions` table gained columns for TraceCompress. If an older
/// database has the legacy shape (no `mode` column), drop and recreate it — the
/// only data lost is local compression history, which is non-critical.
fn migrate(conn: &Connection) -> Result<()> {
    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='prompt_compressions'",
            [],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);

    if table_exists {
        let has_mode: bool = conn
            .prepare("PRAGMA table_info(prompt_compressions)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(|r| r.ok())
            .any(|col| col == "mode");
        if !has_mode {
            conn.execute_batch("DROP TABLE prompt_compressions;")
                .context("dropping legacy prompt_compressions table")?;
        }
    }
    Ok(())
}

impl Store {
    /// Open (creating if needed) the database at `path` and apply the schema.
    pub fn open(path: &std::path::Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating db dir {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("opening sqlite db {}", path.display()))?;
        migrate(&conn)?;
        conn.execute_batch(SCHEMA).context("applying schema")?;
        Ok(Store { conn })
    }

    /// Open an in-memory database (used in tests).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        migrate(&conn)?;
        conn.execute_batch(SCHEMA).context("applying schema")?;
        Ok(Store { conn })
    }

    // --- Projects ---------------------------------------------------------

    /// Insert a project, or update its name/config/timestamp if the path exists.
    pub fn upsert_project(&self, new: &NewProject) -> Result<Project> {
        let now = now_rfc3339();
        if let Some(existing) = self.project_by_path(&new.path)? {
            self.conn.execute(
                "UPDATE projects SET name = ?1, config_path = ?2, updated_at = ?3 WHERE id = ?4",
                params![new.name, new.config_path, now, existing.id],
            )?;
            return self
                .project_by_id(&existing.id)?
                .context("project vanished after update");
        }
        let id = new_id();
        self.conn.execute(
            "INSERT INTO projects (id, name, path, config_path, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, new.name, new.path, new.config_path, now],
        )?;
        self.project_by_id(&id)?
            .context("project vanished after insert")
    }

    pub fn project_by_id(&self, id: &str) -> Result<Option<Project>> {
        self.conn
            .query_row(
                "SELECT id, name, path, config_path, created_at, updated_at FROM projects WHERE id = ?1",
                params![id],
                map_project,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn project_by_path(&self, path: &str) -> Result<Option<Project>> {
        self.conn
            .query_row(
                "SELECT id, name, path, config_path, created_at, updated_at FROM projects WHERE path = ?1",
                params![path],
                map_project,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, config_path, created_at, updated_at FROM projects ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], map_project)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // --- Runs -------------------------------------------------------------

    pub fn create_run(&self, new: &NewRun) -> Result<Run> {
        let id = new_id();
        let now = now_rfc3339();
        self.conn.execute(
            "INSERT INTO runs (id, project_id, command, agent_name, user_prompt, started_at,
                ending_commit, starting_commit, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?6)",
            params![
                id,
                new.project_id,
                new.command,
                new.agent_name,
                new.user_prompt,
                now,
                new.starting_commit,
                RunStatus::Running.as_str(),
            ],
        )?;
        self.run_by_id(&id)?.context("run vanished after insert")
    }

    pub fn run_by_id(&self, id: &str) -> Result<Option<Run>> {
        self.conn
            .query_row(
                "SELECT id, project_id, command, agent_name, user_prompt, started_at, ended_at,
                    starting_commit, ending_commit, status, exit_code, created_at
                 FROM runs WHERE id = ?1",
                params![id],
                map_run,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_runs(&self, limit: i64) -> Result<Vec<Run>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, command, agent_name, user_prompt, started_at, ended_at,
                starting_commit, ending_commit, status, exit_code, created_at
             FROM runs ORDER BY started_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], map_run)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Finalize a run with status, exit code, and ending commit.
    pub fn finish_run(
        &self,
        run_id: &str,
        status: RunStatus,
        exit_code: Option<i64>,
        ending_commit: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE runs SET status = ?1, exit_code = ?2, ending_commit = ?3, ended_at = ?4 WHERE id = ?5",
            params![status.as_str(), exit_code, ending_commit, now_rfc3339(), run_id],
        )?;
        Ok(())
    }

    pub fn set_run_status(&self, run_id: &str, status: RunStatus) -> Result<()> {
        self.conn.execute(
            "UPDATE runs SET status = ?1 WHERE id = ?2",
            params![status.as_str(), run_id],
        )?;
        Ok(())
    }

    // --- Events -----------------------------------------------------------

    pub fn add_event(&self, run_id: &str, new: &NewEvent) -> Result<Event> {
        let id = new_id();
        let now = now_rfc3339();
        self.conn.execute(
            "INSERT INTO events (id, run_id, type, message, metadata_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                run_id,
                new.event_type,
                new.message,
                new.metadata_json,
                now
            ],
        )?;
        Ok(Event {
            id,
            run_id: run_id.to_string(),
            event_type: new.event_type.clone(),
            message: new.message.clone(),
            metadata_json: new.metadata_json.clone(),
            created_at: now,
        })
    }

    pub fn list_events(&self, run_id: &str) -> Result<Vec<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, type, message, metadata_json, created_at
             FROM events WHERE run_id = ?1 ORDER BY created_at ASC, rowid ASC",
        )?;
        let rows = stmt.query_map(params![run_id], map_event)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // --- File changes -----------------------------------------------------

    pub fn add_file_change(&self, run_id: &str, new: &NewFileChange) -> Result<()> {
        self.conn.execute(
            "INSERT INTO file_changes (id, run_id, path, change_type, diff_summary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                new_id(),
                run_id,
                new.path,
                new.change_type,
                new.diff_summary,
                now_rfc3339()
            ],
        )?;
        Ok(())
    }

    /// Replace all recorded file changes for a run (final diff is source of truth).
    pub fn replace_file_changes(&self, run_id: &str, changes: &[NewFileChange]) -> Result<()> {
        self.conn.execute(
            "DELETE FROM file_changes WHERE run_id = ?1",
            params![run_id],
        )?;
        for c in changes {
            self.add_file_change(run_id, c)?;
        }
        Ok(())
    }

    pub fn list_file_changes(&self, run_id: &str) -> Result<Vec<FileChange>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, path, change_type, diff_summary, created_at
             FROM file_changes WHERE run_id = ?1 ORDER BY path ASC",
        )?;
        let rows = stmt.query_map(params![run_id], map_file_change)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // --- Commands ---------------------------------------------------------

    pub fn add_command(&self, run_id: &str, new: &NewCommand) -> Result<()> {
        self.conn.execute(
            "INSERT INTO commands (id, run_id, command, decision, exit_code, stdout_path, stderr_path, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![new_id(), run_id, new.command, new.decision, new.exit_code, new.stdout_path, new.stderr_path, now_rfc3339()],
        )?;
        Ok(())
    }

    pub fn list_commands(&self, run_id: &str) -> Result<Vec<CommandRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, command, decision, exit_code, stdout_path, stderr_path, created_at
             FROM commands WHERE run_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![run_id], map_command)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // --- Secrets ----------------------------------------------------------

    pub fn add_secret(&self, run_id: &str, new: &NewSecret) -> Result<()> {
        self.conn.execute(
            "INSERT INTO secrets (id, run_id, file_path, secret_type, redacted_value, action_taken, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![new_id(), run_id, new.file_path, new.secret_type, new.redacted_value, new.action_taken, now_rfc3339()],
        )?;
        Ok(())
    }

    pub fn list_secrets(&self, run_id: &str) -> Result<Vec<SecretRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, file_path, secret_type, redacted_value, action_taken, created_at
             FROM secrets WHERE run_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![run_id], map_secret)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // --- API usage --------------------------------------------------------

    pub fn add_api_usage(&self, run_id: &str, new: &NewApiUsage) -> Result<()> {
        self.conn.execute(
            "INSERT INTO api_usage (id, run_id, provider, model, input_tokens, output_tokens, cached_tokens, estimated_cost, latency_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![new_id(), run_id, new.provider, new.model, new.input_tokens, new.output_tokens, new.cached_tokens, new.estimated_cost, new.latency_ms, now_rfc3339()],
        )?;
        Ok(())
    }

    pub fn list_api_usage(&self, run_id: &str) -> Result<Vec<ApiUsage>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, provider, model, input_tokens, output_tokens, cached_tokens, estimated_cost, latency_ms, created_at
             FROM api_usage WHERE run_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![run_id], map_api_usage)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // --- Checkpoints ------------------------------------------------------

    pub fn add_checkpoint(&self, run_id: &str, new: &NewCheckpoint) -> Result<Checkpoint> {
        let id = new_id();
        let now = now_rfc3339();
        self.conn.execute(
            "INSERT INTO checkpoints (id, run_id, project_id, git_ref, checkpoint_type, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                run_id,
                new.project_id,
                new.git_ref,
                new.checkpoint_type,
                now
            ],
        )?;
        Ok(Checkpoint {
            id,
            run_id: run_id.to_string(),
            project_id: new.project_id.clone(),
            git_ref: new.git_ref.clone(),
            checkpoint_type: new.checkpoint_type.clone(),
            created_at: now,
        })
    }

    pub fn list_checkpoints(&self, run_id: &str) -> Result<Vec<Checkpoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, project_id, git_ref, checkpoint_type, created_at
             FROM checkpoints WHERE run_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![run_id], map_checkpoint)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// All checkpoints across runs, most recent first (for the rollback center).
    pub fn recent_checkpoints(&self, limit: i64) -> Result<Vec<Checkpoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, project_id, git_ref, checkpoint_type, created_at
             FROM checkpoints ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], map_checkpoint)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // --- Test results -----------------------------------------------------

    pub fn add_test_result(&self, run_id: &str, new: &NewTestResult) -> Result<()> {
        self.conn.execute(
            "INSERT INTO test_results (id, run_id, command, status, output_summary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                new_id(),
                run_id,
                new.command,
                new.status,
                new.output_summary,
                now_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn list_test_results(&self, run_id: &str) -> Result<Vec<TestResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, command, status, output_summary, created_at
             FROM test_results WHERE run_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![run_id], map_test_result)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // --- Prompt compressions ---------------------------------------------

    pub fn add_prompt_compression(&self, new: &NewPromptCompression) -> Result<PromptCompression> {
        let id = new_id();
        let now = now_rfc3339();
        self.conn.execute(
            "INSERT INTO prompt_compressions
                (id, run_id, project_id, mode, original_token_estimate, compressed_token_estimate,
                 estimated_reduction_percent, compressed_prompt_hash, original_prompt_stored,
                 compressed_prompt_stored, preserved_constraints_json, removed_redundancy_json,
                 original_prompt, compressed_prompt, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                id,
                new.run_id,
                new.project_id,
                new.mode,
                new.original_token_estimate,
                new.compressed_token_estimate,
                new.estimated_reduction_percent,
                new.compressed_prompt_hash,
                new.original_prompt_stored,
                new.compressed_prompt_stored,
                new.preserved_constraints_json,
                new.removed_redundancy_json,
                new.original_prompt,
                new.compressed_prompt,
                now
            ],
        )?;
        self.prompt_compression_by_id(&id)?
            .context("compression vanished after insert")
    }

    pub fn list_prompt_compressions(&self, limit: i64) -> Result<Vec<PromptCompression>> {
        let mut stmt = self.conn.prepare(&format!(
            "{PROMPT_COMPRESSION_SELECT} ORDER BY created_at DESC LIMIT ?1"
        ))?;
        let rows = stmt.query_map(params![limit], map_prompt_compression)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn prompt_compression_by_id(&self, id: &str) -> Result<Option<PromptCompression>> {
        self.conn
            .query_row(
                &format!("{PROMPT_COMPRESSION_SELECT} WHERE id = ?1"),
                params![id],
                map_prompt_compression,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn delete_prompt_compression(&self, id: &str) -> Result<bool> {
        let n = self
            .conn
            .execute("DELETE FROM prompt_compressions WHERE id = ?1", params![id])?;
        Ok(n > 0)
    }

    // --- Aggregates -------------------------------------------------------

    /// Build a summary (counts + cost) for a run card.
    pub fn run_summary(&self, run: &Run) -> Result<RunSummary> {
        let project_name = self
            .project_by_id(&run.project_id)?
            .map(|p| p.name)
            .unwrap_or_else(|| "Unknown project".to_string());

        let files_changed: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM file_changes WHERE run_id = ?1",
            params![run.id],
            |r| r.get(0),
        )?;
        let command_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM commands WHERE run_id = ?1",
            params![run.id],
            |r| r.get(0),
        )?;
        let secret_warnings: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM secrets WHERE run_id = ?1",
            params![run.id],
            |r| r.get(0),
        )?;
        let estimated_cost: Option<f64> = self.conn.query_row(
            "SELECT SUM(estimated_cost) FROM api_usage WHERE run_id = ?1",
            params![run.id],
            |r| r.get(0),
        )?;
        let checks_status: Option<String> = self.conn.query_row(
            "SELECT status FROM test_results WHERE run_id = ?1 ORDER BY created_at DESC LIMIT 1",
            params![run.id],
            |r| r.get(0),
        ).optional()?;

        Ok(RunSummary {
            run: run.clone(),
            project_name,
            files_changed,
            command_count,
            secret_warnings,
            estimated_cost,
            checks_status,
        })
    }

    /// Summaries for the most recent runs.
    pub fn recent_run_summaries(&self, limit: i64) -> Result<Vec<RunSummary>> {
        let runs = self.list_runs(limit)?;
        runs.iter().map(|r| self.run_summary(r)).collect()
    }
}

// --- Row mappers ----------------------------------------------------------

fn map_project(row: &rusqlite::Row) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        path: row.get(2)?,
        config_path: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn map_run(row: &rusqlite::Row) -> rusqlite::Result<Run> {
    Ok(Run {
        id: row.get(0)?,
        project_id: row.get(1)?,
        command: row.get(2)?,
        agent_name: row.get(3)?,
        user_prompt: row.get(4)?,
        started_at: row.get(5)?,
        ended_at: row.get(6)?,
        starting_commit: row.get(7)?,
        ending_commit: row.get(8)?,
        status: row.get(9)?,
        exit_code: row.get(10)?,
        created_at: row.get(11)?,
    })
}

fn map_event(row: &rusqlite::Row) -> rusqlite::Result<Event> {
    Ok(Event {
        id: row.get(0)?,
        run_id: row.get(1)?,
        event_type: row.get(2)?,
        message: row.get(3)?,
        metadata_json: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn map_file_change(row: &rusqlite::Row) -> rusqlite::Result<FileChange> {
    Ok(FileChange {
        id: row.get(0)?,
        run_id: row.get(1)?,
        path: row.get(2)?,
        change_type: row.get(3)?,
        diff_summary: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn map_command(row: &rusqlite::Row) -> rusqlite::Result<CommandRecord> {
    Ok(CommandRecord {
        id: row.get(0)?,
        run_id: row.get(1)?,
        command: row.get(2)?,
        decision: row.get(3)?,
        exit_code: row.get(4)?,
        stdout_path: row.get(5)?,
        stderr_path: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn map_secret(row: &rusqlite::Row) -> rusqlite::Result<SecretRecord> {
    Ok(SecretRecord {
        id: row.get(0)?,
        run_id: row.get(1)?,
        file_path: row.get(2)?,
        secret_type: row.get(3)?,
        redacted_value: row.get(4)?,
        action_taken: row.get(5)?,
        created_at: row.get(6)?,
    })
}

fn map_api_usage(row: &rusqlite::Row) -> rusqlite::Result<ApiUsage> {
    Ok(ApiUsage {
        id: row.get(0)?,
        run_id: row.get(1)?,
        provider: row.get(2)?,
        model: row.get(3)?,
        input_tokens: row.get(4)?,
        output_tokens: row.get(5)?,
        cached_tokens: row.get(6)?,
        estimated_cost: row.get(7)?,
        latency_ms: row.get(8)?,
        created_at: row.get(9)?,
    })
}

fn map_checkpoint(row: &rusqlite::Row) -> rusqlite::Result<Checkpoint> {
    Ok(Checkpoint {
        id: row.get(0)?,
        run_id: row.get(1)?,
        project_id: row.get(2)?,
        git_ref: row.get(3)?,
        checkpoint_type: row.get(4)?,
        created_at: row.get(5)?,
    })
}

const PROMPT_COMPRESSION_SELECT: &str = "SELECT id, run_id, project_id, mode, original_token_estimate, \
    compressed_token_estimate, estimated_reduction_percent, compressed_prompt_hash, \
    original_prompt_stored, compressed_prompt_stored, preserved_constraints_json, \
    removed_redundancy_json, original_prompt, compressed_prompt, created_at FROM prompt_compressions";

fn map_prompt_compression(row: &rusqlite::Row) -> rusqlite::Result<PromptCompression> {
    Ok(PromptCompression {
        id: row.get(0)?,
        run_id: row.get(1)?,
        project_id: row.get(2)?,
        mode: row.get(3)?,
        original_token_estimate: row.get(4)?,
        compressed_token_estimate: row.get(5)?,
        estimated_reduction_percent: row.get(6)?,
        compressed_prompt_hash: row.get(7)?,
        original_prompt_stored: row.get(8)?,
        compressed_prompt_stored: row.get(9)?,
        preserved_constraints_json: row.get(10)?,
        removed_redundancy_json: row.get(11)?,
        original_prompt: row.get(12)?,
        compressed_prompt: row.get(13)?,
        created_at: row.get(14)?,
    })
}

fn map_test_result(row: &rusqlite::Row) -> rusqlite::Result<TestResult> {
    Ok(TestResult {
        id: row.get(0)?,
        run_id: row.get(1)?,
        command: row.get(2)?,
        status: row.get(3)?,
        output_summary: row.get(4)?,
        created_at: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_run_event_roundtrip() {
        let store = Store::open_in_memory().unwrap();
        let project = store
            .upsert_project(&NewProject {
                name: "My App".into(),
                path: "/tmp/app".into(),
                config_path: "/tmp/app/.traceguard/config.toml".into(),
            })
            .unwrap();

        let run = store
            .create_run(&NewRun {
                project_id: project.id.clone(),
                command: "npm test".into(),
                agent_name: None,
                user_prompt: None,
                starting_commit: Some("abc123".into()),
            })
            .unwrap();
        assert_eq!(run.status, "running");

        store
            .add_event(
                &run.id,
                &NewEvent {
                    event_type: "run_created".into(),
                    message: "created".into(),
                    metadata_json: None,
                },
            )
            .unwrap();
        assert_eq!(store.list_events(&run.id).unwrap().len(), 1);

        store
            .finish_run(&run.id, RunStatus::Completed, Some(0), Some("def456"))
            .unwrap();
        let summary = store
            .run_summary(&store.run_by_id(&run.id).unwrap().unwrap())
            .unwrap();
        assert_eq!(summary.project_name, "My App");
        assert_eq!(summary.run.status, "completed");
    }

    #[test]
    fn upsert_project_is_idempotent_by_path() {
        let store = Store::open_in_memory().unwrap();
        let a = store
            .upsert_project(&NewProject {
                name: "A".into(),
                path: "/p".into(),
                config_path: "/p/c".into(),
            })
            .unwrap();
        let b = store
            .upsert_project(&NewProject {
                name: "B".into(),
                path: "/p".into(),
                config_path: "/p/c".into(),
            })
            .unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(b.name, "B");
        assert_eq!(store.list_projects().unwrap().len(), 1);
    }
}
