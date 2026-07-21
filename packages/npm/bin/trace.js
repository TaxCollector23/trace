#!/usr/bin/env node
// Launcher: exec the downloaded `trace` binary, forwarding all args and stdio.
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import { binPath } from "../scripts/platform.js";

const bin = binPath();
if (!fs.existsSync(bin)) {
  process.stderr.write(
    "trace: binary not found. Re-run `npm install -g trace`,\n" +
      "or install from https://github.com/TaxCollector23/trace/releases\n"
  );
  process.exit(1);
}

const res = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
process.exit(res.status ?? 1);
