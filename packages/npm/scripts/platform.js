// Shared platform to release-asset mapping for the npm wrapper.
import os from "node:os";
import path from "node:path";
import fs from "node:fs";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";

export const REPO = "TaxCollector23/trace";

// Directory of THIS file, resolved correctly on every platform. The naive
// `new URL(import.meta.url).pathname` returns "/C:/Users/…" on Windows a
// leading slash and forward slashes that path.join() mangles into a
// drive-root path, which is how the binary ended up being written to
// C:\Windows\System32. fileURLToPath handles the file:// to native conversion.
const HERE =
  import.meta.dirname ?? path.dirname(fileURLToPath(import.meta.url));

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
  return path.join(HERE, "..", "bin", exe);
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
  const bytes = Buffer.from(await res.arrayBuffer());

  // Verify the SHA-256 checksum published next to the asset BEFORE writing the
  // binary to disk or marking it executable. A mismatch aborts hard.
  await verifyChecksum(url, bytes, log);

  fs.mkdirSync(path.dirname(out), { recursive: true });
  // Write to a temp path then rename so a partial download never leaves a
  // truncated binary in place (which would defeat the size check above).
  const tmp = `${out}.download-${process.pid}`;
  fs.writeFileSync(tmp, bytes);
  if (process.platform !== "win32") {
    fs.chmodSync(tmp, 0o755);
  }
  fs.renameSync(tmp, out);
  log?.write(`trc: installed ${out}\n`);
  return out;
}

/**
 * Verify `bytes` against the `<asset>.sha256` published alongside it.
 *
 * - checksum present + matches -> proceed.
 * - checksum present + mismatch -> throw (never write a tampered/corrupt binary).
 * - checksum absent (e.g. an older release, or a fetch error) -> proceed with a
 *   note, UNLESS `TRACE_REQUIRE_CHECKSUM` is set, in which case throw. This keeps
 *   pre-checksum releases installable while verifying every release that ships one.
 */
async function verifyChecksum(url, bytes, log) {
  const sumUrl = `${url}.sha256`;
  let published;
  try {
    const res = await fetch(sumUrl, { redirect: "follow" });
    if (res.ok) {
      published = (await res.text()).trim().split(/\s+/)[0]?.toLowerCase();
    }
  } catch {
    // network error fetching the checksum; handled as "absent" below
  }

  if (!published) {
    if (process.env.TRACE_REQUIRE_CHECKSUM) {
      throw new Error(
        `no checksum published at ${sumUrl} and TRACE_REQUIRE_CHECKSUM is set`
      );
    }
    log?.write("trc: no checksum published for this release; skipping verification\n");
    return;
  }

  const actual = crypto.createHash("sha256").update(bytes).digest("hex");
  if (actual.toLowerCase() !== published) {
    throw new Error(
      `checksum mismatch for ${url}\n  expected ${published}\n  got      ${actual}\n` +
        "Refusing to install. Try again, or download manually from " +
        `https://github.com/${REPO}/releases`
    );
  }
  log?.write("trc: checksum verified\n");
}
