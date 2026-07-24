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
            <span className="text-sm text-text-dimmer">Apple Silicon · macOS 12+</span>
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

/* ── Compact preview of the desktop app window, focused on a single Cursor
   session — a different agent from the hero's Claude Code mockup so the
   page doesn't repeat the same example twice. ── */
function InstallPreview() {
  return (
    <div className="overflow-hidden rounded-xl border border-border bg-white shadow-lg">
      <div className="flex items-center gap-2 border-b border-border bg-surface px-4 py-3">
        <span className="h-2.5 w-2.5 rounded-full bg-[#ff5f57]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#febc2e]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#28c840]" />
        <span className="ml-2 text-xs font-medium text-text-dimmer">Trace — Cursor Session</span>
      </div>
      <div className="p-5">
        <div className="flex items-center gap-2.5">
          <img src="/logos/cursor.png" alt="" className="h-7 w-7 rounded-md" />
          <div>
            <div className="text-sm font-semibold text-text">Cursor</div>
            <div className="font-mono text-xs text-text-dimmer">add pagination to /api/users</div>
          </div>
        </div>

        <div className="mt-4 flex items-center justify-between rounded-lg border border-border bg-surface px-3.5 py-2.5">
          <span className="font-mono text-xs text-text-dim">1 command flagged</span>
          <span className="rounded-full bg-warn-soft px-2 py-0.5 text-[11px] font-medium text-warn">review</span>
        </div>
        <div className="mt-2 rounded-lg border border-border bg-surface px-3.5 py-2.5 font-mono text-xs text-text-dim">
          rm -rf node_modules &amp;&amp; npm install
        </div>

        <div className="mt-4 grid grid-cols-2 gap-2.5">
          <div className="rounded-lg border border-border px-3 py-2.5 text-center">
            <div className="font-serif text-xl text-text">5</div>
            <div className="mt-0.5 text-[10px] uppercase tracking-wide text-text-dimmer">Files changed</div>
          </div>
          <div className="rounded-lg border border-border px-3 py-2.5 text-center">
            <div className="font-serif text-xl text-warn">Medium</div>
            <div className="mt-0.5 text-[10px] uppercase tracking-wide text-text-dimmer">Risk</div>
          </div>
        </div>
      </div>
    </div>
  );
}
