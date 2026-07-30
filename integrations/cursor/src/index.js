#!/usr/bin/env node
// TraceGuard MCP server for Cursor (and any MCP client).
//
// Dependency-free stdio JSON-RPC 2.0 server (newline-delimited messages). It
// exposes TraceGuard operations as MCP tools, each backed by the local daemon at
// http://127.0.0.1:<port> (port read from ~/.traceguard/daemon.json). It never
// connects to any remote service.

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import readline from "node:readline";

function daemonBase() {
  try {
    const p = path.join(os.homedir(), ".traceguard", "daemon.json");
    const s = JSON.parse(fs.readFileSync(p, "utf8"));
    if (s && s.port) return `http://127.0.0.1:${s.port}`;
  } catch {
    /* not running */
  }
  return null;
}

async function api(method, route, body) {
  const base = daemonBase();
  if (!base) throw new Error("TraceGuard daemon is not running. Run `trg dashboard`.");
  const res = await fetch(`${base}/api${route}`, {
    method,
    headers: body ? { "content-type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) throw new Error(`daemon ${method} ${route} -> ${res.status}`);
  return res.json();
}

async function projectIdForPath(projectPath) {
  const projects = await api("GET", "/projects");
  const match = projects.find((p) => p.path === projectPath);
  if (!match) throw new Error(`No TraceGuard project at ${projectPath}. Run \`trg init\` there.`);
  return match.id;
}

const TOOLS = {
  traceguard_check_command: {
    description: "Classify a shell command with TraceGuard's guard rules (allow/warn/require_approval/block).",
    inputSchema: {
      type: "object",
      properties: { command: { type: "string" } },
      required: ["command"],
    },
    handler: (args) => api("POST", "/check-command", { command: args.command }),
  },
  traceguard_get_recent_runs: {
    description: "List recent monitored runs with status and counts.",
    inputSchema: { type: "object", properties: {} },
    handler: () => api("GET", "/runs?limit=25"),
  },
  traceguard_start_run: {
    description: "Start a TraceGuard run for a project path. Returns the run id.",
    inputSchema: {
      type: "object",
      properties: {
        project_path: { type: "string" },
        command: { type: "string" },
        agent_name: { type: "string" },
        user_prompt: { type: "string" },
      },
      required: ["project_path", "command"],
    },
    handler: async (args) => {
      const project_id = await projectIdForPath(args.project_path);
      return api("POST", "/runs", {
        project_id,
        command: args.command,
        agent_name: args.agent_name ?? null,
        user_prompt: args.user_prompt ?? null,
        starting_commit: null,
      });
    },
  },
  traceguard_end_run: {
    description: "Finalize a run with a status (completed/failed/blocked).",
    inputSchema: {
      type: "object",
      properties: {
        run_id: { type: "string" },
        status: { type: "string" },
        exit_code: { type: "number" },
      },
      required: ["run_id", "status"],
    },
    handler: (args) =>
      api("POST", `/runs/${args.run_id}/finish`, {
        status: args.status,
        exit_code: args.exit_code ?? null,
        ending_commit: null,
      }),
  },
  traceguard_record_event: {
    description: "Record a timeline event on a run.",
    inputSchema: {
      type: "object",
      properties: {
        run_id: { type: "string" },
        type: { type: "string" },
        message: { type: "string" },
      },
      required: ["run_id", "type", "message"],
    },
    handler: (args) =>
      api("POST", `/runs/${args.run_id}/events`, {
        type: args.type,
        message: args.message,
        metadata_json: null,
      }),
  },
  traceguard_get_patch_summary: {
    description: "Get the list of file changes for a run.",
    inputSchema: {
      type: "object",
      properties: { run_id: { type: "string" } },
      required: ["run_id"],
    },
    handler: (args) => api("GET", `/runs/${args.run_id}/file-changes`),
  },
  traceguard_get_rollback_options: {
    description: "List runs that have a checkpoint available for rollback.",
    inputSchema: { type: "object", properties: {} },
    handler: async () => {
      const runs = await api("GET", "/runs?limit=25");
      const options = [];
      for (const r of runs) {
        const cps = await api("GET", `/runs/${r.id}/checkpoints`);
        const cp = [...cps].reverse().find((c) => c.git_ref);
        if (cp) options.push({ run_id: r.id, command: r.command, git_ref: cp.git_ref });
      }
      return options;
    },
  },
};

function send(msg) {
  process.stdout.write(JSON.stringify(msg) + "\n");
}

function result(id, value) {
  send({
    jsonrpc: "2.0",
    id,
    result: { content: [{ type: "text", text: JSON.stringify(value, null, 2) }] },
  });
}

async function handle(msg) {
  const { id, method, params } = msg;
  switch (method) {
    case "initialize":
      send({
        jsonrpc: "2.0",
        id,
        result: {
          protocolVersion: "2024-11-05",
          capabilities: { tools: {} },
          serverInfo: { name: "traceguard", version: "0.1.0" },
        },
      });
      break;
    case "notifications/initialized":
      break; // notification, no response
    case "tools/list":
      send({
        jsonrpc: "2.0",
        id,
        result: {
          tools: Object.entries(TOOLS).map(([name, t]) => ({
            name,
            description: t.description,
            inputSchema: t.inputSchema,
          })),
        },
      });
      break;
    case "tools/call": {
      const tool = TOOLS[params?.name];
      if (!tool) {
        send({ jsonrpc: "2.0", id, error: { code: -32601, message: "unknown tool" } });
        return;
      }
      try {
        const value = await tool.handler(params.arguments || {});
        result(id, value);
      } catch (e) {
        send({
          jsonrpc: "2.0",
          id,
          result: {
            content: [{ type: "text", text: `Error: ${e.message}` }],
            isError: true,
          },
        });
      }
      break;
    }
    default:
      if (id !== undefined) {
        send({ jsonrpc: "2.0", id, error: { code: -32601, message: `unknown method ${method}` } });
      }
  }
}

const rl = readline.createInterface({ input: process.stdin });
rl.on("line", (line) => {
  const trimmed = line.trim();
  if (!trimmed) return;
  let msg;
  try {
    msg = JSON.parse(trimmed);
  } catch {
    return;
  }
  handle(msg).catch(() => {});
});
