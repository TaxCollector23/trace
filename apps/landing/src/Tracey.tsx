import { useEffect, useState } from "react";
import { motion, useReducedMotion } from "framer-motion";

export type TraceyExpression = "happy" | "sleeping" | "celebrate" | "watching" | "thinking";

/**
 * Tracey — Trace's mascot. Original flat-color SVG, matte, no gradients.
 * One accent blue, big expressive eyes, tiny feet, a single curved antenna.
 * Recognizable from the body silhouette alone; eyes/mouth carry expression.
 *
 * There should only ever be one Tracey visible at a time — this component
 * doesn't enforce that, callers do, by only ever mounting one instance.
 */
export default function Tracey({
  expression = "happy",
  size = 32,
  className,
}: {
  expression?: TraceyExpression;
  size?: number;
  className?: string;
}) {
  const reduceMotion = useReducedMotion();
  const [blink, setBlink] = useState(false);
  const [tilt, setTilt] = useState(0);

  // Idle animation: an occasional blink or head tilt, under 1s, every 8-15s.
  // Sleeping already has closed eyes, so it skips the blink cycle.
  useEffect(() => {
    if (reduceMotion || expression === "sleeping") return;
    let cancelled = false;
    const scheduleNext = () => {
      const delay = 8000 + Math.random() * 7000;
      window.setTimeout(() => {
        if (cancelled) return;
        if (Math.random() < 0.6) {
          setBlink(true);
          window.setTimeout(() => !cancelled && setBlink(false), 140);
        } else {
          setTilt(Math.random() < 0.5 ? -6 : 6);
          window.setTimeout(() => !cancelled && setTilt(0), 500);
        }
        scheduleNext();
      }, delay);
    };
    scheduleNext();
    return () => {
      cancelled = true;
    };
  }, [reduceMotion, expression]);

  const eyesClosed = expression === "sleeping" || blink;

  return (
    <motion.svg
      viewBox="0 0 120 130"
      width={size}
      height={(size * 130) / 120}
      className={className}
      role="img"
      aria-label={`Tracey, ${expression}`}
      animate={reduceMotion ? undefined : { rotate: tilt }}
      transition={{ duration: 0.4, ease: "easeInOut" }}
      style={{ transformOrigin: "60px 120px" }}
    >
      {/* antenna */}
      <path d="M60 24 Q79 8 90 21" stroke="#4fa3ff" strokeWidth="6" fill="none" strokeLinecap="round" />
      <circle cx="91" cy="20" r="7" fill="#4fa3ff" />

      {/* feet */}
      <ellipse cx="42" cy="112" rx="13" ry="9" fill="#4fa3ff" />
      <ellipse cx="78" cy="112" rx="13" ry="9" fill="#4fa3ff" />

      {/* body */}
      <ellipse cx="60" cy="70" rx="44" ry="50" fill="#4fa3ff" />

      {/* celebrate: tiny confetti, otherwise omitted entirely */}
      {expression === "celebrate" && (
        <g>
          <rect x="14" y="18" width="5" height="5" fill="#22c55e" transform="rotate(20 16 20)" />
          <rect x="100" y="34" width="5" height="5" fill="#f59e0b" transform="rotate(-15 102 36)" />
          <rect x="24" y="42" width="4" height="4" fill="#ef4444" transform="rotate(45 26 44)" />
        </g>
      )}

      {/* eyes */}
      {eyesClosed ? (
        <>
          <path d="M36 66 Q46 72 56 66" stroke="#111113" strokeWidth="4" fill="none" strokeLinecap="round" />
          <path d="M64 66 Q74 72 84 66" stroke="#111113" strokeWidth="4" fill="none" strokeLinecap="round" />
        </>
      ) : (
        <>
          <circle cx="46" cy="66" r="12" fill="#fafafa" />
          <circle cx="74" cy="66" r="12" fill="#fafafa" />
          <circle cx={expression === "watching" ? 49 : 46} cy="68" r="5.5" fill="#111113" />
          <circle cx={expression === "watching" ? 77 : 74} cy="68" r="5.5" fill="#111113" />
        </>
      )}

      {/* mouth */}
      {expression === "happy" && (
        <path d="M48 88 Q60 98 72 88" stroke="#111113" strokeWidth="4" fill="none" strokeLinecap="round" />
      )}
      {expression === "sleeping" && <path d="M54 90 H66" stroke="#111113" strokeWidth="4" strokeLinecap="round" />}
      {expression === "celebrate" && (
        <path d="M46 86 Q60 102 74 86" stroke="#111113" strokeWidth="4" fill="none" strokeLinecap="round" />
      )}
      {expression === "watching" && (
        <ellipse cx="60" cy="90" rx="6" ry="4" fill="#111113" />
      )}
      {expression === "thinking" && (
        <path d="M50 90 Q60 86 70 90" stroke="#111113" strokeWidth="4" fill="none" strokeLinecap="round" />
      )}
    </motion.svg>
  );
}
