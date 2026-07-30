# traceguard (npm wrapper)

Optional npm install path for the TraceGuard CLI. Installs the platform `trg`
binary from GitHub Releases and exposes both `trg` and `traceguard`.

```bash
npm install -g traceguard
trg --help
```

This is a **fallback** install method. The primary methods are Homebrew (macOS),
the PowerShell script (Windows), and the curl shell script (Linux/macOS). See
the [main README](../../README.md).

Pin a version with `TRACEGUARD_VERSION=v0.1.0 npm install -g traceguard`.
