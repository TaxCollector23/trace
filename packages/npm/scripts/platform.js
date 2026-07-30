// Shared platform → release-asset mapping for the npm wrapper.
import os from "node:os";
import path from "node:path";

export const REPO = "TaxCollector23/TraceGuard";

/** Map the current platform/arch to a GitHub Release asset name. */
export function assetName() {
  const platform = os.platform();
  const arch = os.arch(); // 'x64' | 'arm64' | ...
  if (platform === "darwin") {
    return arch === "arm64" ? "trg-macos-arm64" : "trg-macos-x64";
  }
  if (platform === "linux") {
    return arch === "arm64" ? "trg-linux-arm64" : "trg-linux-x64";
  }
  if (platform === "win32") {
    return "trg-windows-x64.exe";
  }
  throw new Error(`Unsupported platform: ${platform}/${arch}`);
}

/** Local path where the downloaded binary is stored inside the package. */
export function binPath() {
  const exe = os.platform() === "win32" ? "trg.exe" : "trg";
  return path.join(import.meta.dirname ?? path.dirname(new URL(import.meta.url).pathname), "..", "bin", exe);
}

export function downloadUrl(version) {
  const asset = assetName();
  return version && version !== "latest"
    ? `https://github.com/${REPO}/releases/download/${version}/${asset}`
    : `https://github.com/${REPO}/releases/latest/download/${asset}`;
}
