# Trace installer for Windows (PowerShell).
#
#   irm https://raw.githubusercontent.com/TaxCollector23/trace/main/scripts/install.ps1 | iex
#
# Downloads the correct trace.exe from GitHub Releases, installs it to
# %USERPROFILE%\.trace\bin\trace.exe, updates the user PATH, and prints next steps.

$ErrorActionPreference = "Stop"

$Repo       = "TaxCollector23/trace"
$InstallDir = Join-Path $env:USERPROFILE ".trace\bin"
$Bin        = Join-Path $InstallDir "trace.exe"

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

Invoke-WebRequest -Uri $url -OutFile $Bin -UseBasicParsing

Write-Host ""
Write-Host "Installed trace to $InstallDir"

# --- Update user PATH if needed ---
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
    $newPath = if ([string]::IsNullOrEmpty($userPath)) { $InstallDir } else { "$userPath;$InstallDir" }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host ""
    Write-Host "Added $InstallDir to your user PATH."
    Write-Host "Open a NEW terminal, then run: trace --help"
} else {
    Write-Host "Trace is already on your PATH. Run: trace --help"
}
