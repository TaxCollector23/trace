// TraceGuard browser extension — background service worker.
//
// Content scripts run on https AI sites and cannot reliably reach the local
// http daemon, so all daemon calls go through here. We discover the daemon port
// by probing the preferred range (the daemon prefers 8757 and falls back
// upward). Prompt text only ever travels to 127.0.0.1.

const PORTS = Array.from({ length: 16 }, (_, i) => 8757 + i);
let cachedPort = null;

async function findDaemon() {
  if (cachedPort) {
    if (await healthy(cachedPort)) return cachedPort;
    cachedPort = null;
  }
  for (const port of PORTS) {
    if (await healthy(port)) {
      cachedPort = port;
      return port;
    }
  }
  return null;
}

async function healthy(port) {
  try {
    const res = await fetch(`http://127.0.0.1:${port}/api/health`, {
      method: "GET",
    });
    if (!res.ok) return false;
    const j = await res.json();
    return j.status === "ok";
  } catch {
    return false;
  }
}

async function compress(prompt, mode, outputBudget) {
  const port = await findDaemon();
  if (!port) {
    return { error: "daemon_down" };
  }
  try {
    const res = await fetch(`http://127.0.0.1:${port}/api/prompt-compressor/compress`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ prompt, mode }),
    });
    if (!res.ok) return { error: `http_${res.status}` };
    const result = await res.json();

    let budgetBlock = "";
    if (outputBudget && outputBudget !== "none") {
      const b = await fetch(`http://127.0.0.1:${port}/api/prompt-compressor/output-budget`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ preset: outputBudget }),
      });
      if (b.ok) budgetBlock = (await b.json()).instruction_block || "";
    }
    return { result, budgetBlock };
  } catch (e) {
    return { error: String(e) };
  }
}

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg.type === "compress") {
    compress(msg.prompt, msg.mode, msg.outputBudget).then(sendResponse);
    return true; // async response
  }
  if (msg.type === "status") {
    findDaemon().then((port) => sendResponse({ port }));
    return true;
  }
  return false;
});
