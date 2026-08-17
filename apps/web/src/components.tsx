import { useEffect, useState, type CSSProperties } from "react";
import { useNavigate } from "react-router-dom";
import type { RunStatus, RunSummary } from "./api";

/** Inline style for the `.enter` animation's stagger delay, capped so long
 * lists don't take forever to finish revealing. */
export function stagger(index: number, stepMs = 30, maxMs = 240): CSSProperties {
  return { "--d": `${Math.min(index * stepMs, maxMs)}ms` } as CSSProperties;
}

/** Minimal data-fetching hook with loading + error states. */
export function useAsync<T>(fn: () => Promise<T>, deps: unknown[] = []): {
  data: T | null;
  error: string | null;
  loading: boolean;
  reload: () => void;
} {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [nonce, setNonce] = useState(0);

  useEffect(() => {
    let alive = true;
    setLoading(true);
    fn()
      .then((d) => alive && (setData(d), setError(null)))
      .catch((e) => alive && setError(String(e)))
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, nonce]);

  return { data, error, loading, reload: () => setNonce((n) => n + 1) };
}

export function StatusBadge({ status }: { status: RunStatus }) {
  return <span className={`badge ${status}`}>{status.replace("_", " ")}</span>;
}

/** Subtle tag marking which findings come from the updatable rule pack
 * (`policy-pack`) versus a built-in engine rule (`policy-engine`). Only the
 * pack rules get a badge, so the common built-in case stays unadorned. */
export function SourceBadge({ source }: { source: string }) {
  if (source !== "policy-pack") return null;
  return (
    <span className="pill source-pack" title="From the updatable rule pack">
      pack
    </span>
  );
}

export function fmtTime(iso: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  return d.toLocaleString();
}

export function fmtCost(cost: number | null | undefined): string {
  if (cost === null || cost === undefined) return "unavailable";
  if (cost === 0) return "$0.00";
  return `$${cost.toFixed(cost < 0.01 ? 4 : 2)}`;
}

export function fmtNum(n: number | null): string {
  return n === null || n === undefined ? "—" : n.toLocaleString();
}

type LoadingVariant = "kpis" | "cards" | "timeline" | "table" | "text";

/** Shape-matched skeleton for the content it's replacing, so the layout
 * doesn't jump once real data arrives. Falls back to an error/empty state. */
export function Loading({
  error,
  variant = "text",
  rows = 3,
  onRetry,
}: {
  error?: string | null;
  variant?: LoadingVariant;
  rows?: number;
  onRetry?: () => void;
}) {
  if (error)
    return (
      <div className="empty" role="alert">
        Could not load data: {error}
        {onRetry && (
          <div style={{ marginTop: 12 }}>
            <button className="btn" onClick={onRetry}>
              Retry
            </button>
          </div>
        )}
      </div>
    );

  if (variant === "kpis") {
    return (
      <div className="skel-kpis" aria-busy="true" aria-label="Loading">
        {Array.from({ length: 4 }).map((_, i) => (
          <div key={i} className="skel skel-kpi" />
        ))}
      </div>
    );
  }
  if (variant === "cards" || variant === "table") {
    return (
      <div aria-busy="true" aria-label="Loading">
        {Array.from({ length: rows }).map((_, i) => (
          <div key={i} className="skel skel-card" />
        ))}
      </div>
    );
  }
  if (variant === "timeline") {
    return (
      <div className="timeline" aria-busy="true" aria-label="Loading">
        {Array.from({ length: rows }).map((_, i) => (
          <div key={i} className="skel-tl-item">
            <div className="skel skel-line" style={{ width: 140 }} />
            <div className="skel skel-line" style={{ width: "70%" }} />
          </div>
        ))}
      </div>
    );
  }
  return (
    <div aria-busy="true" aria-label="Loading">
      <div className="skel skel-line" style={{ width: "40%" }} />
      <div className="skel skel-line" style={{ width: "70%" }} />
    </div>
  );
}

/** A select to choose which run a center page is showing. */
export function RunPicker({
  runs,
  current,
  base,
}: {
  runs: RunSummary[];
  current: string | undefined;
  base: string;
}) {
  const navigate = useNavigate();
  return (
    <div className="run-picker">
      <label className="muted" style={{ marginRight: 8 }}>
        Run:
      </label>
      <select
        aria-label="Run"
        value={current ?? ""}
        onChange={(e) => navigate(`${base}/${e.target.value}`)}
      >
        <option value="" disabled>
          Select a run…
        </option>
        {runs.map((r) => (
          <option key={r.id} value={r.id}>
            {label(r)}
          </option>
        ))}
      </select>
    </div>
  );
}

/** Render a unified diff with line-level coloring. */
/** Above this many lines, a diff is truncated with an expander so a huge PR
 * doesn't emit tens of thousands of DOM nodes and freeze the tab. */
const DIFF_LINE_CAP = 800;

export function DiffView({ diff }: { diff: string }) {
  const [expanded, setExpanded] = useState(false);
  if (!diff.trim()) {
    return (
      <div className="empty">
        No stored diff for this run (the run may predate diff capture, or the
        project is not a Git repository).
      </div>
    );
  }
  const allLines = diff.split("\n");
  const truncated = !expanded && allLines.length > DIFF_LINE_CAP;
  const lines = truncated ? allLines.slice(0, DIFF_LINE_CAP) : allLines;
  return (
    <>
      <pre className="diff">
        {lines.map((line, i) => {
          let cls = "";
          if (line.startsWith("+++") || line.startsWith("---")) cls = "meta";
          else if (line.startsWith("@@")) cls = "hunk";
          else if (line.startsWith("+")) cls = "add";
          else if (line.startsWith("-")) cls = "del";
          else if (line.startsWith("diff ") || line.startsWith("index ")) cls = "meta";
          return (
            <div key={i} className={cls}>
              {line || " "}
            </div>
          );
        })}
      </pre>
      {truncated && (
        <button className="btn" onClick={() => setExpanded(true)}>
          Show full diff ({allLines.length.toLocaleString()} lines)
        </button>
      )}
    </>
  );
}

function label(r: RunSummary): string {
  const cmd = r.command.length > 50 ? r.command.slice(0, 50) + "…" : r.command;
  return `${cmd}  ·  ${r.status}  ·  ${fmtTime(r.started_at)}`;
}
