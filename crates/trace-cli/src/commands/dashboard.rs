//! `trc dashboard` — ensure the daemon is up and open the local dashboard.

use anyhow::Result;

use crate::daemon_ctl;

pub fn run() -> Result<()> {
    let port = daemon_ctl::ensure_running()?;
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
