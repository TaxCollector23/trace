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
}: {
  error?: string | null;
  variant?: LoadingVariant;
  rows?: number;
}) {
  if (error) return <div className="empty">Could not load data: {error}</div>;

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
export function DiffView({ diff }: { diff: string }) {
  if (!diff.trim()) {
    return (
      <div className="empty">
        No stored diff for this run (the run may predate diff capture, or the
        project is not a Git repository).
      </div>
    );
  }
  const lines = diff.split("\n");
  return (
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
  );
}

function label(r: RunSummary): string {
  const cmd = r.command.length > 50 ? r.command.slice(0, 50) + "…" : r.command;
  return `${cmd}  ·  ${r.status}  ·  ${fmtTime(r.started_at)}`;
}
