#!/usr/bin/env node
// Shared lifecycle hook for Claude Code and Cursor.
//
// The two products use different event names, but both provide session,
// command, and file-edit hooks. This adapter turns those events into one
// Trace run and keeps the enforcement boundary local and deterministic.

import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const home = process.env.TRACE_HOME || path.join(os.homedir(), ".trace");
const sessionsDir = path.join(home, "agent-sessions");

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

function eventName(input) {
  return input.hook_event_name || input.event || "";
}

function agentFor(input) {
  const event = eventName(input);
  if (/^[A-Z]/.test(event)) return "claude";
  return "cursor";
}

function cwdFor(input) {
  return input.cwd || input.workspace_roots?.[0] || input.workspaceRoots?.[0] || process.cwd();
}

function sessionIdFor(input, agent) {
  return String(
    input.session_id || input.sessionID || input.conversation_id || input.generation_id ||
      `${agent}:${cwdFor(input)}`,
  );
}

function sessionPath(agent, sessionId) {
  const key = crypto.createHash("sha256").update(`${agent}:${sessionId}`).digest("hex");
  return path.join(sessionsDir, `${key}.json`);
}

function loadSession(agent, sessionId) {
  try {
    return JSON.parse(fs.readFileSync(sessionPath(agent, sessionId), "utf8"));
  } catch {
    return null;
  }
}

function saveSession(agent, sessionId, session) {
  fs.mkdirSync(sessionsDir, { recursive: true });
  fs.writeFileSync(sessionPath(agent, sessionId), JSON.stringify(session));
}

function gitHead(cwd) {
  try {
    const result = spawnSync("git", ["rev-parse", "HEAD"], { cwd, encoding: "utf8" });
    return result.status === 0 ? result.stdout.trim() || null : null;
  } catch {
    return null;
  }
}

async function projectIdForPath(cwd) {
  const projects = await api("GET", "/projects");
  const existing = projects.find((project) => path.resolve(project.path) === path.resolve(cwd));
  if (existing) return existing.id;
  const created = await api("POST", "/projects", {
    name: path.basename(cwd) || "Agent project",
    path: cwd,
    config_path: path.join(cwd, ".trace", "config.toml"),
  });
  return created.id;
}

