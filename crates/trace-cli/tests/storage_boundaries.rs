//! End-to-end storage-boundary regression tests.
//!
//! The command deliberately emits a known fake credential on both output
//! streams. After the run, every regular file in the isolated Trace home and
//! project `.trace` directory is scanned as bytes. This catches leaks in the
//! SQLite database, daemon log, stdout/stderr logs, compressed diff, and sidecar
//! metadata rather than only checking the in-memory API response.

use std::path::{Path, PathBuf};
use std::process::Command;

fn run_git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap_or_else(|e| panic!("failed to run git {args:?}: {e}"));
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

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

fn scan_files(root: &Path, needle: &[u8], found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_files(&path, needle, found);
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if bytes.windows(needle.len()).any(|window| window == needle) {
            found.push(path);
        }
    }
}

#[test]
fn fake_secret_never_reaches_trace_storage_files() {
    let repo = unique_path("storage-repo");
    let trace_home = unique_path("storage-home");
    let trace_db = trace_home.join("isolated.db");
    std::fs::create_dir_all(&repo).expect("create repo");
    std::fs::create_dir_all(&trace_home).expect("create TRACE_HOME");

    run_git(&repo, &["init", "-q"]);
    run_git(&repo, &["config", "user.email", "trc-test@example.com"]);
    run_git(&repo, &["config", "user.name", "Trace Storage Test"]);
    std::fs::write(repo.join("README.md"), "storage boundary test\n").unwrap();
    run_git(&repo, &["add", "README.md"]);
    run_git(&repo, &["commit", "-q", "-m", "initial"]);

    let secret = concat!("sk-ant-", "abcdefghij1234567890ABCDEFtail");
    let envs = |command: &mut Command| {
        command
            .env("TRACE_HOME", &trace_home)
            .env("TRACE_DB", &trace_db)
            .env("TRACE_STORAGE_TEST_SECRET", secret)
            .env_remove("TRACE_DEBUG");
    };

    let mut init = Command::new(env!("CARGO_BIN_EXE_trc"));
    init.arg("init").current_dir(&repo);
    envs(&mut init);
    let init_output = init.output().expect("run trc init");
    assert!(
        init_output.status.success(),
        "trc init failed:\n{}\n{}",
        String::from_utf8_lossy(&init_output.stdout),
        String::from_utf8_lossy(&init_output.stderr)
    );

    let command = r#"printf '%s\n' "$TRACE_STORAGE_TEST_SECRET" > leaked.txt; printf '%s\n' "$TRACE_STORAGE_TEST_SECRET" >&2"#;
    let mut run = Command::new(env!("CARGO_BIN_EXE_trc"));
    run.args(["run", "--yes", "--no-checks", command])
        .current_dir(&repo);
    envs(&mut run);
    let run_output = run.output().expect("run trc run");
    assert!(
        run_output.status.success(),
        "trc run failed:\n{}\n{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );

    let mut stop = Command::new(env!("CARGO_BIN_EXE_trc"));
    stop.args(["daemon", "stop"]);
    envs(&mut stop);
    let _ = stop.output();

    // The test payload is intentionally outside Trace's storage boundary. Do
    // not include the project file itself in the assertion after removing it.
    let _ = std::fs::remove_file(repo.join("leaked.txt"));
    let needle = secret.as_bytes();
    let mut leaked = Vec::new();
    scan_files(&trace_home, needle, &mut leaked);
    scan_files(&repo.join(".trace"), needle, &mut leaked);
    assert!(
        leaked.is_empty(),
        "raw secret found in Trace storage files: {leaked:?}"
    );

    let mut stored_files = Vec::new();
    scan_file_names(&repo.join(".trace"), &mut stored_files);
    assert!(
        stored_files.iter().any(|p| p.ends_with("stdout.log")),
        "stdout log was not persisted: {stored_files:?}"
    );
    assert!(
        stored_files.iter().any(|p| p.ends_with("stderr.log")),
        "stderr log was not persisted: {stored_files:?}"
    );
    assert!(
        stored_files.iter().any(|p| p.ends_with("diff.patch.gz")),
        "compressed diff was not persisted: {stored_files:?}"
    );

    let _ = std::fs::remove_dir_all(&repo);
    let _ = std::fs::remove_dir_all(&trace_home);
}

fn scan_file_names(root: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_file_names(&path, found);
        } else {
            found.push(path);
        }
    }
}
