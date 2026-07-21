//! `trace dashboard` — ensure the daemon is up and open the local dashboard.

use anyhow::Result;

use crate::daemon_ctl;

pub fn run() -> Result<()> {
    let port = daemon_ctl::ensure_running()?;
    let url = format!("http://127.0.0.1:{port}");
    println!("Opening Trace dashboard at {url}");
    if let Err(e) = open::that(&url) {
        println!("Could not open a browser automatically ({e}).");
        println!("Open this URL manually: {url}");
    }
    Ok(())
}