async function ensureRun(input, agent) {
  const sessionId = sessionIdFor(input, agent);
  const current = loadSession(agent, sessionId);
  if (current?.runId) return current;

  const cwd = cwdFor(input);
  const externalRunId = process.env.TRACE_RUN_ID || null;
  let runId = externalRunId;
  if (!runId) {
    const projectId = await projectIdForPath(cwd);
    const run = await api("POST", "/runs", {
      project_id: projectId,
      command: `${agent === "claude" ? "Claude Code" : "Cursor"} session`,
      agent_name: agent,
      user_prompt: null,
      starting_commit: gitHead(cwd),
    });
    runId = run.id;
  }

  const session = { agent, runId, cwd, sessionId, external: Boolean(externalRunId) };
  saveSession(agent, sessionId, session);
  await api("POST", `/runs/${runId}/events`, {
    type: "session_started",
    message: `${agent === "claude" ? "Claude Code" : "Cursor"} session started`,
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
  const eventType = decision === "block" ? "risky_command_blocked" :
    decision === "allow" ? "command_started" : "risky_command_warned";
  await api("POST", `/runs/${session.runId}/events`, {
    type: eventType,
    message: decision === "block" ? `Trace blocked: ${command}` : `Agent ran: ${command}`,
    metadata_json: null,
  });
}

function denyClaude(reason) {
  process.stdout.write(JSON.stringify({
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      permissionDecision: "deny",
      permissionDecisionReason: reason,
    },
  }));
}

function denyCursor(reason) {
  process.stdout.write(JSON.stringify({
    permission: "deny",
    user_message: reason,
    agent_message: reason,
  }));
}

function allowCursor() {
  process.stdout.write(JSON.stringify({ permission: "allow" }));
}

async function checkCommand(input, agent) {
  const command = input.command || input.tool_input?.command;
  if (typeof command !== "string" || !command.trim()) {
    if (agent === "cursor") allowCursor();
    return;
  }
  const session = await ensureRun(input, agent);
  const result = await api("POST", "/check-command", { command });
  await recordCommand(session, command, result.decision);
  if (result.decision === "block") {
    const reason = `Trace blocked this command: ${result.reason || "matched a high-risk rule"}`;
    if (agent === "cursor") denyCursor(reason);
    else denyClaude(reason);
  } else if (agent === "cursor") {
    allowCursor();
  }
}

function editDetails(input, agent) {
  const toolInput = input.tool_input || {};
  const filePath = input.file_path || toolInput.file_path || input.tool_response?.filePath || null;
  const edits = input.edits || toolInput.edits;
  const diff = input.new_string || toolInput.new_string || input.content || toolInput.content ||
    (edits ? JSON.stringify(edits) : null);
  return { filePath, diff: typeof diff === "string" ? diff.slice(0, 8000) : null, agent };
}

async function recordEdit(input, agent) {
  const session = await ensureRun(input, agent);
  const { filePath, diff } = editDetails(input, agent);
  await api("POST", `/runs/${session.runId}/events`, {
    type: "file_changed",
    message: `${agent === "claude" ? "Claude Code" : "Cursor"} changed ${filePath || "a file"}`,
    metadata_json: JSON.stringify({ file_path: filePath }),
  });

  if (!diff && !filePath) return null;
  return api("POST", `/runs/${session.runId}/hook-check`, {
    tool_name: input.tool_name || "file_edit",
    file_path: filePath,
    diff_summary: diff,
  });
}

async function finishSession(input, agent) {
  const sessionId = sessionIdFor(input, agent);
  const session = loadSession(agent, sessionId);
  if (!session) return;
  try {
    await api("POST", `/runs/${session.runId}/capture`, {});
    await api("POST", `/runs/${session.runId}/analyze`, {});
    await api("POST", `/runs/${session.runId}/events`, {
      type: "session_ended",
      message: `${agent === "claude" ? "Claude Code" : "Cursor"} session ended`,
      metadata_json: null,
    });
    if (!session.external) {
      await api("POST", `/runs/${session.runId}/finish`, {
        status: input.reason === "error" || input.status === "error" ? "failed" : "completed",
        exit_code: null,
        ending_commit: gitHead(session.cwd),
      });
    }
  } catch {
    // A shutdown hook must not make the agent unusable. The run remains visible
    // and the next Trace session can still capture the working tree.
  }
}

async function handle(input) {
  const agent = agentFor(input);
  const event = eventName(input);

  if (event === "SessionStart" || event === "sessionStart") {
    await ensureRun(input, agent);
    return;
  }
  if (event === "SessionEnd" || event === "sessionEnd") {
    await finishSession(input, agent);
    return;
  }
  if (event === "PreToolUse" && input.tool_name === "Bash") {
    await checkCommand(input, agent);
    return;
  }
  if (event === "beforeShellExecution") {
    await checkCommand(input, agent);
    return;
  }
  if (event === "PostToolUse" && ["Edit", "Write", "MultiEdit", "NotebookEdit", "Bash"].includes(input.tool_name)) {
    if (input.tool_name === "Bash") {
      const session = await ensureRun(input, agent);
      await api("POST", `/runs/${session.runId}/events`, {
        type: "tool_call_finished",
        message: `Claude Code finished: ${input.tool_input?.command || "shell command"}`,
        metadata_json: JSON.stringify({ exit_code: input.tool_response?.exit_code ?? null }),
      });
      return;
    }
    const result = await recordEdit(input, agent);
    if (result?.agent_feedback && result.block && agent === "claude") {
      process.stdout.write(JSON.stringify({
        hookSpecificOutput: { hookEventName: "PostToolUse", additionalContext: result.agent_feedback },
      }));
    }
    return;
  }
  if (event === "afterFileEdit") {
    await recordEdit(input, agent);
    return;
  }
  if (event === "afterShellExecution") {
    const session = await ensureRun(input, agent);
    await api("POST", `/runs/${session.runId}/events`, {
      type: "tool_call_finished",
      message: `Cursor finished: ${input.command || "shell command"}`,
      metadata_json: JSON.stringify({ duration_ms: input.duration ?? null }),
    });
  }
}

let raw = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => { raw += chunk; });
process.stdin.on("end", async () => {
  let input;
  try {
    input = JSON.parse(raw || "{}");
  } catch {
    return;
  }
  try {
    await handle(input);
  } catch (error) {
    const agent = agentFor(input);
    const event = eventName(input);
    if (event === "PreToolUse" || event === "beforeShellExecution") {
      const reason = `Trace could not check this action: ${error.message}`;
      if (agent === "cursor") denyCursor(reason);
      else denyClaude(reason);
    }
  }
});
