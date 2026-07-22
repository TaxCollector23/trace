import { useEffect, useState } from "react";

export type TraceyExpression = "happy" | "sleeping" | "celebrate" | "watching" | "thinking";

/**
 * Tracey — Trace's mascot. Same original design as the landing site's
 * version, reimplemented with plain SVG + CSS (no framer-motion) to match
 * this app's existing no-runtime-UI-dependency architecture. Only one
 * instance should ever be mounted at a time.
 */
export default function Tracey({
  expression = "happy",
  size = 28,
  className,
}: {
  expression?: TraceyExpression;
  size?: number;
  className?: string;
}) {
  const [blink, setBlink] = useState(false);

  useEffect(() => {
    if (expression === "sleeping") return;
    let cancelled = false;
    const schedule = () => {
      const delay = 8000 + Math.random() * 7000;
      window.setTimeout(() => {
        if (cancelled) return;
        setBlink(true);
        window.setTimeout(() => !cancelled && setBlink(false), 140);
        schedule();
      }, delay);
    };
    schedule();
    return () => {
      cancelled = true;
    };
  }, [expression]);

  const eyesClosed = expression === "sleeping" || blink;

  return (
    <svg
      viewBox="0 0 120 130"
      width={size}
      height={(size * 130) / 120}
      className={className}
      role="img"
      aria-label={`Tracey, ${expression}`}
    >
      <path d="M60 24 Q79 8 90 21" stroke="#4fa3ff" strokeWidth="6" fill="none" strokeLinecap="round" />
      <circle cx="91" cy="20" r="7" fill="#4fa3ff" />
      <ellipse cx="42" cy="112" rx="13" ry="9" fill="#4fa3ff" />
      <ellipse cx="78" cy="112" rx="13" ry="9" fill="#4fa3ff" />
      <ellipse cx="60" cy="70" rx="44" ry="50" fill="#4fa3ff" />

      {expression === "celebrate" && (
        <g>
          <rect x="14" y="18" width="5" height="5" fill="#3fb950" transform="rotate(20 16 20)" />
          <rect x="100" y="34" width="5" height="5" fill="#d29922" transform="rotate(-15 102 36)" />
          <rect x="24" y="42" width="4" height="4" fill="#f85149" transform="rotate(45 26 44)" />
        </g>
      )}

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

      {expression === "happy" && (
        <path d="M48 88 Q60 98 72 88" stroke="#111113" strokeWidth="4" fill="none" strokeLinecap="round" />
      )}
      {expression === "sleeping" && <path d="M54 90 H66" stroke="#111113" strokeWidth="4" strokeLinecap="round" />}
      {expression === "celebrate" && (
        <path d="M46 86 Q60 102 74 86" stroke="#111113" strokeWidth="4" fill="none" strokeLinecap="round" />
      )}
      {expression === "watching" && <ellipse cx="60" cy="90" rx="6" ry="4" fill="#111113" />}
      {expression === "thinking" && (
        <path d="M50 90 Q60 86 70 90" stroke="#111113" strokeWidth="4" fill="none" strokeLinecap="round" />
      )}
    </svg>
  );
}
