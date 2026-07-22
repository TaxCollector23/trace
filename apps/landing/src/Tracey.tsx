import { useEffect, useState } from "react";
import { motion, useReducedMotion } from "framer-motion";

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

const BODY = "#4A8EF5";
const BODY_SHADOW = "#3D7AE0";
const INK = "#1a1a2e";
const WHITE = "#ffffff";
const BG_HOLE = "#09090b";

const SKIP_BLINK: TraceyExpression[] = ["success", "tired", "cool", "peek"];
const ARMS_UP: TraceyExpression[] = ["excited", "success"];

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

  useEffect(() => {
    if (reduceMotion || SKIP_BLINK.includes(expression)) return;
    let cancelled = false;
    const scheduleNext = () => {
      const delay = 6000 + Math.random() * 5000;
      window.setTimeout(() => {
        if (cancelled) return;
        if (Math.random() < 0.65) {
          setBlink(true);
          window.setTimeout(() => !cancelled && setBlink(false), 130);
        } else {
          setTilt(Math.random() < 0.5 ? -5 : 5);
          window.setTimeout(() => !cancelled && setTilt(0), 450);
        }
        scheduleNext();
      }, delay);
    };
    scheduleNext();
    return () => { cancelled = true; };
  }, [reduceMotion, expression]);

  const armsUp = ARMS_UP.includes(expression);
  const eyesClosed = blink || expression === "tired";

  const pupilOffsetX = expression === "thinking" || expression === "peek" ? 3 : 0;
  const pupilOffsetY = expression === "thinking" ? -3 : 2;

  return (
    <motion.svg
      viewBox="0 0 120 140"
      width={size}
      height={(size * 140) / 120}
      className={className}
      role="img"
      aria-label={`Tracey, ${expression}`}
      animate={reduceMotion ? undefined : { rotate: tilt }}
      transition={{ duration: 0.4, ease: "easeInOut" }}
      style={{ transformOrigin: "60px 120px", overflow: "visible" }}
    >
      {/* sparkles for success/excited */}
      {(expression === "success" || expression === "excited") && (
        <g>
          <polygon points="10,20 12,14 14,20 12,26" fill="#fbbf24" />
          <polygon points="105,16 107,10 109,16 107,22" fill="#fbbf24" />
          <polygon points="16,44 18,38 20,44 18,50" fill="#4ade80" />
          <polygon points="100,40 102,34 104,40 102,46" fill="#60a5fa" />
          <polygon points="8,60 10,56 12,60 10,64" fill="#f472b6" />
          <polygon points="108,58 110,54 112,58 110,62" fill="#a78bfa" />
        </g>
      )}

      {/* antenna ring */}
      <circle cx="60" cy="12" r="10" fill={BODY} />
      <circle cx="60" cy="12" r="5" fill={BG_HOLE} />
      {/* antenna stem */}
      <rect x="57" y="18" width="6" height="16" rx="3" fill={BODY} />

      {/* feet */}
      <ellipse cx="44" cy="122" rx="10" ry="6" fill={BODY_SHADOW} />
      <ellipse cx="76" cy="122" rx="10" ry="6" fill={BODY_SHADOW} />

      {/* arms */}
      <motion.ellipse
        cx="16"
        cy="84"
        rx="9"
        ry="13"
        fill={BODY}
        animate={{
          rotate: armsUp ? -60 : -8,
          x: armsUp ? 4 : 0,
          y: armsUp ? -20 : 0,
        }}
        style={{ transformOrigin: "16px 84px" }}
        transition={{ duration: 0.3 }}
      />
      <motion.ellipse
        cx="104"
        cy="84"
        rx="9"
        ry="13"
        fill={BODY}
        animate={{
          rotate: armsUp ? 60 : 8,
          x: armsUp ? -4 : 0,
          y: armsUp ? -20 : 0,
        }}
        style={{ transformOrigin: "104px 84px" }}
        transition={{ duration: 0.3 }}
      />

      {/* body — rounded bell shape */}
      <path
        d="M60 32 C88 32 100 54 100 76 C100 100 88 122 60 122 C32 122 20 100 20 76 C20 54 32 32 60 32 Z"
        fill={BODY}
      />
      {/* body shading — subtle bottom crescent */}
      <path
        d="M38 108 C46 120 74 120 82 108 C78 122 42 122 38 108 Z"
        fill={BODY_SHADOW}
        opacity="0.5"
      />

      {/* eyebrows */}
      {(expression === "focused" || expression === "concerned" || expression === "warning") && (
        <>
          <path
            d={expression === "focused" ? "M34 54 L52 58" : "M34 58 L52 54"}
            stroke={INK}
            strokeWidth="3"
            strokeLinecap="round"
          />
          <path
            d={expression === "focused" ? "M86 54 L68 58" : "M86 58 L68 54"}
            stroke={INK}
            strokeWidth="3"
            strokeLinecap="round"
          />
        </>
      )}
      {expression === "thinking" && (
        <path d="M62 54 Q72 48 82 54" stroke={INK} strokeWidth="3" fill="none" strokeLinecap="round" />
      )}

      {/* eyes */}
      {expression !== "cool" && (
        <>
          {eyesClosed ? (
            <>
              <path
                d={expression === "tired" ? "M34 68 Q44 74 54 68" : "M34 66 Q44 58 54 66"}
                stroke={INK}
                strokeWidth="3.5"
                fill="none"
                strokeLinecap="round"
              />
              <path
                d={expression === "tired" ? "M66 68 Q76 74 86 68" : "M66 66 Q76 58 86 66"}
                stroke={INK}
                strokeWidth="3.5"
                fill="none"
                strokeLinecap="round"
              />
            </>
          ) : (
            <>
              {/* eye whites */}
              <ellipse cx="44" cy="68" rx="12" ry="14" fill={WHITE} />
              <ellipse cx="76" cy="68" rx="12" ry="14" fill={WHITE} />
              {/* pupils */}
              <circle cx={44 + pupilOffsetX} cy={68 + pupilOffsetY} r="5.5" fill={INK} />
              <circle cx={76 + pupilOffsetX} cy={68 + pupilOffsetY} r="5.5" fill={INK} />
              {/* highlight dots */}
              <circle cx={47 + pupilOffsetX} cy={65 + pupilOffsetY} r="2" fill={WHITE} />
              <circle cx={79 + pupilOffsetX} cy={65 + pupilOffsetY} r="2" fill={WHITE} />
            </>
          )}
        </>
      )}

      {/* cool: sunglasses */}
      {expression === "cool" && (
        <g>
          <rect x="30" y="60" width="26" height="16" rx="7" fill="#18181b" />
          <rect x="64" y="60" width="26" height="16" rx="7" fill="#18181b" />
          <path d="M56 66 H64" stroke="#18181b" strokeWidth="3.5" />
          <path d="M30 66 L22 62" stroke="#18181b" strokeWidth="3" strokeLinecap="round" />
          <path d="M90 66 L98 62" stroke="#18181b" strokeWidth="3" strokeLinecap="round" />
        </g>
      )}

      {/* analyzing: magnifying glass */}
      {expression === "analyzing" && (
        <g>
          <circle cx="88" cy="56" r="10" fill="none" stroke={WHITE} strokeWidth="3.5" />
          <path d="M95 63 L102 70" stroke={WHITE} strokeWidth="4" strokeLinecap="round" />
        </g>
      )}

      {/* warning: exclamation */}
      {expression === "warning" && (
        <g>
          <rect x="96" y="30" width="6" height="16" rx="3" fill="#f59e0b" />
          <circle cx="99" cy="52" r="3.5" fill="#f59e0b" />
        </g>
      )}

      {/* detecting: radar rings near antenna */}
      {expression === "detecting" && (
        <g>
          <circle cx="92" cy="24" r="8" fill="#22c55e" opacity="0.25" />
          <circle cx="92" cy="24" r="4" fill="#22c55e" opacity="0.5" />
          <circle cx="92" cy="24" r="1.5" fill="#22c55e" />
          <circle cx="92" cy="24" r="14" fill="none" stroke="#22c55e" strokeWidth="1.5" opacity="0.2" />
        </g>
      )}

      {/* tired: zzz */}
      {expression === "tired" && (
        <g fontFamily="inherit" fontWeight="700" fill="#a1a1aa">
          <text x="90" y="36" fontSize="14">z</text>
          <text x="96" y="26" fontSize="11">z</text>
        </g>
      )}

      {/* mouth */}
      {expression === "happy" && (
        <path d="M48 90 Q60 100 72 90" stroke={INK} strokeWidth="3.5" fill="none" strokeLinecap="round" />
      )}
      {(expression === "success" || expression === "excited") && (
        <>
          <path d="M44 88 Q60 106 76 88" stroke={INK} strokeWidth="3.5" fill="none" strokeLinecap="round" />
          <path d="M48 90 Q60 98 72 90" fill={INK} opacity="0.15" />
        </>
      )}
      {expression === "thinking" && (
        <circle cx="68" cy="90" r="4" fill={INK} />
      )}
      {expression === "analyzing" && <ellipse cx="56" cy="90" rx="5" ry="4" fill={INK} />}
      {expression === "focused" && <path d="M48 92 H72" stroke={INK} strokeWidth="3.5" strokeLinecap="round" />}
      {expression === "cool" && <path d="M50 88 Q60 94 70 88" stroke={INK} strokeWidth="3" fill="none" strokeLinecap="round" />}
      {expression === "warning" && <ellipse cx="60" cy="90" rx="6" ry="5" fill={INK} />}
      {expression === "concerned" && (
        <path d="M48 94 Q60 86 72 94" stroke={INK} strokeWidth="3.5" fill="none" strokeLinecap="round" />
      )}
      {expression === "detecting" && <ellipse cx="60" cy="90" rx="4.5" ry="3.5" fill={INK} />}
      {expression === "tired" && <ellipse cx="60" cy="90" rx="6" ry="4" fill={INK} />}
      {expression === "peek" && (
        <path d="M50 88 Q58 94 66 89" stroke={INK} strokeWidth="3.5" fill="none" strokeLinecap="round" />
      )}
    </motion.svg>
  );
}
