# Trace installer for Windows (PowerShell).
#
#   irm https://raw.githubusercontent.com/TaxCollector23/trace/main/scripts/install.ps1 | iex
#
# Downloads the correct trc.exe from GitHub Releases, installs it to
# %USERPROFILE%\.trace\bin\trc.exe, updates the user PATH, and prints next steps.

$ErrorActionPreference = "Stop"

$Repo       = "TaxCollector23/trace"
$InstallDir = Join-Path $env:USERPROFILE ".trace\bin"
$Bin        = Join-Path $InstallDir "trc.exe"

# --- Detect architecture ---
$arch = $env:PROCESSOR_ARCHITECTURE
switch ($arch) {
    "AMD64" { $asset = "trace-windows-x64.exe" }
    "ARM64" { $asset = "trace-windows-x64.exe" } # x64 binary runs under emulation
    default { throw "Unsupported architecture: $arch" }
}

$version = if ($env:TRACE_VERSION) { $env:TRACE_VERSION } else { "latest" }
if ($version -eq "latest") {
    $url = "https://github.com/$Repo/releases/latest/download/$asset"
} else {
    $url = "https://github.com/$Repo/releases/download/$version/$asset"
}

Write-Host "Installing Trace ($asset) ..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

$tmp = "$Bin.download"
Invoke-WebRequest -Uri $url -OutFile $tmp -UseBasicParsing

# Verify the SHA-256 checksum published next to the asset before trusting it.
# A missing checksum (older releases) is allowed unless TRACE_REQUIRE_CHECKSUM.
$published = $null
try {
    $published = (Invoke-WebRequest -Uri "$url.sha256" -UseBasicParsing).Content.Trim().Split()[0].ToLower()
} catch {
    $published = $null
}
if ($published) {
    $local = (Get-FileHash -Algorithm SHA256 $tmp).Hash.ToLower()
    if ($local -ne $published) {
        Remove-Item $tmp -Force
        throw "checksum mismatch for $asset (expected $published, got $local)"
    }
    Write-Host "Checksum verified."
} elseif ($env:TRACE_REQUIRE_CHECKSUM) {
    Remove-Item $tmp -Force
    throw "no checksum published for $asset and TRACE_REQUIRE_CHECKSUM is set"
} else {
    Write-Host "note: no checksum published for this release; skipping verification"
}

Move-Item -Force $tmp $Bin

Write-Host ""
Write-Host "Installed trc to $InstallDir"

# --- Update user PATH if needed ---
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
    $newPath = if ([string]::IsNullOrEmpty($userPath)) { $InstallDir } else { "$userPath;$InstallDir" }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host ""
    Write-Host "Added $InstallDir to your user PATH."
    Write-Host "Open a NEW terminal, then run: trc --help"
} else {
    Write-Host "Trace is already on your PATH. Run: trc --help"
}
