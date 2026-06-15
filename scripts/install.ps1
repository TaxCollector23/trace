# TraceGuard installer for Windows (PowerShell).
#
#   irm https://raw.githubusercontent.com/TaxCollector23/TraceGuard/main/scripts/install.ps1 | iex
#
# Downloads the correct trg.exe from GitHub Releases, installs it to
# %USERPROFILE%\.traceguard\bin\trg.exe, updates the user PATH, and prints next steps.

$ErrorActionPreference = "Stop"

$Repo       = "TaxCollector23/TraceGuard"
$InstallDir = Join-Path $env:USERPROFILE ".traceguard\bin"
$Bin        = Join-Path $InstallDir "trg.exe"

# --- Detect architecture ---
$arch = $env:PROCESSOR_ARCHITECTURE
switch ($arch) {
    "AMD64" { $asset = "trg-windows-x64.exe" }
    "ARM64" { $asset = "trg-windows-x64.exe" } # x64 binary runs under emulation
    default { throw "Unsupported architecture: $arch" }
}

$version = if ($env:TRACEGUARD_VERSION) { $env:TRACEGUARD_VERSION } else { "latest" }
if ($version -eq "latest") {
    $url = "https://github.com/$Repo/releases/latest/download/$asset"
} else {
    $url = "https://github.com/$Repo/releases/download/$version/$asset"
}

Write-Host "Installing TraceGuard ($asset) ..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

Invoke-WebRequest -Uri $url -OutFile $Bin -UseBasicParsing

# Provide the `traceguard` long alias as a copy of the same binary.
$Alias = Join-Path $InstallDir "traceguard.exe"
Copy-Item -Path $Bin -Destination $Alias -Force

Write-Host ""
Write-Host "Installed trg (and traceguard alias) to $InstallDir"

# --- Update user PATH if needed ---
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
    $newPath = if ([string]::IsNullOrEmpty($userPath)) { $InstallDir } else { "$userPath;$InstallDir" }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host ""
    Write-Host "Added $InstallDir to your user PATH."
    Write-Host "Open a NEW terminal, then run: trg --help"
} else {
    Write-Host "TraceGuard is already on your PATH. Run: trg --help"
}
