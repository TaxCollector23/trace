import { motion } from "framer-motion";
import { Section } from "./components";
import { GITHUB_REPO } from "./config";

const RELEASE_URL = `${GITHUB_REPO}/releases/latest`;
const DMG_URL = `${GITHUB_REPO}/releases/latest/download/trace-desktop-macos-arm64.dmg`;

export default function Download() {
  return (
    <Section id="download">
      <div className="grid grid-cols-1 items-center gap-10 md:grid-cols-[1fr_0.9fr]">
        <div>
          <h2 className="font-serif text-3xl text-text">Download the desktop app</h2>
          <p className="mt-3 max-w-[440px] text-text-dim">
            One native app, no terminal required. It starts its own local daemon,
            connects to Claude Code, Codex CLI, and Cursor automatically, and shows
            you everything they changed before you ship it.
          </p>

          <div className="mt-7 flex flex-wrap items-center gap-4">
            <motion.a
              href={DMG_URL}
              whileHover={{ y: -2 }}
              whileTap={{ scale: 0.96 }}
              className="btn-pop flex items-center gap-2.5 rounded-full bg-brand px-6 py-3.5 text-base font-medium text-white shadow-glow"
            >
              <AppleMark />
              Download for macOS
            </motion.a>
            <span className="text-sm text-text-dimmer">Apple Silicon · macOS 12+</span>
          </div>

          <p className="mt-5 text-sm text-text-dimmer">
            Windows and Linux are on the way.{" "}
            <a
              href={RELEASE_URL}
              target="_blank"
              rel="noreferrer"
              className="font-medium text-brand hover:text-brand-dim"
            >
              Watch releases on GitHub ↗
            </a>
          </p>
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

function AppleMark() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M16.365 1.43c0 1.14-.462 2.05-1.207 2.9-.83.95-2.1 1.688-3.298 1.598-.132-1.09.418-2.235 1.16-3.026C13.807.99 15.298.28 16.365 1.43zm3.6 16.7c-.593 1.35-1.312 2.68-2.36 3.83-.907 1-1.87 2.02-3.28 2.04-1.37.02-1.822-.86-3.4-.86-1.578 0-2.08.84-3.38.88-1.35.04-2.38-1.08-3.29-2.08-1.86-2.04-3.29-5.78-1.38-8.32.95-1.27 2.63-2.07 4.44-2.1 1.32-.02 2.56.9 3.36.9.8 0 2.3-1.11 3.87-.95.66.03 2.52.27 3.71 2.02-.1.06-2.22 1.3-2.2 3.86.02 3.06 2.68 4.08 2.72 4.1-.02.08-.42 1.46-1.4 2.68z"/>
    </svg>
  );
}

/* ── Compact preview of the desktop app window, echoing the fuller
   AppPreview shown in the hero, so this section reads as "here is the thing
   you're about to download" rather than a second unrelated illustration. ── */
function InstallPreview() {
  return (
    <div className="overflow-hidden rounded-xl border border-border bg-surface shadow-lg">
      <div className="flex items-center gap-2 border-b border-border bg-white px-4 py-2.5">
        <span className="h-2.5 w-2.5 rounded-full bg-[#ff5f57]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#febc2e]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#28c840]" />
        <span className="ml-2 text-xs font-medium text-text-dimmer">Trace.app</span>
      </div>
      <div className="space-y-2.5 p-5">
        <Row agent="Claude Code" status="good" label="reviewed · $0.04" />
        <Row agent="Cursor" status="warn" label="1 risky command" />
        <Row agent="Codex CLI" status="good" label="reviewed · $0.11" />
      </div>
    </div>
  );
}

function Row({ agent, status, label }: { agent: string; status: "good" | "warn"; label: string }) {
  const dot = status === "good" ? "bg-good" : "bg-warn";
  return (
    <div className="flex items-center justify-between rounded-lg border border-border bg-white px-3.5 py-2.5">
      <div className="flex items-center gap-2.5">
        <span className={`h-1.5 w-1.5 rounded-full ${dot}`} />
        <span className="text-sm font-medium text-text">{agent}</span>
      </div>
      <span className="text-xs text-text-dimmer">{label}</span>
    </div>
  );
}
