//! `trc reset --local-data` is explicit, isolated, and keeps project setup.

use std::path::PathBuf;
use std::process::Command;
use trace_core::models::{NewCommand, NewEvent, NewProject, NewRun};
use trace_core::Store;

fn unique_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "trace-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn reset_local_data_purges_telemetry_and_preserves_project_registry() {
    let home = unique_path("reset-home");
    let db = home.join("custom.db");
    let project_path = home.join("project");
    std::fs::create_dir_all(project_path.join(".trace/runs/run-1")).unwrap();

    let store = Store::open(&db).unwrap();
    let project = store
        .upsert_project(&NewProject {
            name: "Reset test".into(),
            path: project_path.display().to_string(),
            config_path: project_path
                .join(".trace/config.toml")
                .display()
                .to_string(),
        })
        .unwrap();
    let run = store
        .create_run(&NewRun {
            project_id: project.id,
            command: "echo telemetry".into(),
            agent_name: None,
            user_prompt: None,
            starting_commit: None,
            owner_pid: None,
        })
        .unwrap();
    store
        .add_command(
            &run.id,
            &NewCommand {
                command: "echo telemetry".into(),
                decision: "allow".into(),
                exit_code: None,
                stdout_path: None,
                stderr_path: None,
            },
        )
        .unwrap();
    store
        .add_event(
            &run.id,
            &NewEvent {
                event_type: "note".into(),
                message: "telemetry".into(),
                metadata_json: None,
            },
        )
        .unwrap();
    drop(store);

    let output = Command::new(env!("CARGO_BIN_EXE_trc"))
        .args(["reset", "--local-data", "--yes"])
        .env("TRACE_HOME", &home)
        .env("TRACE_DB", &db)
        .output()
        .expect("run trc reset");
    assert!(
        output.status.success(),
        "reset failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let reopened = Store::open(&db).unwrap();
    assert!(reopened.telemetry_counts().unwrap().is_empty());
    assert_eq!(reopened.list_projects().unwrap().len(), 1);
    assert!(!project_path.join(".trace/runs").exists());

    let _ = std::fs::remove_dir_all(&home);
}
