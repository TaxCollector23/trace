//! `trg update` — replace the running binary with the latest GitHub release.
//!
//! Downloads the correct release asset for this platform and atomically swaps it
//! in place. Falls back to printing the install command if the binary cannot be
//! replaced (e.g. installed somewhere read-only such as a Homebrew cellar).

use std::io::Read;

use anyhow::{anyhow, Context, Result};

const REPO: &str = "TaxCollector23/TraceGuard";

pub fn run() -> Result<()> {
    let asset = asset_name()?;
    let url = format!("https://github.com/{REPO}/releases/latest/download/{asset}");
    let exe = std::env::current_exe().context("locating current executable")?;

    // Homebrew-managed installs should be updated via brew, not in place.
    if exe.to_string_lossy().contains("/Cellar/") || exe.to_string_lossy().contains("/homebrew/") {
        println!("Detected a Homebrew install. Update with:");
        println!("  brew update && brew upgrade traceguard");
        return Ok(());
    }

    println!("Downloading latest TraceGuard ({asset})...");
    let bytes = download(&url)?;

    // Write to a temp file next to the target, then rename over it.
    let dir = exe
        .parent()
        .ok_or_else(|| anyhow!("no parent dir for {}", exe.display()))?;
    let tmp = dir.join(".trg-update.tmp");
    std::fs::write(&tmp, &bytes).with_context(|| format!("writing {}", tmp.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    }

    std::fs::rename(&tmp, &exe)
        .with_context(|| format!("replacing {} (need write permission)", exe.display()))?;

    println!("Updated: {}", exe.display());
    println!("Run `trg --version` to confirm.");
    Ok(())
}

fn download(url: &str) -> Result<Vec<u8>> {
    let resp = ureq::get(url)
        .call()
        .with_context(|| format!("GET {url}"))?;
    let mut buf = Vec::new();
    resp.into_reader()
        .read_to_end(&mut buf)
        .context("reading release body")?;
    if buf.is_empty() {
        return Err(anyhow!("downloaded an empty file"));
    }
    Ok(buf)
}

fn asset_name() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let name = match (os, arch) {
        ("macos", "aarch64") => "trg-macos-arm64",
        ("macos", "x86_64") => "trg-macos-x64",
        ("linux", "aarch64") => "trg-linux-arm64",
        ("linux", "x86_64") => "trg-linux-x64",
        ("windows", _) => "trg-windows-x64.exe",
        _ => return Err(anyhow!("unsupported platform: {os}/{arch}")),
    };
    Ok(name.to_string())
}
