//! Integration test for fix #4: a 404 on a `/api/runs/<id>`-shaped daemon
//! path must surface as a direct "run not found" message, not the raw
//! `error: GET /api/runs/<id>` HTTP internals (which used to require
//! `TRACE_DEBUG=1` just to learn what actually happened).
//!
//! Exercises `trc show` end to end against a real (isolated) daemon.

use std::process::Command;

#[test]
fn show_translates_404_into_a_friendly_run_not_found_message() {
    let trace_home = std::env::temp_dir().join(format!(
        "trc-404-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&trace_home).expect("create TRACE_HOME dir");

    let run_id = "nonexistent-run-id";
    let output = Command::new(env!("CARGO_BIN_EXE_trc"))
        .args(["show", run_id])
        .env("TRACE_HOME", &trace_home)
        .env_remove("TRACE_DEBUG")
        .env_remove("TRACE_DB")
        .output()
        .expect("failed to run trc show");

    // Best-effort cleanup of the daemon this test started.
    let cleanup = || {
        let _ = Command::new(env!("CARGO_BIN_EXE_trc"))
            .args(["daemon", "stop"])
            .env("TRACE_HOME", &trace_home)
            .output();
        let _ = std::fs::remove_dir_all(&trace_home);
    };

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    cleanup();

    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stderr.contains(&format!("run {run_id} not found")),
        "expected a direct 'run not found' message, got:\nstderr: {stderr}"
    );
    assert!(
        stderr.contains("trc runs"),
        "expected the error to point at `trc runs`, got:\nstderr: {stderr}"
    );
    // The raw HTTP verb + path must no longer leak into the default
    // (non-debug) error message.
    assert!(
        !stderr.contains("GET /api/runs"),
        "raw HTTP internals leaked into the default error message:\nstderr: {stderr}"
    );
}
