import { useEffect, useState } from "react";

export type TraceyExpression =
  | "happy"
  | "thinking"
  | "analyzing"
  | "focused"
  | "success"
  | "cool"
  | "warning"
  | "concerned"
  | "detecting"
  | "excited"
  | "tired"
  | "peek";

const BLUE = "#4fa3ff";
const INK = "#111113";
const WHITE = "#fafafa";

const SKIP_BLINK: TraceyExpression[] = ["success", "tired", "cool", "peek"];
const ARMS_UP: TraceyExpression[] = ["excited", "success"];

/**
 * Tracey — Trace's mascot. Same design as the landing site's version,
 * reimplemented with plain SVG + CSS (no framer-motion) to match this app's
 * existing no-runtime-UI-dependency architecture.
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
    if (SKIP_BLINK.includes(expression)) return;
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

  const armsUp = ARMS_UP.includes(expression);
  const eyesClosed = blink || expression === "tired";

  return (
    <svg
      viewBox="0 0 120 148"
      width={size}
      height={(size * 148) / 120}
      className={className}
      role="img"
      aria-label={`Tracey, ${expression}`}
      style={{ overflow: "visible" }}
    >
      {(expression === "success" || expression === "excited") && (
        <g>
          <rect x="6" y="20" width="6" height="6" fill="#22c55e" transform="rotate(20 9 23)" />
          <rect x="106" y="34" width="6" height="6" fill="#f59e0b" transform="rotate(-15 109 37)" />
          <rect x="18" y="46" width="5" height="5" fill="#ef4444" transform="rotate(45 20 48)" />
          <rect x="98" y="12" width="5" height="5" fill="#4fa3ff" transform="rotate(10 100 14)" />
        </g>
      )}

      <path
        d="M64 22 C68 10 76 18 78 8 C80 0 88 4 86 12"
        stroke={BLUE}
        strokeWidth="5"
        fill="none"
        strokeLinecap="round"
      />
      <circle cx="87" cy="10" r="6" fill={BLUE} />

      <ellipse cx="45" cy="128" rx="12" ry="8" fill={BLUE} />
      <ellipse cx="75" cy="128" rx="12" ry="8" fill={BLUE} />

      <ellipse
        cx="14"
        cy="92"
        rx="9"
        ry="14"
        fill={BLUE}
        style={{
          transformOrigin: "14px 92px",
          transform: armsUp ? "translate(-2px, -18px) rotate(-75deg)" : "rotate(-10deg)",
          transition: "transform 0.3s ease",
        }}
      />
      <ellipse
        cx="106"
        cy="92"
        rx="9"
        ry="14"
        fill={BLUE}
        style={{
          transformOrigin: "106px 92px",
          transform: armsUp ? "translate(2px, -18px) rotate(75deg)" : "rotate(10deg)",
          transition: "transform 0.3s ease",
        }}
      />

      <path
        d="M60 20 C86 20 104 42 104 68 L104 96 C104 116 90 128 70 126 C64 130 56 130 50 126 C30 128 16 116 16 96 L16 68 C16 42 34 20 60 20 Z"
        fill={BLUE}
      />

      {(expression === "focused" || expression === "concerned" || expression === "warning") && (
        <>
          <path
            d="M36 48 L54 52"
            stroke={INK}
            strokeWidth="3.5"
            strokeLinecap="round"
            transform={expression === "focused" ? "rotate(6 45 50)" : "rotate(-6 45 50)"}
          />
          <path
            d="M84 48 L66 52"
            stroke={INK}
            strokeWidth="3.5"
            strokeLinecap="round"
            transform={expression === "focused" ? "rotate(-6 75 50)" : "rotate(6 75 50)"}
          />
        </>
      )}
      {expression === "thinking" && (
        <path d="M62 46 Q72 40 82 46" stroke={INK} strokeWidth="3.5" fill="none" strokeLinecap="round" />
      )}

      {expression !== "cool" && (
        <>
          {eyesClosed ? (
            <>
              <path
                d={expression === "tired" ? "M36 64 Q46 68 56 64" : "M36 62 Q46 54 56 62"}
                stroke={INK}
                strokeWidth="4"
                fill="none"
                strokeLinecap="round"
              />
              <path
                d={expression === "tired" ? "M64 64 Q74 68 84 64" : "M64 62 Q74 54 84 62"}
                stroke={INK}
                strokeWidth="4"
                fill="none"
                strokeLinecap="round"
              />
            </>
          ) : (
            <>
              <ellipse cx="46" cy="64" rx="13" ry="15" fill={WHITE} />
              <ellipse cx="74" cy="64" rx="13" ry="15" fill={WHITE} />
              <circle
                cx={expression === "thinking" || expression === "peek" ? 49 : 46}
                cy={expression === "thinking" ? 60 : 67}
                r="6"
                fill={INK}
              />
              <circle
                cx={expression === "thinking" || expression === "peek" ? 77 : 74}
                cy={expression === "thinking" ? 60 : 67}
                r="6"
                fill={INK}
              />
            </>
          )}
        </>
      )}

      {expression === "cool" && (
        <g>
          <rect x="33" y="56" width="26" height="16" rx="7" fill="#18181b" />
          <rect x="61" y="56" width="26" height="16" rx="7" fill="#18181b" />
          <path d="M59 60 H61" stroke="#18181b" strokeWidth="4" />
        </g>
      )}

      {expression === "analyzing" && (
        <g>
          <circle cx="82" cy="58" r="11" fill="none" stroke={WHITE} strokeWidth="4" />
          <path d="M90 66 L98 74" stroke={WHITE} strokeWidth="5" strokeLinecap="round" />
        </g>
      )}

      {expression === "warning" && (
        <g>
          <rect x="94" y="26" width="7" height="18" rx="3.5" fill="#f59e0b" />
          <circle cx="97.5" cy="50" r="4" fill="#f59e0b" />
        </g>
      )}

      {expression === "detecting" && (
        <g opacity="0.8">
          <circle cx="87" cy="10" r="14" fill="none" stroke={BLUE} strokeWidth="2" opacity="0.5" />
          <circle cx="87" cy="10" r="21" fill="none" stroke={BLUE} strokeWidth="2" opacity="0.25" />
        </g>
      )}

      {expression === "tired" && (
        <text x="92" y="30" fontSize="16" fontWeight="700" fill="#a1a1aa" fontFamily="inherit">
          z
        </text>
      )}

      {expression === "happy" && (
        <path d="M48 86 Q60 96 72 86" stroke={INK} strokeWidth="4" fill="none" strokeLinecap="round" />
      )}
      {(expression === "success" || expression === "excited") && (
        <path d="M46 84 Q60 100 74 84" stroke={INK} strokeWidth="4" fill="none" strokeLinecap="round" />
      )}
      {expression === "thinking" && (
        <path d="M50 88 Q60 84 68 88" stroke={INK} strokeWidth="4" fill="none" strokeLinecap="round" />
      )}
      {expression === "analyzing" && <ellipse cx="55" cy="88" rx="5" ry="4" fill={INK} />}
      {expression === "focused" && <path d="M48 90 H72" stroke={INK} strokeWidth="4" strokeLinecap="round" />}
      {expression === "cool" && <path d="M52 86 Q60 90 68 86" stroke={INK} strokeWidth="3.5" fill="none" strokeLinecap="round" />}
      {expression === "warning" && <ellipse cx="60" cy="88" rx="6" ry="5" fill={INK} />}
      {expression === "concerned" && (
        <path d="M48 92 Q60 84 72 92" stroke={INK} strokeWidth="4" fill="none" strokeLinecap="round" />
      )}
      {expression === "detecting" && <ellipse cx="60" cy="88" rx="5" ry="3.5" fill={INK} />}
      {expression === "tired" && <ellipse cx="60" cy="88" rx="7" ry="5" fill={INK} />}
      {expression === "peek" && (
        <path d="M50 86 Q60 92 66 87" stroke={INK} strokeWidth="4" fill="none" strokeLinecap="round" />
      )}
    </svg>
  );
}
