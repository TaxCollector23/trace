//! Integration test for fix #3 (`trc self-check`): the same requirement as
//! `trc check` — an intentional CI-gate failure exits non-zero through its
//! own summary, never the generic "Re-run with TRACE_DEBUG=1" crash-style
//! footer. `run_policy_eval`/`run_redteam_eval` are pure and local, so this
//! needs no daemon.

use std::process::Command;

#[test]
fn self_check_never_prints_the_debug_footer() {
    let output = Command::new(env!("CARGO_BIN_EXE_trc"))
        .arg("self-check")
        .env_remove("TRACE_DEBUG")
        .output()
        .expect("failed to run trc self-check");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stdout.contains("TRACE_DEBUG") && !stderr.contains("TRACE_DEBUG"),
        "self-check's gate must never go through the crash-style debug \
         footer, regardless of pass/fail\nstdout: {stdout}\nstderr: {stderr}"
    );
    // The shipped fixtures are expected to pass on a healthy build (exit 0).
    // A regression would exit 1 via its own clean summary — either is a
    // valid outcome for this assertion; the property under test is the one
    // checked above.
    assert!(
        matches!(output.status.code(), Some(0) | Some(1)),
        "unexpected exit status: {:?}\nstdout: {stdout}\nstderr: {stderr}",
        output.status
    );
}
