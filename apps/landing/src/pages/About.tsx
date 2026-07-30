import { Link } from "react-router-dom";
import { motion } from "framer-motion";

export default function About() {
  return (
    <div className="max-w-[700px] py-16">
      <Link to="/" className="text-sm text-text-dim hover:text-text">← Back</Link>

      <motion.h1
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, ease: [0.16, 1, 0.3, 1] }}
        className="mt-5 font-serif text-4xl text-text md:text-5xl"
      >
        The black box recorder for your AI agents
      </motion.h1>
      <motion.p
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, delay: 0.08, ease: [0.16, 1, 0.3, 1] }}
        className="mt-4 text-lg leading-relaxed text-text-dim"
      >
        You wouldn't merge a coworker's PR without reading the diff. Most people
        approve their AI agent's changes without reading anything at all —
        because there was nothing to read. Trace exists to close that gap.
      </motion.p>

      <Prose title="Why Trace exists">
        Claude Code, Codex, Cursor — they're good enough now that it's tempting to
        stop watching. That's exactly when something breaks: a dependency gets
        swapped, a config value changes, a shell command runs that you'd never
        have approved if you'd seen it coming. The agent isn't malicious. It's
        just fast, and by the time you notice, the context is gone — what
        changed, why, and what it touched along the way.
      </Prose>

      <Prose title="What Trace does">
        Trace is a desktop app that sits next to your agents, not inside them.
        Launch a session and it checkpoints your Git state, watches every file
        change as it happens, classifies the commands your agent runs, flags
        anything that looks like a secret, and totals the cost — all before you
        decide whether to keep it. If something goes wrong, rollback is one
        click, because the checkpoint was already there.
      </Prose>

      <Prose title="How it works">
        Terminal agents — Claude Code, Codex, OpenCode — are launched directly by
        the desktop app, so every command and file change is fully attributed.
        GUI tools connect in instead: Cursor over MCP, GitHub Copilot through a
        companion extension. Everything lands in the same local dashboard,
        running only on <code>127.0.0.1</code> — your code, diffs, and command
        history never leave your machine unless you explicitly wire up an
        integration, and even then only sanitized summaries go out.
      </Prose>

    </div>
  );
}

function Prose({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="mt-12 border-t border-border pt-8">
      <h2 className="font-serif text-2xl text-text">{title}</h2>
      <div className="mt-3 leading-relaxed text-text-dim [&_code]:rounded [&_code]:bg-surface [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:text-text [&_li]:list-disc [&_li]:ml-5">
        {children}
      </div>
    </section>
  );
}
