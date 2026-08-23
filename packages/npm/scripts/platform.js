// Shared platform → release-asset mapping for the npm wrapper.
import os from "node:os";
import path from "node:path";
import fs from "node:fs";

export const REPO = "TaxCollector23/trace";

/** Map the current platform/arch to a GitHub Release asset name. */
export function assetName() {
  const platform = os.platform();
  const arch = os.arch(); // 'x64' | 'arm64' | ...
  if (platform === "darwin") {
    return arch === "arm64" ? "trace-macos-arm64" : "trace-macos-x64";
  }
  if (platform === "linux") {
    return arch === "arm64" ? "trace-linux-arm64" : "trace-linux-x64";
  }
  if (platform === "win32") {
    return "trace-windows-x64.exe";
  }
  throw new Error(`Unsupported platform: ${platform}/${arch}`);
}

/** Local path where the downloaded binary is stored inside the package. */
export function binPath() {
  const exe = os.platform() === "win32" ? "trc.exe" : "trc";
  return path.join(import.meta.dirname ?? path.dirname(new URL(import.meta.url).pathname), "..", "bin", exe);
}

export function downloadUrl(version) {
  const asset = assetName();
  return version && version !== "latest"
    ? `https://github.com/${REPO}/releases/download/${version}/${asset}`
    : `https://github.com/${REPO}/releases/latest/download/${asset}`;
}

/**
 * Ensure the platform binary exists on disk, downloading it if missing.
 *
 * This is the single source of truth for "get the binary" and is called from
 * BOTH the postinstall script AND the launcher (`bin/trc.js`). Calling it from
 * the launcher is what makes the wrapper self-healing: if postinstall was
 * skipped or blocked (`--ignore-scripts`, sandboxed CI, offline install),
 * the first `trc` invocation downloads the binary instead of failing with
 * "binary not found".
 *
 * Returns the resolved binary path. Set `log` to stream progress to a stream
 * (postinstall uses stdout; the launcher uses stderr so it never pollutes
 * command output). Honors TRACE_VERSION for pinning.
 */
export async function ensureBinary({ log } = {}) {
  const out = binPath();
  if (fs.existsSync(out) && fs.statSync(out).size > 0) {
    return out;
  }
  const version = process.env.TRACE_VERSION || "latest";
  const url = downloadUrl(version);
  log?.write(`trc: downloading ${url}\n`);
  const res = await fetch(url, { redirect: "follow" });
  if (!res.ok) {
    throw new Error(`download failed: ${res.status} ${res.statusText} (${url})`);
  }
  fs.mkdirSync(path.dirname(out), { recursive: true });
  // Write to a temp path then rename so a partial download never leaves a
  // truncated binary in place (which would defeat the size check above).
  const tmp = `${out}.download-${process.pid}`;
  fs.writeFileSync(tmp, Buffer.from(await res.arrayBuffer()));
  if (process.platform !== "win32") {
    fs.chmodSync(tmp, 0o755);
  }
  fs.renameSync(tmp, out);
  log?.write(`trc: installed ${out}\n`);
  return out;
}
