import { useState } from "react";
import { motion } from "framer-motion";
import { Section, Button } from "./components";
import { GITHUB_REPO } from "./config";

const DMG_URL = `${GITHUB_REPO}/releases/latest/download/trace-desktop-macos-arm64.dmg`;

export default function Download() {
  return (
    <Section id="download">
      <div className="grid grid-cols-1 items-center gap-10 md:grid-cols-[1fr_0.9fr]">
        <div>
          <h2 className="font-serif text-3xl text-text">Download the desktop app</h2>
          <p className="mt-3 max-w-[440px] text-text-dim">
            One native app for macOS, no terminal required. It starts its own local
            daemon, connects to Claude Code, Codex CLI, and Cursor automatically, and
            shows you everything they changed before you ship it.
          </p>

          <div className="mt-7 flex flex-wrap items-center gap-4">
            <Button href={DMG_URL}>
              <img src="/logos/macosapple.png" alt="" className="h-4 w-4 object-contain" />
              Download for macOS
            </Button>
            <span className="text-sm text-text-dim">Apple Silicon · macOS 12+</span>
          </div>
        </div>

        <motion.div
          initial={{ opacity: 0, y: 12 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-60px" }}
          transition={{ duration: 0.5, ease: [0.16, 1, 0.3, 1] }}
        >
          <InstallPreview />
        </motion.div>
      </div>
    </Section>
  );
}

/* ── Compact, clickable preview of the desktop app window, focused on a
   single Cursor session — a different agent from the hero's Claude Code
   mockup so the page doesn't repeat the same example twice. ── */
function InstallPreview() {
  const [decision, setDecision] = useState<"pending" | "approved" | "blocked">("pending");

  return (
    <div className="overflow-hidden rounded-xl border border-border bg-white shadow-lg">
      <div className="flex items-center gap-2 border-b border-border bg-surface px-4 py-3">
        <span className="h-2.5 w-2.5 rounded-full bg-[#ff5f57]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#febc2e]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#28c840]" />
        <span className="ml-2 text-xs font-medium text-text-dim">Trace — Cursor Session</span>
      </div>
      <div className="p-5">
        <div>
          <div className="text-sm font-semibold text-text">Cursor</div>
          <div className="font-mono text-xs text-text-dim">add pagination to /api/users</div>
        </div>

        <div className="mt-4 rounded-lg border border-border bg-surface px-3.5 py-2.5">
          <div className="flex items-center justify-between">
            <span className="font-mono text-xs text-text-dim">rm -rf node_modules &amp;&amp; npm install</span>
            {decision === "pending" ? (
              <span className="rounded-full bg-warn-soft px-2 py-0.5 text-[11px] font-medium text-warn">flagged</span>
            ) : decision === "approved" ? (
              <span className="rounded-full bg-good-soft px-2 py-0.5 text-[11px] font-medium text-good">approved</span>
            ) : (
              <span className="rounded-full bg-bad-soft px-2 py-0.5 text-[11px] font-medium text-bad">blocked</span>
            )}
          </div>
          {decision === "pending" && (
            <div className="mt-2.5 flex gap-2">
              <button
                onClick={() => setDecision("approved")}
                className="btn-pop flex-1 rounded-md bg-good-soft py-1.5 text-xs font-medium text-good"
              >
                Approve
              </button>
              <button
                onClick={() => setDecision("blocked")}
                className="btn-pop flex-1 rounded-md bg-bad-soft py-1.5 text-xs font-medium text-bad"
              >
                Block
              </button>
            </div>
          )}
        </div>

        <div className="mt-4 grid grid-cols-2 gap-2.5">
          <div className="rounded-lg border border-border px-3 py-2.5 text-center">
            <div className="text-base font-semibold text-text">5</div>
            <div className="mt-0.5 text-[10px] uppercase tracking-wide text-text-dim">Files changed</div>
          </div>
          <div className="rounded-lg border border-border px-3 py-2.5 text-center">
            <div className="text-base font-semibold text-warn">Medium</div>
            <div className="mt-0.5 text-[10px] uppercase tracking-wide text-text-dim">Risk</div>
          </div>
        </div>
      </div>
    </div>
  );
}
