//! Integration test for fix #2 (`trc init`): a freshly committed, clean repo
//! must not be reported as dirty because of `trc init`'s own `.trace/`
//! writes. Also proves fix #5's improved next-steps message.
//!
//! Uses a real `git init` + commit in a disposable directory, and a
//! disposable `TRACE_HOME` so this never touches the developer's real
//! `~/.trace` store (the daemon and database both honor `TRACE_HOME`).

use std::path::Path;
use std::process::Command;

fn run_git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap_or_else(|e| panic!("failed to run git {args:?}: {e}"));
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

fn git_status_porcelain(dir: &Path) -> String {
    let out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir)
        .output()
        .expect("failed to run git status");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn init_does_not_misreport_a_freshly_committed_clean_repo_as_dirty() {
    let unique = format!(
        "trc-init-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let repo = std::env::temp_dir().join(format!("{unique}-repo"));
    let trace_home = std::env::temp_dir().join(format!("{unique}-home"));
    std::fs::create_dir_all(&repo).expect("create repo dir");
    std::fs::create_dir_all(&trace_home).expect("create TRACE_HOME dir");

    run_git(&repo, &["init", "-q"]);
    run_git(&repo, &["config", "user.email", "trc-test@example.com"]);
    run_git(&repo, &["config", "user.name", "Trace Test"]);
    std::fs::write(repo.join("README.md"), "hello\n").unwrap();
    run_git(&repo, &["add", "README.md"]);
    run_git(&repo, &["commit", "-q", "-m", "initial commit"]);

    // Sanity: the repo really is clean before trc touches anything.
    let pre = git_status_porcelain(&repo);
    assert!(
        pre.is_empty(),
        "test repo was not clean before init: {pre:?}"
    );

    let output = Command::new(env!("CARGO_BIN_EXE_trc"))
        .arg("init")
        .current_dir(&repo)
        .env("TRACE_HOME", &trace_home)
        .env_remove("TRACE_DB")
        .output()
        .expect("failed to run trc init");

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // Best-effort cleanup of the daemon this test started, regardless of
    // what the assertions below decide.
    let cleanup = || {
        let _ = Command::new(env!("CARGO_BIN_EXE_trc"))
            .args(["daemon", "stop"])
            .env("TRACE_HOME", &trace_home)
            .output();
        let _ = std::fs::remove_dir_all(&repo);
        let _ = std::fs::remove_dir_all(&trace_home);
    };

    if !output.status.success() {
        cleanup();
        panic!("trc init failed:\nstdout: {stdout}\nstderr: {stderr}");
    }

    // The bug: init used to write .trace/ before checking dirtiness, so it
    // always reported the clean repo as dirty because of its own files.
    if stdout.contains("uncommitted changes") {
        cleanup();
        panic!("trc init misreported the freshly committed clean repo as dirty:\n{stdout}");
    }

    // Fix #5: the next-steps message should point at the real golden path,
    // not just `trc install agents`.
    let has_run = stdout.contains("trc run");
    let has_dashboard = stdout.contains("trc dashboard");

    // git status should also stay clean after init: .trace/ is now fully
    // git-ignored (widened from ignoring only `runs/`).
    let post = git_status_porcelain(&repo);

    cleanup();

    assert!(has_run, "next-steps message missing `trc run`:\n{stdout}");
    assert!(
        has_dashboard,
        "next-steps message missing `trc dashboard`:\n{stdout}"
    );
    assert!(
        post.is_empty(),
        "git status not clean after trc init (expected .trace/ to be fully \
         git-ignored): {post:?}"
    );
}
