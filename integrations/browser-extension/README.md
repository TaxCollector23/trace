# TraceGuard browser extension (TraceCompress)

Adds a TraceCompress launcher to AI prompt boxes (ChatGPT, Claude, Gemini, AI
Studio, and generic textareas/contenteditable). It compresses your prompt and
attaches Bare Mode output rules using your **local** TraceGuard daemon.

- Prompt text is sent only to `http://127.0.0.1:<port>` (your local daemon),
  never to any other server.
- It never replaces your prompt without an explicit confirmation.
- Minimal permissions: `storage`, `clipboardWrite`, and host access to
  `127.0.0.1` only.

## How it works

1. Start the daemon: `trg daemon start` (or `trg dashboard`).
2. On a supported site, click the green **TG** button (bottom-right).
3. Pick a mode (Normal / Concise / Bare Mode) and an output budget, then
   **Compress current prompt**.
4. Review the before/after token estimate and the compressed text. Click
   **Copy**, or **Replace prompt** (asks first).

If the daemon is not running, the panel tells you how to start it.

## Install (developer / unpacked)

Chrome/Edge/Brave:

1. Open `chrome://extensions`.
2. Enable **Developer mode**.
3. Click **Load unpacked** and select this `integrations/browser-extension`
   folder.

> Publishing to the Chrome Web Store requires a developer account and review;
> that step is intentionally not automated here. The unpacked extension is fully
> functional locally.

## Detection

The content script uses a layered approach: site-specific selectors first
(ChatGPT, Claude ProseMirror, Gemini Quill), then the largest visible
`textarea`, then `[contenteditable="true"]` / `[role="textbox"]`. Replacing text
in framework-controlled editors is best-effort; **Copy** always works as a
fallback.
