# trace-agent-cli (npm wrapper)

The npm install path for the Trace CLI. It installs the platform `trc` binary,
starts the local dashboard, and exposes `trc` on your PATH.

```bash
npm install -g trace-agent-cli
trc dashboard
```

After installation, connect every supported agent with:

```bash
trc integrations install all
```

Pin a version with `TRACE_VERSION=v1.3.0 npm install -g trace-agent-cli`.
