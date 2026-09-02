import type { ReactNode } from "react";
import type { Resource } from "../data";
import type { Tone } from "../state";

// ---------------------------------------------------------------------------
// Shared v4 primitives. Every status carries an ICON + TEXT LABEL as well as
// color, so color is never the only signal (accessibility requirement §92).
// ---------------------------------------------------------------------------

/** Distinct glyph per tone — a redundant, non-color cue. */
export function ToneIcon({ tone, size = 14 }: { tone: Tone; size?: number }) {
  const common = {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 2.2,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    "aria-hidden": true,
  };
  switch (tone) {
    case "danger":
      return (
        <svg {...common}>
          <path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" />
          <path d="M12 9v4M12 17h.01" />
        </svg>
      );
    case "attention":
      return (
        <svg {...common}>
          <circle cx="12" cy="12" r="9" />
          <path d="M12 8v4M12 16h.01" />
        </svg>
      );
    case "success":
      return (
        <svg {...common}>
          <circle cx="12" cy="12" r="9" />
          <path d="m8.5 12.5 2.5 2.5 4.5-5" />
        </svg>
      );
    case "active":
      return (
        <svg {...common}>
          <circle cx="12" cy="12" r="3.5" />
          <path d="M12 2v3M12 19v3M2 12h3M19 12h3" />
        </svg>
      );
    case "info":
      return (
        <svg {...common}>
          <circle cx="12" cy="12" r="9" />
          <path d="M12 11v5M12 8h.01" />
        </svg>
      );
    case "muted":
      return (
        <svg {...common}>
          <circle cx="12" cy="12" r="9" />
          <path d="M8 12h8" />
        </svg>
      );
    default:
      return (
        <svg {...common}>
          <circle cx="12" cy="12" r="9" />
        </svg>
      );
  }
}

export function ToneBadge({
  tone,
  children,
}: {
  tone: Tone;
  children: ReactNode;
}) {
  return (
    <span className={`v4-badge tone-${tone}`}>
      <ToneIcon tone={tone} />
      <span>{children}</span>
    </span>
  );
}

const SEV_TONE: Record<string, Tone> = {
  critical: "danger",
  high: "danger",
  medium: "attention",
  low: "muted",
  none: "muted",
};

export function RiskTag({ level }: { level: string }) {
  const tone = SEV_TONE[level.toLowerCase()] ?? "muted";
  return (
    <span className={`v4-tag tone-${tone}`}>
      <ToneIcon tone={tone} size={12} />
      {level.toUpperCase()}
    </span>
  );
}

/** The honest marker for a data source the backend does not instrument. Never
 * a number, never a fake chart — a plain, explicit statement (§126). */
export function NotInstrumented({ what, reason }: { what: string; reason?: string }) {
  return (
    <div className="v4-uninstrumented" role="note">
      <ToneIcon tone="muted" />
      <div>
        <b>{what}: not instrumented</b>
        <div className="muted">
          {reason ??
            "The daemon does not record this yet. Trace shows nothing here rather than an estimate."}
        </div>
      </div>
    </div>
  );
}

/** Honest failure panel for an unavailable resource. */
export function Unavailable({
  title,
  reason,
  onRetry,
}: {
  title: string;
  reason: string;
  onRetry?: () => void;
}) {
  return (
    <div className="v4-unavailable" role="alert">
      <ToneIcon tone="danger" size={18} />
      <div className="v4-unavailable-body">
        <b>{title}</b>
        <div className="muted">{reason}</div>
        {onRetry && (
          <button className="btn" style={{ marginTop: 10 }} onClick={onRetry}>
            Retry
          </button>
        )}
      </div>
    </div>
  );
}

export function Stalled({ onRetry }: { onRetry?: () => void }) {
  return (
    <div className="v4-stalled" role="status">
      <ToneIcon tone="attention" />
      <span>
        Live updates have stalled — the stream has not refreshed recently. The data below may be
        out of date.
      </span>
      {onRetry && (
        <button className="btn-ghost" onClick={onRetry}>
          Refresh now
        </button>
      )}
    </div>
  );
}

/** Render a `Resource<T>` honestly. This is the ONE place that decides how each
 * resource state looks, so no page can accidentally render a spinner as data or
 * fabricate a value for an unavailable source. */
export function ResourceGate<T>({
  resource,
  what,
  onRetry,
  empty,
  children,
  skeleton,
}: {
  resource: Resource<T>;
  what: string;
  onRetry?: () => void;
  empty?: ReactNode;
  skeleton?: ReactNode;
  children: (data: T) => ReactNode;
}) {
  switch (resource.state) {
    case "loading":
      return <>{skeleton ?? <div className="skel skel-card" aria-busy="true" aria-label={`Loading ${what}`} />}</>;
    case "ok":
      return <>{children(resource.data)}</>;
    case "empty":
      return <>{empty ?? <div className="v4-empty muted">No {what} recorded.</div>}</>;
    case "not_instrumented":
      return <NotInstrumented what={what} reason={resource.reason} />;
    case "unavailable":
      return <Unavailable title={`${what} unavailable`} reason={resource.reason} onRetry={onRetry} />;
    default:
      return null;
  }
}

/** Politely announce text to assistive tech via a visually-hidden live region. */
export function LiveRegion({ text, assertive }: { text: string; assertive: boolean }) {
  return (
    <div
      className="sr-only"
      role="status"
      aria-live={assertive ? "assertive" : "polite"}
      aria-atomic="true"
    >
      {text}
    </div>
  );
}
