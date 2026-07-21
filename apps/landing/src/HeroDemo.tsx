import { useEffect, useReducer, useRef } from "react";
import { AnimatePresence, motion, useReducedMotion } from "framer-motion";

/**
 * A scripted terminal session, typed and streamed line by line, then looped.
 * The text matches the CLI's real output verbatim (see
 * crates/trace-cli/src/commands/run.rs — the "── Trace run summary ──"
 * block, the checkpoint/check lines, and `trace patch`'s diff format) rather
 * than invented marketing copy.
 */

type Line =
  | { kind: "input"; text: string }
  | { kind: "out"; text: string }
  | { kind: "add"; text: string }
  | { kind: "del"; text: string }
  | { kind: "dim"; text: string }
  | { kind: "gap" };

const SCRIPT: Line[] = [
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

const TYPE_MS = 22;
const LINE_GAP_MS = 260;
const HOLD_MS = 2600;

type State = { visible: number; typed: number; phase: "typing" | "line-pause" | "hold" | "reset" };
type Action = { type: "tick" } | { type: "reset" };

function reducer(state: State, action: Action): State {
  if (action.type === "reset") return { visible: 0, typed: 0, phase: "typing" };

  const current = SCRIPT[state.visible];
  if (!current) return { ...state, phase: "hold" };

  if (current.kind === "input" && state.phase === "typing") {
    if (state.typed < current.text.length) {
      return { ...state, typed: state.typed + 1 };
    }
    return { ...state, phase: "line-pause" };
  }

  // Non-input lines (or an input line that finished typing) just advance.
  const next = state.visible + 1;
  if (next >= SCRIPT.length) return { ...state, phase: "hold" };
  return { visible: next, typed: 0, phase: SCRIPT[next].kind === "input" ? "typing" : "line-pause" };
}

function lineDelay(state: State): number {
  if (state.phase === "hold") return HOLD_MS;
  const current = SCRIPT[state.visible];
  if (!current) return HOLD_MS;
  if (current.kind === "input" && state.phase === "typing") return TYPE_MS;
  if (current.kind === "gap") return 120;
  return LINE_GAP_MS;
}

const toneClass: Record<Line["kind"], string> = {
  input: "text-text",
  out: "text-text-dim",
  add: "text-good",
  del: "text-bad",
  dim: "text-text-dim",
  gap: "",
};

export default function HeroDemo() {
  const reduceMotion = useReducedMotion();
  const [state, dispatch] = useReducer(reducer, { visible: 0, typed: 0, phase: "typing" });
  const timer = useRef<ReturnType<typeof setTimeout>>();

  useEffect(() => {
    if (reduceMotion) return; // static final frame, no timers
    if (state.phase === "hold") {
      timer.current = setTimeout(() => dispatch({ type: "reset" }), HOLD_MS);
      return () => clearTimeout(timer.current);
    }
    timer.current = setTimeout(() => dispatch({ type: "tick" }), lineDelay(state));
    return () => clearTimeout(timer.current);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state, reduceMotion]);

  const shown = reduceMotion ? SCRIPT : SCRIPT.slice(0, state.visible + 1);
  const cursorOn = !reduceMotion && state.phase === "typing";

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
