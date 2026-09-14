// Downloads the correct `trc` binary from GitHub Releases into ./bin during
// `npm install`. Honors TRACE_VERSION. Fails soft with a clear message so
// installs do not hard-error in CI without network access — and even if this
// is skipped entirely (`--ignore-scripts`), the launcher (bin/trc.js) will
// download the binary on first run, so `trc` still works.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { binPath, ensureBinary } from "./platform.js";

// npm runs lifecycle scripts in a noisy context. Keep the happy path to one
// small, memorable hand-off, and write failures where the local dashboard can
// surface them instead of making an install look broken when it can recover.
const traceHome = process.env.TRACE_HOME || path.join(os.homedir(), ".trace");
const diagnostics = path.join(traceHome, "install-errors.log");
const bannerMarker = path.join(traceHome, "install-banner-shown");

function recordFailure(message) {
  try {
    fs.mkdirSync(traceHome, { recursive: true });
    fs.appendFileSync(diagnostics, `${new Date().toISOString()} ${message}\n`);
  } catch {
    // An install must never fail because diagnostics could not be written.
  }
}

function startDaemon(binary) {
  const result = spawnSync(binary, ["daemon", "start"], {
    stdio: "ignore",
    windowsHide: true,
    env: process.env,
  });
  if (result.error || result.status !== 0) {
    recordFailure(`daemon did not start: ${result.error?.message || `exit ${result.status}`}`);
    return null;
  }
  try {
    const state = JSON.parse(fs.readFileSync(path.join(traceHome, "daemon.json"), "utf8"));
    return state.port ? `http://127.0.0.1:${state.port}` : null;
  } catch (error) {
    recordFailure(`daemon started without readable state: ${error.message}`);
    return null;
  }
}

try {
  const binary = await ensureBinary();
  const dashboard = startDaemon(binary);
  const firstInstall = !fs.existsSync(bannerMarker);
  if (firstInstall) {
    fs.mkdirSync(traceHome, { recursive: true });
    fs.writeFileSync(bannerMarker, new Date().toISOString());
    process.stdout.write(
      "Trace is ready.\n" +
        "Next: trc integrations install all\n"
    );
    if (dashboard) process.stdout.write(`Dashboard: ${dashboard}\n`);
  }
} catch (error) {
  recordFailure(`binary install failed: ${error.message}`);
  // Do not fail npm itself. The launcher retries the download on first use.
}
