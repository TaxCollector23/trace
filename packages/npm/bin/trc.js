#!/usr/bin/env node
// Launcher: exec the downloaded `trc` binary, forwarding all args and stdio.
//
// Self-healing: if the binary isn't present yet (postinstall was skipped or
// blocked, e.g. `npm install --ignore-scripts`), download it now on first run
// so the user never sees "binary not found".
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { binPath, ensureBinary } from "../scripts/platform.js";

let bin;
try {
  bin = await ensureBinary();
} catch (e) {
  const home = process.env.TRACE_HOME || path.join(os.homedir(), ".trace");
  try {
    fs.mkdirSync(home, { recursive: true });
    fs.appendFileSync(path.join(home, "install-errors.log"), `${new Date().toISOString()} binary unavailable: ${e.message}\n`);
  } catch { /* keep the command quiet if diagnostics cannot be written */ }
  process.exit(1);
}

const res = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
process.exit(res.status ?? 1);
