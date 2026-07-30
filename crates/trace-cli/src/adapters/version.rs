//! Shared "is this tool installed, and what version" probe used by every
//! per-agent adapter's `capture_metadata`.

/// Best-effort `<bin> --version` probe. `None` if the binary isn't on PATH
/// or doesn't understand `--version`.
pub fn tool_version(bin: &str) -> Option<String> {
    let out = std::process::Command::new(bin)
        .arg("--version")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let line = s.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        None
    } else {
        Some(line.chars().take(80).collect())
    }
}
