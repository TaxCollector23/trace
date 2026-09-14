# @rangan23/trace-cli (npm wrapper)

The npm install path for the Trace CLI. It installs the platform `trc` binary,
starts the local dashboard, and exposes `trc` on your PATH.

```bash
npm install -g @rangan23/trace-cli
trc dashboard
```

After installation, connect every supported agent with:

```bash
trc integrations install all
```

The npm package is `@rangan23/trace-cli`; `trace-cli` without the scope is not
the Trace package.

Pin a version with `TRACE_VERSION=v1.3.0 npm install -g @rangan23/trace-cli`.
