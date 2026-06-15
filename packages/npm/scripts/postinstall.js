// Downloads the correct `trg` binary from GitHub Releases into ./bin during
// `npm install`. Honors TRACEGUARD_VERSION. Fails soft with a clear message so
// installs do not hard-error in CI without network access.
import fs from "node:fs";
import path from "node:path";
import { downloadUrl, binPath } from "./platform.js";

const version = process.env.TRACEGUARD_VERSION || "latest";

async function main() {
  const url = downloadUrl(version);
  const out = binPath();
  fs.mkdirSync(path.dirname(out), { recursive: true });

  process.stdout.write(`traceguard: downloading ${url}\n`);
  const res = await fetch(url, { redirect: "follow" });
  if (!res.ok) {
    throw new Error(`download failed: ${res.status} ${res.statusText}`);
  }
  const buf = Buffer.from(await res.arrayBuffer());
  fs.writeFileSync(out, buf);
  if (process.platform !== "win32") {
    fs.chmodSync(out, 0o755);
  }
  process.stdout.write(`traceguard: installed ${out}\n`);
}

main().catch((e) => {
  process.stderr.write(
    `traceguard: could not download the binary (${e.message}).\n` +
      `You can install manually from https://github.com/TaxCollector23/TraceGuard/releases\n`
  );
  // Do not fail the whole npm install.
  process.exit(0);
});
