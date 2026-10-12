#!/usr/bin/env node

import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const args = new Map();
for (let index = 2; index < process.argv.length; index += 1) {
  const value = process.argv[index];
  if (value.startsWith("--")) args.set(value, process.argv[index + 1] ?? "true");
}

const count = Math.max(1, Number.parseInt(args.get("--events") ?? "1000", 10) || 1000);
const sessionCount = Math.max(1, Number.parseInt(args.get("--sessions") ?? "40", 10) || 40);
const seed = Number.parseInt(args.get("--seed") ?? "42", 10) || 42;
const output = resolve(args.get("--out") ?? "tests/fixtures/usage-large.json");
const agents = ["claude", "codex", "cursor", "opencode", "aider"];
const models = ["claude-3-5-sonnet-20241022", "gpt-5-codex", "cursor-default", "opencode-default"];
const projects = ["trace", "nomo-engine", "cutdex"];
const baseTime = Date.parse("2026-10-12T12:00:00.000Z");

let state = seed >>> 0;
function random() {
  state = (state * 1664525 + 1013904223) >>> 0;
  return state / 0x100000000;
}

const events = Array.from({ length: count }, (_, index) => {
  const session = index % sessionCount;
  const isSubagent = index % 17 === 0;
  const knownPrice = index % 13 !== 0;
  const input = 300 + Math.floor(random() * 5800);
  const output = 80 + Math.floor(random() * 2200);
  const cacheRead = index % 3 === 0 ? Math.floor(random() * 1200) : 0;
  const cacheWrite = index % 11 === 0 ? Math.floor(random() * 200) : 0;
  const reasoning = index % 5 === 0 ? Math.floor(random() * 900) : null;
  const agent = agents[index % agents.length];
  return {
    id: `synthetic-event-${String(index + 1).padStart(7, "0")}`,
    run_id: `synthetic-run-${session + 1}`,
    agent,
    session_id: `synthetic-session-${session + 1}`,
    message_id: `synthetic-message-${index + 1}`,
    parent_message_id: isSubagent ? `synthetic-message-${Math.max(1, index)}` : null,
    is_subagent: isSubagent,
    subagent_id: isSubagent ? `synthetic-subagent-${session + 1}` : null,
    model: models[index % models.length],
    input_tokens: input,
    output_tokens: output,
    cache_read_tokens: cacheRead,
    cache_write_tokens: cacheWrite,
    reasoning_tokens: reasoning,
    ts: new Date(baseTime - (index % 720) * 60 * 60 * 1000).toISOString(),
    cost_usd: knownPrice ? Number(((input * 0.000003 + output * 0.000015) + cacheRead * 0.0000003).toFixed(8)) : null,
    cost_source: knownPrice ? (index % 2 === 0 ? "computed" : "reported") : "unknown",
    raw_ref: `synthetic://usage/${index + 1}`,
    project_name: projects[session % projects.length],
  };
});

await mkdir(dirname(output), { recursive: true });
await writeFile(
  output,
  `${JSON.stringify({ synthetic: true, generated_at: new Date().toISOString(), events }, null, 2)}\n`,
  "utf8"
);
console.log(`Wrote ${events.length.toLocaleString()} synthetic usage events to ${output}`);
