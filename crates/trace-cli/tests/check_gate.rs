//! Integration test for fix #3 (`trc check`): an intentional CI-gate
//! failure must exit non-zero through its own clean summary, never through
//! the generic crash-style error path that appends
//! "Re-run with TRACE_DEBUG=1 for the full details." — that footer is meant
//! for real internal failures, not an expected "found something risky"
//! result.
//!
//! `trc check` is fully local (command guard + secret scanner over a file),
//! so this test needs no daemon.

use std::process::Command;

fn trc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_trc"))
}

fn unique_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "trc-check-test-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn check_exits_nonzero_without_crash_footer_on_a_risky_file() {
    let dir = unique_dir("risky");
    let file = dir.join("risky.sh");
    std::fs::write(&file, "rm -rf /\n").unwrap();

    let output = trc()
        .arg("check")
        .arg(&file)
        .env_remove("TRACE_DEBUG")
        .output()
        .expect("failed to run trc check");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1, got {:?}\nstdout: {stdout}\nstderr: {stderr}",
        output.status.code()
    );
    assert!(
        !stdout.contains("TRACE_DEBUG") && !stderr.contains("TRACE_DEBUG"),
        "an intentional gate failure must not go through the crash-style \
         debug footer\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("check failed"),
        "expected check's own clear summary line, got:\n{stdout}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn check_exits_zero_on_a_clean_file() {
    let dir = unique_dir("clean");
    let file = dir.join("clean.sh");
    std::fs::write(&file, "echo hello world\n").unwrap();

    let output = trc()
        .arg("check")
        .arg(&file)
        .output()
        .expect("failed to run trc check");

    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_dir_all(&dir);
}
