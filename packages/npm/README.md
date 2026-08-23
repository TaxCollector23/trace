# trace-dev (npm wrapper)

Optional npm install path for the Trace CLI. Installs the platform `trc`
binary from GitHub Releases and exposes it on your PATH as `trc`.

```bash
npm install -g trace-dev
trc --help
```

This is a **fallback** install method. The primary methods are Homebrew (macOS),
the PowerShell script (Windows), and the curl shell script (Linux/macOS). See
the [main README](../../README.md).

Pin a version with `TRACE_VERSION=v1.2.0 npm install -g trace-dev`.
