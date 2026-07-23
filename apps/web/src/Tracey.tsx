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

const INK = "#1a1a2e";
const WHITE = "#ffffff";

const SKIP_BLINK: TraceyExpression[] = ["success", "tired", "cool", "peek"];
const ARMS_UP: TraceyExpression[] = ["excited", "success"];

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
      const delay = 6000 + Math.random() * 5000;
      window.setTimeout(() => {
        if (cancelled) return;
        setBlink(true);
        window.setTimeout(() => !cancelled && setBlink(false), 130);
        schedule();
      }, delay);
    };
    schedule();
    return () => { cancelled = true; };
  }, [expression]);

  const armsUp = ARMS_UP.includes(expression);
  const eyesClosed = blink || expression === "tired";
  const pupilX = expression === "thinking" || expression === "peek" ? 3 : 0;
  const pupilY = expression === "thinking" ? -4 : 2;

  return (
    <svg
      viewBox="0 0 120 150"
      width={size}
      height={(size * 150) / 120}
      className={className}
      role="img"
      aria-label={`Tracey, ${expression}`}
      style={{ overflow: "visible" }}
    >
      <defs>
        <radialGradient id="t3dBody" cx="40%" cy="30%" r="65%" fx="35%" fy="25%">
          <stop offset="0%" stopColor="#7BB8FF" />
          <stop offset="35%" stopColor="#5A9DF5" />
          <stop offset="70%" stopColor="#4A8EF5" />
          <stop offset="100%" stopColor="#2D6AD0" />
        </radialGradient>
        <radialGradient id="t3dGloss" cx="42%" cy="25%" r="40%">
          <stop offset="0%" stopColor="#ffffff" stopOpacity="0.45" />
          <stop offset="100%" stopColor="#ffffff" stopOpacity="0" />
        </radialGradient>
        <radialGradient id="t3dArm" cx="50%" cy="30%" r="70%">
          <stop offset="0%" stopColor="#6AACFF" />
          <stop offset="100%" stopColor="#3A7AE0" />
        </radialGradient>
        <radialGradient id="t3dAntenna" cx="40%" cy="30%" r="60%">
          <stop offset="0%" stopColor="#6AACFF" />
          <stop offset="100%" stopColor="#3870D4" />
        </radialGradient>
        <radialGradient id="t3dShadow" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor="#000000" stopOpacity="0.25" />
          <stop offset="100%" stopColor="#000000" stopOpacity="0" />
        </radialGradient>
        <radialGradient id="t3dEye" cx="45%" cy="35%" r="55%">
          <stop offset="0%" stopColor="#ffffff" />
          <stop offset="100%" stopColor="#e8ecf0" />
        </radialGradient>
        <radialGradient id="t3dFoot" cx="50%" cy="30%" r="70%">
          <stop offset="0%" stopColor="#5A9DF5" />
          <stop offset="100%" stopColor="#2D6AD0" />
        </radialGradient>
      </defs>

      <ellipse cx="60" cy="136" rx="32" ry="6" fill="url(#t3dShadow)" />

      {(expression === "success" || expression === "excited") && (
        <g>
          <polygon points="10,22 12,16 14,22 12,28" fill="#fbbf24" />
          <polygon points="106,18 108,12 110,18 108,24" fill="#fbbf24" />
          <polygon points="16,46 18,40 20,46 18,52" fill="#4ade80" />
          <polygon points="100,42 102,36 104,42 102,48" fill="#60a5fa" />
        </g>
      )}

      <ellipse cx="60" cy="12" rx="11" ry="10" fill="url(#t3dAntenna)" />
      <ellipse cx="60" cy="11" rx="5.5" ry="5" fill="#09090b" />
      <ellipse cx="57" cy="8" rx="3" ry="1.5" fill="#ffffff" opacity="0.3" />
      <rect x="57" y="18" width="6" height="14" rx="3" fill="url(#t3dAntenna)" />
      <rect x="57.5" y="18" width="2.5" height="12" rx="1.5" fill="#ffffff" opacity="0.12" />

      <ellipse cx="44" cy="130" rx="10" ry="6" fill="url(#t3dFoot)" />
      <ellipse cx="76" cy="130" rx="10" ry="6" fill="url(#t3dFoot)" />
      <ellipse cx="42" cy="128" rx="4" ry="2" fill="#ffffff" opacity="0.15" />
      <ellipse cx="74" cy="128" rx="4" ry="2" fill="#ffffff" opacity="0.15" />

      <ellipse
        cx="18"
        cy="90"
        rx="8"
        ry="12"
        fill="url(#t3dArm)"
        style={{
          transformOrigin: "18px 90px",
          transform: armsUp ? "translate(4px, -18px) rotate(-55deg)" : "rotate(-6deg)",
          transition: "transform 0.3s ease",
        }}
      />
      <ellipse
        cx="102"
        cy="90"
        rx="8"
        ry="12"
        fill="url(#t3dArm)"
        style={{
          transformOrigin: "102px 90px",
          transform: armsUp ? "translate(-4px, -18px) rotate(55deg)" : "rotate(6deg)",
          transition: "transform 0.3s ease",
        }}
      />

      <path
        d="M60 30 C76 30 90 42 96 64 C102 86 98 110 86 124 C78 132 42 132 34 124 C22 110 18 86 24 64 C30 42 44 30 60 30 Z"
        fill="url(#t3dBody)"
      />
      <path
        d="M60 30 C76 30 90 42 96 64 C102 86 98 110 86 124 C78 132 42 132 34 124 C22 110 18 86 24 64 C30 42 44 30 60 30 Z"
        fill="url(#t3dGloss)"
      />
      <ellipse cx="46" cy="50" rx="12" ry="8" fill="#ffffff" opacity="0.18" />
      <path d="M90 50 C96 64 98 86 92 108" stroke="#ffffff" strokeWidth="2" fill="none" opacity="0.08" strokeLinecap="round" />
      <path d="M40 118 Q60 130 80 118" fill="#1a3a7a" opacity="0.15" />

      {expression === "warning" && (
        <>
          <path d="M34 60 L50 56" stroke={INK} strokeWidth="3" strokeLinecap="round" />
          <path d="M86 60 L70 56" stroke={INK} strokeWidth="3" strokeLinecap="round" />
        </>
      )}
      {expression === "concerned" && (
        <>
          <path d="M34 60 L50 56" stroke={INK} strokeWidth="3" strokeLinecap="round" />
          <path d="M86 60 L70 56" stroke={INK} strokeWidth="3" strokeLinecap="round" />
        </>
      )}
      {expression === "focused" && (
        <>
          <path d="M34 58 L50 62" stroke={INK} strokeWidth="3" strokeLinecap="round" />
          <path d="M86 58 L70 62" stroke={INK} strokeWidth="3" strokeLinecap="round" />
        </>
      )}

      {expression !== "cool" && (
        <>
          {eyesClosed ? (
            <>
              <path
                d={expression === "tired" ? "M34 72 Q44 78 54 72" : "M34 70 Q44 62 54 70"}
                stroke={INK} strokeWidth="3.5" fill="none" strokeLinecap="round"
              />
              <path
                d={expression === "tired" ? "M66 72 Q76 78 86 72" : "M66 70 Q76 62 86 70"}
                stroke={INK} strokeWidth="3.5" fill="none" strokeLinecap="round"
              />
            </>
          ) : (
            <>
              <ellipse cx="44" cy="72" rx="14" ry="16" fill="url(#t3dEye)" />
              <ellipse cx="76" cy="72" rx="14" ry="16" fill="url(#t3dEye)" />
              <ellipse cx="44" cy="60" rx="12" ry="5" fill="#c0cee0" opacity="0.25" />
              <ellipse cx="76" cy="60" rx="12" ry="5" fill="#c0cee0" opacity="0.25" />
              <circle cx={44 + pupilX} cy={72 + pupilY} r="7" fill={INK} />
              <circle cx={76 + pupilX} cy={72 + pupilY} r="7" fill={INK} />
              <circle cx={47 + pupilX} cy={69 + pupilY} r="3" fill={WHITE} />
              <circle cx={79 + pupilX} cy={69 + pupilY} r="3" fill={WHITE} />
              <circle cx={42 + pupilX} cy={75 + pupilY} r="1.5" fill={WHITE} opacity="0.6" />
              <circle cx={74 + pupilX} cy={75 + pupilY} r="1.5" fill={WHITE} opacity="0.6" />
            </>
          )}
        </>
      )}

      {expression === "cool" && (
        <g>
          <rect x="28" y="62" width="28" height="18" rx="8" fill="#18181b" />
          <rect x="64" y="62" width="28" height="18" rx="8" fill="#18181b" />
          <path d="M56 70 H64" stroke="#18181b" strokeWidth="3.5" />
          <path d="M32 66 Q42 64 52 66" stroke="#ffffff" strokeWidth="1.5" fill="none" opacity="0.2" strokeLinecap="round" />
          <path d="M68 66 Q78 64 88 66" stroke="#ffffff" strokeWidth="1.5" fill="none" opacity="0.2" strokeLinecap="round" />
        </g>
      )}

      {expression === "detecting" && (
        <g>
          <circle cx="92" cy="54" r="13" fill="#e0ecff" opacity="0.15" />
          <circle cx="92" cy="54" r="13" fill="none" stroke="#d0e4ff" strokeWidth="3.5" />
          <circle cx="92" cy="54" r="13" fill="none" stroke="#ffffff" strokeWidth="2" opacity="0.5" />
          <path d="M102 64 L110 72" stroke="#d0e4ff" strokeWidth="4.5" strokeLinecap="round" />
          <path d="M102 64 L110 72" stroke="#ffffff" strokeWidth="2" strokeLinecap="round" opacity="0.4" />
          <ellipse cx="88" cy="50" rx="4" ry="3" fill="#ffffff" opacity="0.25" />
          <circle cx="92" cy="30" r="8" fill="none" stroke="#4af" strokeWidth="1.5" opacity="0.3" />
          <circle cx="92" cy="30" r="15" fill="none" stroke="#4af" strokeWidth="1" opacity="0.15" />
          <circle cx="92" cy="30" r="3" fill="#4af" opacity="0.5" />
        </g>
      )}

      {expression === "warning" && (
        <g transform="translate(88, 22)">
          <path d="M12 0 L24 20 H0 Z" fill="#ef4444" />
          <path d="M12 0 L18 10 H6 Z" fill="#ff6b6b" opacity="0.4" />
          <rect x="10.5" y="6" width="3" height="9" rx="1.5" fill={WHITE} />
          <circle cx="12" cy="17.5" r="1.5" fill={WHITE} />
        </g>
      )}

      {expression === "analyzing" && (
        <g>
          <circle cx="90" cy="58" r="11" fill="none" stroke="#d0e4ff" strokeWidth="3.5" />
          <path d="M98 66 L106 74" stroke="#d0e4ff" strokeWidth="4.5" strokeLinecap="round" />
          <ellipse cx="87" cy="55" rx="3.5" ry="2.5" fill="#ffffff" opacity="0.2" />
        </g>
      )}

      {expression === "tired" && (
        <g fontFamily="inherit" fontWeight="700" fill="#a1a1aa">
          <text x="90" y="42" fontSize="14">z</text>
          <text x="96" y="30" fontSize="11">z</text>
        </g>
      )}

      {expression === "happy" && (
        <path d="M46 96 Q60 108 74 96" stroke={INK} strokeWidth="3.5" fill="none" strokeLinecap="round" />
      )}
      {(expression === "success" || expression === "excited") && (
        <path d="M42 94 Q60 114 78 94" stroke={INK} strokeWidth="3.5" fill="none" strokeLinecap="round" />
      )}
      {expression === "thinking" && <circle cx="68" cy="96" r="4.5" fill={INK} />}
      {expression === "analyzing" && <ellipse cx="56" cy="96" rx="5" ry="4" fill={INK} />}
      {expression === "focused" && <path d="M46 98 H74" stroke={INK} strokeWidth="3.5" strokeLinecap="round" />}
      {expression === "cool" && <path d="M48 94 Q60 100 72 94" stroke={INK} strokeWidth="3" fill="none" strokeLinecap="round" />}
      {expression === "warning" && (
        <path d="M44 100 Q60 92 76 100" stroke={INK} strokeWidth="3.5" fill="none" strokeLinecap="round" />
      )}
      {expression === "concerned" && (
        <path d="M46 100 Q60 92 74 100" stroke={INK} strokeWidth="3.5" fill="none" strokeLinecap="round" />
      )}
      {expression === "detecting" && <ellipse cx="55" cy="96" rx="5" ry="3.5" fill={INK} />}
      {expression === "tired" && <ellipse cx="60" cy="96" rx="6" ry="4" fill={INK} />}
      {expression === "peek" && <ellipse cx="58" cy="96" rx="4" ry="3" fill={INK} />}
    </svg>
  );
}
