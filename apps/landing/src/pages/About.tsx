import { Link } from "react-router-dom";
import { motion } from "framer-motion";
import { Reveal } from "../components";
import { GITHUB_REPO } from "../config";

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

      <Prose title="What it does not do">
        <ul className="space-y-1.5">
          <li>It does not write code or replace your agent — it watches, records, and reviews.</li>
          <li>It does not upload your project, your secrets, or your local database anywhere.</li>
          <li>It does not require an account, a login, or a subscription to use today.</li>
          <li>It does not claim to fully sandbox GUI tools — the strongest guarantees are for agents Trace launches directly.</li>
        </ul>
      </Prose>

      <Prose title="Where this is going">
        Today Trace is a single developer's desktop app. The roadmap is a team
        tier — shared policy rules, an organization-wide view of what agents
        changed and where, sanitized audit trails, and role-based access — for
        teams who've decided AI-written code needs the same review discipline as
        human-written code, just automated.
      </Prose>

      <Reveal>
        <div className="mt-14 rounded-xl border border-border bg-surface p-6">
          <p className="text-sm leading-relaxed text-text-dim">
            Trace is built by developers who wanted to keep shipping fast with AI
            agents without losing the habit of knowing what actually happened.
            If you want the fuller story of why, <a href={`${GITHUB_REPO}/discussions`} target="_blank" rel="noreferrer" className="font-medium text-brand hover:text-brand-dim">start a discussion</a> — always glad to talk about it.
          </p>
        </div>
      </Reveal>
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
