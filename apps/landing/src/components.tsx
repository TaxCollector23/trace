import { useState, type ReactNode } from "react";
import { motion } from "framer-motion";
import Tracey, { type TraceyExpression } from "./Tracey";

/** A copyable command block. Developer-tool look, no decoration — the
 * darker fill alone marks it as a distinct block, no border needed. */
export function Cmd({ children }: { children: string }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(children);
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    } catch {
      /* clipboard may be unavailable; ignore */
    }
  };
  return (
    <div className="flex items-center justify-between gap-3 rounded bg-black/40 px-4 py-2.5">
      <code className="overflow-x-auto text-sm text-text">
        <span className="mr-2 select-none text-brand">$</span>
        {children}
      </code>
      <button
        onClick={copy}
        aria-label="Copy command"
        className="shrink-0 rounded-sm bg-surface px-2.5 py-1 text-xs text-text-dim hover:text-text"
      >
        {copied ? "copied" : "copy"}
      </button>
    </div>
  );
}

export function Section({
  id,
  title,
  lede,
  children,
  tracey,
}: {
  id?: string;
  title: string;
  lede?: ReactNode;
  children: ReactNode;
  /** Renders Tracey peeking from a corner of this section. */
  tracey?: { expression?: TraceyExpression; size?: number; corner?: "bottom-right" | "bottom-left" | "top-right" | "top-left" };
}) {
  return (
    <section id={id} className="relative py-16">
      {tracey && <TraceyPeek {...tracey} />}
      <h2 className="text-2xl font-semibold">{title}</h2>
      {lede && <p className="mt-1.5 max-w-[640px] text-text-dim">{lede}</p>}
      <div className="relative mt-6">{children}</div>
    </section>
  );
}

/** Fades a section's contents in as it enters the viewport. One pass, no loop. */
export function Reveal({
  children,
  delay = 0,
  className,
}: {
  children: ReactNode;
  delay?: number;
  className?: string;
}) {
  return (
    <motion.div
      className={className}
      initial={{ opacity: 0, y: 8 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-80px" }}
      transition={{ duration: 0.4, delay, ease: [0.16, 1, 0.3, 1] }}
    >
      {children}
    </motion.div>
  );
}

/**
 * Tracey, peeking out from behind whatever she's placed inside — used
 * throughout the scroll story so she shows up again each time a new section
 * enters the viewport, always tucked half-behind the real content rather
 * than floating on top of it.
 */
export function TraceyPeek({
  expression = "peek",
  size = 96,
  corner = "bottom-right",
  className = "",
}: {
  expression?: TraceyExpression;
  size?: number;
  corner?: "bottom-right" | "bottom-left" | "top-right" | "top-left";
  className?: string;
}) {
  const pos: Record<string, string> = {
    "bottom-right": "-bottom-8 -right-6 md:-right-10",
    "bottom-left": "-bottom-8 -left-6 md:-left-10",
    "top-right": "-top-8 -right-6 md:-right-10",
    "top-left": "-top-8 -left-6 md:-left-10",
  };
  return (
    <motion.div
      className={`pointer-events-none absolute z-0 ${pos[corner]} ${className}`}
      initial={{ opacity: 0, y: 16, scale: 0.9 }}
      whileInView={{ opacity: 1, y: 0, scale: 1 }}
      viewport={{ once: false, margin: "-100px" }}
      transition={{ duration: 0.5, ease: [0.16, 1, 0.3, 1] }}
      aria-hidden="true"
    >
      <Tracey expression={expression} size={size} />
    </motion.div>
  );
}
