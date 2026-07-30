// TraceGuard content script — adds a TraceCompress launcher to AI prompt boxes.
//
// It never sends prompt text anywhere except the local daemon (via the
// background worker), and never replaces your prompt without confirmation.

(function () {
  if (window.__traceguardInjected) return;
  window.__traceguardInjected = true;

  // ---- Layered prompt-box detection --------------------------------------
  const SITE_SELECTORS = [
    "#prompt-textarea", // ChatGPT
    'div[contenteditable="true"].ProseMirror', // ChatGPT / Claude ProseMirror
    'div.ql-editor[contenteditable="true"]', // Gemini (Quill)
    'textarea[aria-label]',
    'textarea[placeholder]',
  ];

  function visible(el) {
    if (!el) return false;
    const r = el.getBoundingClientRect();
    return r.width > 120 && r.height > 16 && r.bottom > 0;
  }

  function findPromptBox() {
    for (const sel of SITE_SELECTORS) {
      const el = [...document.querySelectorAll(sel)].filter(visible).pop();
      if (el) return el;
    }
    // Generic: largest visible textarea.
    const ta = [...document.querySelectorAll("textarea")]
      .filter(visible)
      .sort((a, b) => area(b) - area(a))[0];
    if (ta) return ta;
    // Generic: a focusable contenteditable / role=textbox.
    const ce = [...document.querySelectorAll('[contenteditable="true"], [role="textbox"]')]
      .filter(visible)
      .sort((a, b) => area(b) - area(a))[0];
    return ce || null;
  }

  function area(el) {
    const r = el.getBoundingClientRect();
    return r.width * r.height;
  }

  function getText(el) {
    if (!el) return "";
    if (el.tagName === "TEXTAREA") return el.value;
    return el.innerText;
  }

  function setText(el, text) {
    if (!el) return false;
    el.focus();
    if (el.tagName === "TEXTAREA") {
      const setter = Object.getOwnPropertyDescriptor(
        window.HTMLTextAreaElement.prototype,
        "value"
      ).set;
      setter.call(el, text);
      el.dispatchEvent(new Event("input", { bubbles: true }));
      return true;
    }
    // contenteditable: select-all then insert so the editor's framework updates.
    try {
      const sel = window.getSelection();
      const range = document.createRange();
      range.selectNodeContents(el);
      sel.removeAllRanges();
      sel.addRange(range);
      document.execCommand("insertText", false, text);
      return true;
    } catch {
      el.textContent = text;
      el.dispatchEvent(new Event("input", { bubbles: true }));
      return true;
    }
  }

  // ---- UI -----------------------------------------------------------------
  const launcher = document.createElement("button");
  launcher.className = "tg-launcher";
  launcher.title = "TraceCompress this prompt";
  launcher.textContent = "TG";
  document.body.appendChild(launcher);

  const panel = document.createElement("div");
  panel.className = "tg-panel tg-hidden";
  panel.innerHTML = `
    <div class="tg-head">
      <span>TraceCompress</span>
      <button class="tg-close" aria-label="Close">×</button>
    </div>
    <div class="tg-row">
      <label>Mode
        <select class="tg-mode">
          <option value="normal">Normal</option>
          <option value="concise" selected>Concise</option>
          <option value="bare">Bare Mode</option>
        </select>
      </label>
      <label>Output budget
        <select class="tg-budget">
          <option value="none">None</option>
          <option value="tiny">Tiny</option>
          <option value="short" selected>Short</option>
          <option value="normal">Normal</option>
          <option value="detailed">Detailed</option>
        </select>
      </label>
    </div>
    <button class="tg-btn tg-compress">Compress current prompt</button>
    <div class="tg-status"></div>
    <div class="tg-result tg-hidden">
      <div class="tg-est"></div>
      <textarea class="tg-out" rows="6" readonly></textarea>
      <div class="tg-actions">
        <button class="tg-btn tg-copy">Copy</button>
        <button class="tg-btn tg-replace">Replace prompt</button>
      </div>
      <div class="tg-conflicts tg-hidden"></div>
    </div>
  `;
  document.body.appendChild(panel);

  const $ = (s) => panel.querySelector(s);
  let lastOutput = "";

  launcher.addEventListener("click", () => panel.classList.toggle("tg-hidden"));
  $(".tg-close").addEventListener("click", () => panel.classList.add("tg-hidden"));

  $(".tg-compress").addEventListener("click", async () => {
    const box = findPromptBox();
    const prompt = getText(box).trim();
    const status = $(".tg-status");
    if (!prompt) {
      status.textContent = "No prompt text found in the box.";
      return;
    }
    status.textContent = "Compressing locally…";
    const mode = $(".tg-mode").value;
    const outputBudget = $(".tg-budget").value;

    const resp = await chrome.runtime.sendMessage({
      type: "compress",
      prompt,
      mode,
      outputBudget,
    });

    if (resp.error === "daemon_down") {
      status.innerHTML =
        "TraceGuard daemon is not running. Start it with <code>trg dashboard</code> or <code>trg daemon start</code>.";
      return;
    }
    if (resp.error) {
      status.textContent = "Error: " + resp.error;
      return;
    }

    const { result, budgetBlock } = resp;
    let out = result.compressed;
    if (budgetBlock) out += "\n\n" + budgetBlock;
    if (mode === "bare" || result.response_rules) out += "\n\n" + result.response_rules;
    lastOutput = out;

    status.textContent = "";
    $(".tg-result").classList.remove("tg-hidden");
    $(".tg-est").textContent = `~${result.original_tokens} → ~${result.compressed_tokens} tokens (~${Math.round(
      result.reduction_pct
    )}% smaller). Estimates.`;
    $(".tg-out").value = out;

    const conf = $(".tg-conflicts");
    if (result.conflicts && result.conflicts.length) {
      conf.classList.remove("tg-hidden");
      conf.innerHTML =
        "⚠ Possible conflicts: " + result.conflicts.map((c) => c).join("; ");
    } else {
      conf.classList.add("tg-hidden");
    }
  });

  $(".tg-copy").addEventListener("click", () => {
    navigator.clipboard.writeText(lastOutput);
    $(".tg-status").textContent = "Copied.";
  });

  $(".tg-replace").addEventListener("click", () => {
    if (!lastOutput) return;
    if (!confirm("Replace your prompt box text with the compressed version?")) return;
    const box = findPromptBox();
    const ok = setText(box, lastOutput);
    $(".tg-status").textContent = ok
      ? "Prompt replaced."
      : "Could not replace automatically — use Copy instead.";
  });
})();
