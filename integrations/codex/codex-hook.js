#!/usr/bin/env node
// Trace hooks for Codex CLI, desktop, and IDE surfaces.
//
// Codex sends one JSON object on stdin for each lifecycle event. The hook is
// deliberately dependency-free so it can run anywhere Node is available.
// PreToolUse is the enforcement boundary: dangerous Bash commands are denied
// before Codex executes them. Session hooks create/finalize a Trace run so the
// dashboard shows the session even when Codex was not launched with `trc run`.

import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const home = process.env.TRACE_HOME || path.join(os.homedir(), ".trace");
const sessionsDir = path.join(home, "codex-sessions");

function daemonBase() {
  try {
    const state = JSON.parse(fs.readFileSync(path.join(home, "daemon.json"), "utf8"));
    return state?.port ? `http://127.0.0.1:${state.port}` : null;
  } catch {
    return null;
  }
}

async function api(method, route, body) {
  const base = daemonBase();
  if (!base) throw new Error("Trace daemon is not running");
  const response = await fetch(`${base}/api${route}`, {
    method,
    headers: body ? { "content-type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
    signal: AbortSignal.timeout(3000),
  });
  if (!response.ok) throw new Error(`Trace daemon returned ${response.status}`);
  return response.json();
}

function sessionFile(sessionId) {
  const key = crypto.createHash("sha256").update(sessionId || "unknown").digest("hex");
  return path.join(sessionsDir, `${key}.json`);
}

function loadSession(sessionId) {
  try {
    return JSON.parse(fs.readFileSync(sessionFile(sessionId), "utf8"));
  } catch {
    return null;
  }
}

function saveSession(sessionId, value) {
  fs.mkdirSync(sessionsDir, { recursive: true });
  fs.writeFileSync(sessionFile(sessionId), JSON.stringify(value));
}

function gitHead(cwd) {
  const result = spawnSync("git", ["rev-parse", "HEAD"], { cwd, encoding: "utf8" });
  return result.status === 0 ? result.stdout.trim() || null : null;
}

async function projectIdForPath(cwd) {
  const projects = await api("GET", "/projects");
  const existing = projects.find((project) => path.resolve(project.path) === path.resolve(cwd));
  if (existing) return existing.id;
  const created = await api("POST", "/projects", {
    name: path.basename(cwd) || "Codex project",
    path: cwd,
    config_path: path.join(cwd, ".trace", "config.toml"),
  });
  return created.id;
}

async function ensureRun(input) {
  const sessionId = input.session_id || "codex-unknown-session";
  const current = loadSession(sessionId);
  if (current?.runId) return current;
  const cwd = input.cwd || process.cwd();
  const projectId = await projectIdForPath(cwd);
  const run = await api("POST", "/runs", {
    project_id: projectId,
    command: "Codex session",
    agent_name: "codex",
    user_prompt: null,
    starting_commit: gitHead(cwd),
  });
  const session = { runId: run.id, cwd, sessionId };
  saveSession(sessionId, session);
  await api("POST", `/runs/${run.id}/events`, {
    type: "session_started",
    message: "Codex session started",
    metadata_json: JSON.stringify({ session_id: sessionId }),
  });
  return session;
}

async function recordCommand(session, command, decision) {
  await api("POST", `/runs/${session.runId}/commands`, {
    command,
    decision,
    exit_code: null,
    stdout_path: null,
    stderr_path: null,
  });
  await api("POST", `/runs/${session.runId}/events`, {
    type: decision === "block" ? "risky_command_blocked" : "command_started",
    message: decision === "block" ? `Trace blocked: ${command}` : `Codex ran: ${command}`,
    metadata_json: null,
  });
}

function deny(reason) {
  process.stdout.write(JSON.stringify({
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      permissionDecision: "deny",
      permissionDecisionReason: reason,
    },
  }));
}

async function finishSession(input) {
  const session = loadSession(input.session_id || "codex-unknown-session");
  if (!session) return;
  try {
    await api("POST", `/runs/${session.runId}/capture`, {});
    await api("POST", `/runs/${session.runId}/analyze`, {});
    await api("POST", `/runs/${session.runId}/events`, {
      type: "session_ended",
      message: "Codex session ended",
      metadata_json: null,
    });
    await api("POST", `/runs/${session.runId}/finish`, {
      status: "completed",
      exit_code: null,
      ending_commit: gitHead(session.cwd),
    });
  } catch {
    // SessionEnd must never make Codex unusable. The run remains visible as
    // interrupted if the daemon disappeared during shutdown.
  }
}

async function main(input) {
  const event = input.hook_event_name;
  if (event === "SessionStart") {
    await ensureRun(input);
    return;
  }
  if (event === "SessionEnd") {
    await finishSession(input);
    return;
  }

  const tool = input.tool_name;
  if (event === "PreToolUse" && tool === "Bash") {
    const command = input.tool_input?.command;
    if (typeof command !== "string" || !command.trim()) return;
    const session = await ensureRun(input);
    const result = await api("POST", "/check-command", { command });
    await recordCommand(session, command, result.decision);
    if (result.decision === "block") {
      deny(`Trace blocked this command: ${result.reason || "matched a high-risk rule"}`);
    }
    return;
  }

  if (event === "PostToolUse" && (tool === "Bash" || tool === "apply_patch")) {
    const session = await ensureRun(input);
    await api("POST", `/runs/${session.runId}/events`, {
      type: "tool_call_finished",
      message: `Codex finished ${tool}`,
      metadata_json: null,
    });
  }
}

let raw = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => { raw += chunk; });
process.stdin.on("end", async () => {
  let input = null;
  try {
    input = JSON.parse(raw || "{}");
  } catch {
    // There is no safely identifiable tool to deny when the lifecycle payload
    // itself is malformed.
    return;
  }
  try {
    await main(input);
  } catch (error) {
    // PreToolUse is a safety boundary. A missing daemon or malformed response
    // denies the Bash call rather than silently pretending it was checked.
    if (input.hook_event_name === "PreToolUse") {
      deny(`Trace could not check this action: ${error.message}`);
      process.exitCode = 0;
    }
  }
});
