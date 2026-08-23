//! SQLite storage layer for the cloud API.
//!
//! Schema is intentionally minimal — a run row, an events row per timeline
//! entry, and a users row for token → user_id lookup. No migrations
//! framework: we `CREATE TABLE IF NOT EXISTS` on every open so a Render
//! cold-start on an empty disk always works.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

pub struct Store {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RunSummary {
    pub id: String,
    pub project_name: String,
    pub agent_name: Option<String>,
    pub command: String,
    pub user_prompt: Option<String>,
    pub status: String,
    pub exit_code: Option<i64>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub event_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CloudEvent {
    pub event_type: String,
    pub message: String,
    pub metadata_json: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RunUpload {
    pub run: RunSummary,
    pub events: Vec<CloudEvent>,
}

impl Store {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path).with_context(|| format!("opening {path}"))?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                user_id TEXT PRIMARY KEY,
                token   TEXT UNIQUE NOT NULL,
                email   TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS runs (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                project_name TEXT NOT NULL,
                agent_name TEXT,
                command TEXT NOT NULL,
                user_prompt TEXT,
                status TEXT NOT NULL,
                exit_code INTEGER,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                FOREIGN KEY (user_id) REFERENCES users(user_id)
            );
            CREATE INDEX IF NOT EXISTS runs_user_idx ON runs(user_id, created_at DESC);
            CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                message TEXT NOT NULL,
                metadata_json TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (run_id) REFERENCES runs(id)
            );
            CREATE INDEX IF NOT EXISTS events_run_idx ON events(run_id, id);
            "#,
        )?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    /// Look up or create a user by opaque bearer token. Kept simple on
    /// purpose: users provision a token in `trc daemon cloud-login`
    /// (future) or paste one from the web dashboard. First use registers
    /// the token → user_id binding.
    pub fn upsert_user_by_token(&self, token: &str) -> Result<String> {
        let c = self.conn.lock().unwrap();
        if let Ok(uid) = c.query_row(
            "SELECT user_id FROM users WHERE token = ?1",
            params![token],
            |r| r.get::<_, String>(0),
        ) {
            return Ok(uid);
        }
        let uid = uuid::Uuid::new_v4().to_string();
        c.execute(
            "INSERT INTO users (user_id, token) VALUES (?1, ?2)",
            params![uid, token],
        )?;
        Ok(uid)
    }

    pub fn insert_run(&self, user_id: &str, upload: &RunUpload) -> Result<()> {
        let mut c = self.conn.lock().unwrap();
        let tx = c.transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO runs
              (id, user_id, project_name, agent_name, command, user_prompt, status, exit_code, created_at, completed_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                upload.run.id,
                user_id,
                upload.run.project_name,
                upload.run.agent_name,
                upload.run.command,
                upload.run.user_prompt,
                upload.run.status,
                upload.run.exit_code,
                upload.run.created_at,
                upload.run.completed_at,
            ],
        )?;
        tx.execute(
            "DELETE FROM events WHERE run_id = ?1",
            params![upload.run.id],
        )?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO events (run_id, event_type, message, metadata_json, created_at)
                 VALUES (?1,?2,?3,?4, COALESCE(?5, datetime('now')))",
            )?;
            for e in &upload.events {
                stmt.execute(params![
                    upload.run.id,
                    e.event_type,
                    e.message,
                    e.metadata_json,
                    e.created_at
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_runs(&self, user_id: &str, limit: usize) -> Result<Vec<RunSummary>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT r.id, r.project_name, r.agent_name, r.command, r.user_prompt, r.status,
                    r.exit_code, r.created_at, r.completed_at,
                    (SELECT COUNT(*) FROM events e WHERE e.run_id = r.id)
             FROM runs r
             WHERE r.user_id = ?1
             ORDER BY r.created_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![user_id, limit as i64], |row| {
                Ok(RunSummary {
                    id: row.get(0)?,
                    project_name: row.get(1)?,
                    agent_name: row.get(2)?,
                    command: row.get(3)?,
                    user_prompt: row.get(4)?,
                    status: row.get(5)?,
                    exit_code: row.get(6)?,
                    created_at: row.get(7)?,
                    completed_at: row.get(8)?,
                    event_count: row.get(9)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_run(
        &self,
        user_id: &str,
        run_id: &str,
    ) -> Result<Option<(RunSummary, Vec<CloudEvent>)>> {
        let c = self.conn.lock().unwrap();
        let run = c
            .query_row(
                "SELECT id, project_name, agent_name, command, user_prompt, status,
                        exit_code, created_at, completed_at,
                        (SELECT COUNT(*) FROM events e WHERE e.run_id = ?2)
                 FROM runs WHERE user_id = ?1 AND id = ?2",
                params![user_id, run_id],
                |row| {
                    Ok(RunSummary {
                        id: row.get(0)?,
                        project_name: row.get(1)?,
                        agent_name: row.get(2)?,
                        command: row.get(3)?,
                        user_prompt: row.get(4)?,
                        status: row.get(5)?,
                        exit_code: row.get(6)?,
                        created_at: row.get(7)?,
                        completed_at: row.get(8)?,
                        event_count: row.get(9)?,
                    })
                },
            )
            .ok();
        let Some(run) = run else { return Ok(None) };
        let mut stmt = c.prepare(
            "SELECT event_type, message, metadata_json, created_at
             FROM events WHERE run_id = ?1 ORDER BY id",
        )?;
        let events = stmt
            .query_map(params![run_id], |row| {
                Ok(CloudEvent {
                    event_type: row.get(0)?,
                    message: row.get(1)?,
                    metadata_json: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(Some((run, events)))
    }
}
