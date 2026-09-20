// Trace's OpenCode plugin.
//
// OpenCode has two plugin contracts in the wild. The export below supports
// both: V1 uses server() and V2 uses setup() with ctx.tool.hook(). Either way,
// Bash commands are checked before execution and every active session becomes
// a Trace run on its first tool call.

import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const home = process.env.TRACE_HOME || path.join(os.homedir(), ".trace");
const sessionsDir = path.join(home, "agent-sessions", "opencode");
const sessions = new Map();

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

function gitHead(cwd) {
  try {
    const result = spawnSync("git", ["rev-parse", "HEAD"], { cwd, encoding: "utf8" });
    return result.status === 0 ? result.stdout.trim() || null : null;
  } catch {
    return null;
  }
}

function sessionId(input) {
  return String(input?.sessionID || input?.sessionId || input?.session_id || "opencode-session");
}

function sessionFile(id) {
  const key = crypto.createHash("sha256").update(id).digest("hex");
  return path.join(sessionsDir, `${key}.json`);
}

function loadSession(id) {
  if (sessions.has(id)) return sessions.get(id);
  try {
    const value = JSON.parse(fs.readFileSync(sessionFile(id), "utf8"));
    sessions.set(id, value);
    return value;
  } catch {
    return null;
  }
}

function saveSession(id, value) {
  fs.mkdirSync(sessionsDir, { recursive: true });
  fs.writeFileSync(sessionFile(id), JSON.stringify(value));
  sessions.set(id, value);
}

async function projectIdForPath(cwd) {
  const projects = await api("GET", "/projects");
  const existing = projects.find((project) => path.resolve(project.path) === path.resolve(cwd));
  if (existing) return existing.id;
  const created = await api("POST", "/projects", {
    name: path.basename(cwd) || "OpenCode project",
    path: cwd,
    config_path: path.join(cwd, ".trace", "config.toml"),
  });
  return created.id;
}

async function ensureRun(id, cwd) {
  const existing = loadSession(id);
  if (existing?.runId) return existing;
  const externalRunId = process.env.TRACE_RUN_ID || null;
  let runId = externalRunId;
  if (!runId) {
    const projectId = await projectIdForPath(cwd);
    const run = await api("POST", "/runs", {
      project_id: projectId,
      command: "OpenCode session",
      agent_name: "opencode",
      user_prompt: null,
      starting_commit: gitHead(cwd),
    });
    runId = run.id;
  }
  const session = { runId, cwd, sessionId: id, external: Boolean(externalRunId) };
  saveSession(id, session);
  await api("POST", `/runs/${runId}/events`, {
    type: "session_started",
    message: "OpenCode session started",
    metadata_json: JSON.stringify({ session_id: id }),
  });
  return session;
}

async function beforeTool(input, output, context) {
  const tool = input?.tool || context?.tool;
  const args = output?.args || input?.input || input?.args || {};
  const command = args?.command;
  const cwd = args?.cwd || input?.cwd || context?.cwd || process.cwd();
  const id = sessionId(input);
  const session = await ensureRun(id, cwd);
  if (tool !== "bash" || typeof command !== "string" || !command.trim()) return;

  const result = await api("POST", "/check-command", { command });
  await api("POST", `/runs/${session.runId}/commands`, {
    command,
    decision: result.decision,
    exit_code: null,
    stdout_path: null,
    stderr_path: null,
  });
  await api("POST", `/runs/${session.runId}/events`, {
    type: result.decision === "block" ? "risky_command_blocked" : "command_started",
    message: result.decision === "block" ? `Trace blocked: ${command}` : `OpenCode ran: ${command}`,
    metadata_json: null,
  });
  if (result.decision === "block") {
    throw new Error(`Trace blocked this command: ${result.reason || "matched a high-risk rule"}`);
  }
}

async function afterTool(input, output, context) {
  const tool = input?.tool || context?.tool;
  const cwd = input?.cwd || context?.cwd || process.cwd();
  const session = await ensureRun(sessionId(input), cwd);
  await api("POST", `/runs/${session.runId}/events`, {
    type: "tool_call_finished",
    message: `OpenCode finished ${tool || "a tool call"}`,
    metadata_json: null,
  });
  if (["edit", "write", "apply_patch"].includes(tool)) {
    await api("POST", `/runs/${session.runId}/capture`, {});
    await api("POST", `/runs/${session.runId}/analyze`, {});
  }
}

async function finishAll() {
  for (const session of sessions.values()) {
    try {
      await api("POST", `/runs/${session.runId}/capture`, {});
      await api("POST", `/runs/${session.runId}/analyze`, {});
      await api("POST", `/runs/${session.runId}/events`, {
        type: "session_ended",
        message: "OpenCode session ended",
        metadata_json: null,
      });
      if (!session.external) {
        await api("POST", `/runs/${session.runId}/finish`, {
          status: "completed",
          exit_code: null,
          ending_commit: gitHead(session.cwd),
        });
      }
    } catch {
      // Unloading a plugin must not make OpenCode unusable.
    }
  }
}

export default {
  id: "trace",

  // OpenCode V2.
  async setup(ctx) {
    await ctx.tool.hook("execute.before", async (event) => {
      await beforeTool(event, { args: event.input }, { tool: event.tool, cwd: ctx.location?.project?.canonical });
    });
    await ctx.tool.hook("execute.after", async (event) => {
      await afterTool(event, { args: event.input }, { tool: event.tool, cwd: ctx.location?.project?.canonical });
    });
    return finishAll;
  },

  // OpenCode V1 (supported by current V1-compatible releases).
  async server() {
    return {
      "tool.execute.before": async (input, output) => beforeTool(input, output),
      "tool.execute.after": async (input, output) => afterTool(input, output),
    };
  },
};
