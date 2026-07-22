import { useEffect, useReducer, useRef } from "react";
import { useReducedMotion } from "framer-motion";

/** A single line in a scripted terminal transcript. */
export type TermLine =
  | { kind: "input"; text: string }
  | { kind: "out"; text: string }
  | { kind: "add"; text: string }
  | { kind: "del"; text: string }
  | { kind: "dim"; text: string }
  | { kind: "gap" };

type State = { visible: number; typed: number; phase: "typing" | "line-pause" | "hold" | "reset" };
type Action = { type: "tick" } | { type: "reset" };

export interface TypedTerminalOptions {
  typeMs?: number;
  lineGapMs?: number;
  holdMs?: number;
  gapMs?: number;
}

/**
 * Drives a scripted terminal transcript: types "input" lines character by
 * character, reveals other lines instantly with a short pause between them,
 * then holds on the finished frame before looping. Shared by every landing
 * section that shows a fake-but-accurate terminal (hero, CLI showcase).
 */
export function useTypedTerminal(script: TermLine[], opts: TypedTerminalOptions = {}) {
  const { typeMs = 22, lineGapMs = 260, holdMs = 2600, gapMs = 120 } = opts;
  const reduceMotion = useReducedMotion();
  const [state, dispatch] = useReducer(reducer, { visible: 0, typed: 0, phase: "typing" });
  const timer = useRef<ReturnType<typeof setTimeout>>();

  function reducer(s: State, action: Action): State {
    if (action.type === "reset") return { visible: 0, typed: 0, phase: "typing" };
    const current = script[s.visible];
    if (!current) return { ...s, phase: "hold" };
    if (current.kind === "input" && s.phase === "typing") {
      if (s.typed < current.text.length) return { ...s, typed: s.typed + 1 };
      return { ...s, phase: "line-pause" };
    }
    const next = s.visible + 1;
    if (next >= script.length) return { ...s, phase: "hold" };
    return { visible: next, typed: 0, phase: script[next].kind === "input" ? "typing" : "line-pause" };
  }

  function lineDelay(s: State): number {
    if (s.phase === "hold") return holdMs;
    const current = script[s.visible];
    if (!current) return holdMs;
    if (current.kind === "input" && s.phase === "typing") return typeMs;
    if (current.kind === "gap") return gapMs;
    return lineGapMs;
  }

  useEffect(() => {
    if (reduceMotion) return;
    if (state.phase === "hold") {
      timer.current = setTimeout(() => dispatch({ type: "reset" }), holdMs);
      return () => clearTimeout(timer.current);
    }
    timer.current = setTimeout(() => dispatch({ type: "tick" }), lineDelay(state));
    return () => clearTimeout(timer.current);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state, reduceMotion]);

  const shown = reduceMotion ? script : script.slice(0, state.visible + 1);
  const cursorOn = !reduceMotion && state.phase === "typing";

  return { shown, state, reduceMotion, cursorOn };
}

export const toneClass: Record<TermLine["kind"], string> = {
  input: "text-text",
  out: "text-text-dim",
  add: "text-good",
  del: "text-bad",
  dim: "text-text-dim",
  gap: "",
};
