import { AnimatePresence, motion } from "framer-motion";
import { toneClass, useTypedTerminal, type TermLine } from "./useTypedTerminal";

/**
 * A scripted terminal session, typed and streamed line by line, then looped.
 * The text matches the CLI's real output verbatim (see
 * crates/trace-cli/src/commands/run.rs — the "── Trace run summary ──"
 * block, the checkpoint/check lines, and `trace patch`'s diff format) rather
 * than invented marketing copy.
 */
const SCRIPT: TermLine[] = [
  { kind: "input", text: 'trace run "claude fix the login bug"' },
  { kind: "out", text: "Checkpoint created at a3f9c21" },
  { kind: "out", text: "Watching file changes…" },
  { kind: "gap" },
  { kind: "input", text: "trace patch a3f9c21" },
  { kind: "dim", text: "src/auth/login.ts" },
  { kind: "del", text: "- if (user.token = null) {" },
  { kind: "add", text: "+ if (user.token === null) {" },
  { kind: "out", text: "    return unauthorized();" },
  { kind: "dim", text: "  }" },
  { kind: "gap" },
  { kind: "dim", text: "── Trace run summary ──" },
  { kind: "out", text: "  status:    completed (exit 0)" },
  { kind: "out", text: "  changes:   1 file changed" },
  { kind: "out", text: "  cost:      $0.02" },
];

export default function HeroDemo() {
  const { shown, state, reduceMotion, cursorOn } = useTypedTerminal(SCRIPT);

  return (
    <div className="overflow-hidden rounded-md border border-border bg-surface">
      <div className="flex items-center gap-2 border-b border-border bg-black/30 px-3.5 py-2.5">
        <span className="h-2.5 w-2.5 rounded-full bg-bad" />
        <span className="h-2.5 w-2.5 rounded-full bg-warn" />
        <span className="h-2.5 w-2.5 rounded-full bg-good" />
        <span className="ml-2 font-mono text-xs text-text-dim">trace — zsh</span>
      </div>
      <div className="min-h-[300px] p-4 font-mono text-[13px] leading-[1.7]">
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
  );
}
