//! `trc dashboard` — ensure the daemon is up and open the local dashboard.

use std::io::Read;
use std::time::Duration;

use anyhow::Result;
use serde_json::Value;

use crate::daemon_ctl;

pub fn run() -> Result<()> {
    let mut port = daemon_ctl::ensure_running()?;
    if matches!(serves_current_bundle(port), Some(false)) {
        match has_active_runs(port) {
            Some(false) if daemon_ctl::stop()? => {
                port = daemon_ctl::ensure_running()?;
                println!("Updated the local dashboard to this build.");
            }
            Some(true) => eprintln!(
                "A monitored run is still active. This daemon serves an older dashboard; run `trc dashboard` again after the session ends to update it."
            ),
            _ => eprintln!(
                "Could not safely update the running dashboard. Check `trc status` and retry after any active session ends."
            ),
        }
    }
    let url = format!("http://127.0.0.1:{port}");
    if let Err(_e) = open::that(&url) {
        // Keep the terminal useful when no desktop browser is available. The
        // full error belongs in the daemon log/dashboard, not in onboarding.
        eprintln!("dashboard: {url} (open it in a browser)");
    } else {
        println!("dashboard: {url}");
    }
    Ok(())
}

fn serves_current_bundle(port: u16) -> Option<bool> {
    let expected = trace_daemon::assets::dashboard_index_bytes()?;
    let response = ureq::get(&format!("http://127.0.0.1:{port}/"))
        .timeout(Duration::from_secs(2))
        .call()
        .ok()?;
    let mut served = Vec::new();
    response
        .into_reader()
        .take(1024 * 1024)
        .read_to_end(&mut served)
        .ok()?;
    Some(served == expected)
}

fn has_active_runs(port: u16) -> Option<bool> {
    let dashboard: Value = ureq::get(&format!("http://127.0.0.1:{port}/api/dashboard"))
        .timeout(Duration::from_secs(2))
        .call()
        .ok()?
        .into_json()
        .ok()?;
    active_state(&dashboard)
}

fn active_state(dashboard: &Value) -> Option<bool> {
    let runs = dashboard.get("runs")?.as_array()?;
    if runs.iter().any(|run| run["status"] == "running") {
        return Some(true);
    }
    // Older daemons cap /api/dashboard at 50 rows. A full page cannot prove
    // that an older, still-running session is absent, so keep that daemon up.
    if runs.len() >= 50 {
        return None;
    }
    Some(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn restart_check_requires_a_complete_idle_snapshot() {
        assert_eq!(active_state(&json!({ "runs": [] })), Some(false));
        assert_eq!(
            active_state(&json!({ "runs": [{ "status": "running" }] })),
            Some(true)
        );
        assert_eq!(
            active_state(&json!({ "runs": vec![json!({ "status": "completed" }); 50] })),
            None
        );
        assert_eq!(active_state(&json!({})), None);
    }
}
