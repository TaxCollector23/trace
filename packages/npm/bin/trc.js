#!/usr/bin/env node
// Launcher: exec the downloaded `trc` binary, forwarding all args and stdio.
//
// Self-healing: if the binary isn't present yet (postinstall was skipped or
// blocked, e.g. `npm install --ignore-scripts`), download it now on first run
// so the user never sees "binary not found".
import { spawnSync } from "node:child_process";
import { ensureBinary } from "../scripts/platform.js";

let bin;
try {
  bin = await ensureBinary({ log: process.stderr });
} catch (e) {
  process.stderr.write(
    `trc: could not obtain the binary (${e.message}).\n` +
      "Install manually from https://github.com/TaxCollector23/trace/releases\n"
  );
  process.exit(1);
}

const res = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
process.exit(res.status ?? 1);
