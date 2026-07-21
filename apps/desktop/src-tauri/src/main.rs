//! Trace desktop shell: a thin native window around the local dashboard.
//!
//! It does not duplicate the dashboard's UI — it hosts the same daemon the CLI
//! uses (reusing an already-running one if `trace dashboard`/`trace run` started
//! it first) and points a native window at it.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::thread;
use std::time::{Duration, Instant};

use tauri::{WebviewUrl, WebviewWindowBuilder};

fn daemon_healthy(port: u16) -> bool {
    ureq::get(&format!("http://127.0.0.1:{port}/api/health"))
        .timeout(Duration::from_millis(500))
        .call()
        .is_ok()
}

/// Reuse an already-running daemon if one is healthy, otherwise host one for
/// the lifetime of this app. Returns the port to load in the window.
fn ensure_daemon() -> anyhow::Result<u16> {
    if let Ok(Some(state)) = trace_daemon::DaemonState::read() {
        if daemon_healthy(state.port) {
            return Ok(state.port);
        }
    }

    thread::spawn(|| {
        if let Err(e) = trace_daemon::run_blocking(trace_daemon::PREFERRED_PORT) {
            eprintln!("trace-desktop: daemon exited: {e:#}");
        }
    });

    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(Some(state)) = trace_daemon::DaemonState::read() {
            if daemon_healthy(state.port) {
                return Ok(state.port);
            }
        }
        thread::sleep(Duration::from_millis(150));
    }
    anyhow::bail!("daemon did not become ready within 10s")
}

fn build_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{MenuBuilder, PredefinedMenuItem, SubmenuBuilder};

    let app_submenu = SubmenuBuilder::new(app, "Trace")
        .item(&PredefinedMenuItem::about(app, Some("About Trace"), None)?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, Some("Quit Trace"))?)
        .build()?;

    MenuBuilder::new(app).item(&app_submenu).build()
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let menu = build_menu(app.handle())?;
            app.set_menu(menu)?;

            let port = ensure_daemon().map_err(|e| e.to_string())?;
            let url = format!("http://127.0.0.1:{port}");
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse()?))
                .title("Trace")
                .inner_size(1280.0, 860.0)
                .min_inner_size(960.0, 640.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running trace-desktop");
}
