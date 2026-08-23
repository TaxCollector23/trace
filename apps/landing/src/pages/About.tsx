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
        You gave an AI write access to your codebase.
      </motion.h1>
      <motion.p
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, delay: 0.08, ease: [0.16, 1, 0.3, 1] }}
        className="mt-4 text-lg leading-relaxed text-text-dim"
      >
        It can edit any file, run any command, and touch your git history, and it does it in seconds, faster than anyone can read. Trace is the record of
        what it actually did, and the brake that stops the one command you'd never
        have approved.
      </motion.p>

      <Prose title="This is the new normal">
        Companies are handing every engineer an enterprise subscription to an
        autonomous coding agent and telling them to move faster. Those agents
        don't just suggest code anymore. They have a shell. They install
        packages, rewrite configs, run migrations, delete files, and push
        branches, all on a real machine with real credentials. Multiply that by a
        whole engineering org and you have thousands of unattended sessions a day,
        each one trusted with write access to production code.
      </Prose>

      <Prose title="The failure nobody budgets for">
        The agent isn't malicious. It's confident and it's fast, which is exactly
        the problem. One session decides the cleanest fix is to wipe a directory
        and regenerate it, and quietly takes the wrong directory with it. Another
        force-pushes over a teammate's branch. Another deletes the test file that
        was failing instead of the bug that was failing it, pastes an API key into
        a committed config, or runs a <code>curl ... | sh</code> from an answer it
        found. By the time a human looks up, the change is done and the context (what changed, why, and what it touched on the way) is already gone.
        <br />
        <br />
        You wouldn't merge a coworker's PR without reading the diff. Most people
        approve an agent's work without reading anything at all, because there was
        nothing to read. That is the gap Trace closes.
      </Prose>

      <Prose title="What Trace does about it">
        Trace sits beside your agents, whoever makes them, and turns every session
        into something you can see, stop, and undo:
        <ul className="mt-3 space-y-2">
          <li>
            <b>A checkpoint before anything changes.</b> Trace snapshots your git
            state at the start of every run, so "undo the last hour" is one click, and the safety net is already in place before the first edit lands.
          </li>
          <li>
            <b>Every file change, as a diff you can read.</b> The authoritative
            patch for the whole session, in one place, with additions and
            deletions per file, so you review it like a PR instead of trusting a summary.
          </li>
          <li>
            <b>A guard on every command.</b> Each command the agent runs is
            classified in real time as allow, warn, require approval, or block. The
            destructive ones (<code>rm -rf</code>, a force-push, a piped shell
            installer, a raw disk write) are stopped <i>before</i> they execute,
            not reported after.
          </li>
          <li>
            <b>Secret scanning.</b> Keys, tokens, and credentials that show up in a
            diff are caught and redacted before they're stored or shipped.
          </li>
          <li>
            <b>Cost, per session.</b> Token usage and spend, broken down by model,
            so an agent burning money in a loop is visible immediately.
          </li>
          <li>
            <b>PR ratification.</b> Run the exact same checks over a pull request, from the dashboard or in CI, and get a clear <b>pass / review / block</b>{" "}
            verdict with every finding and its severity.
          </li>
          <li>
            <b>One-click rollback.</b> Any checkpoint, restored in a click, because
            it was captured the moment the run began.
          </li>
        </ul>
      </Prose>

      <Prose title="Deterministic, local, and yours">
        Every check is plain pattern matching: a rule against the diff, a
        normalized-command lookup, a secret detector. There is <b>no LLM in the
        guard path and no API key</b>, so a verdict is instant, free, and identical
        on every machine. A guardrail you can trust in the tool-call path cannot
        depend on a slow, paid, non-deterministic third party. And it's measured,
        not asserted: Trace ships a labeled red-team corpus of dangerous commands,
        planted secrets, and unsafe prompts, and you can reproduce every detection
        number yourself. It all runs on <code>127.0.0.1</code>. Your code, diffs,
        and command history never leave your machine unless you explicitly wire up
        an integration, and even then only sanitized summaries go out.
      </Prose>

      <div className="mt-12 border-t border-border pt-8">
        <p className="text-text-dim">
          Whoever your agent is, you should be able to see what it did.{" "}
          <Link to="/#install" className="text-brand hover:text-brand-dim">
            Install the CLI →
          </Link>
        </p>
      </div>
    </div>
  );
}

function Prose({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="mt-12 border-t border-border pt-8">
      <h2 className="font-serif text-2xl text-text">{title}</h2>
      <div className="mt-3 leading-relaxed text-text-dim [&_code]:rounded [&_code]:bg-surface [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:text-text [&_li]:ml-5 [&_li]:list-disc">
        {children}
      </div>
    </section>
  );
}
