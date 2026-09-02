import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { api, type RunSummary } from "../api";

// ---------------------------------------------------------------------------
// Keyboard-first command palette (§85, §86). Opens on ⌘K / Ctrl-K (or "/").
// Every navigation and every recent run is reachable without the mouse.
// ---------------------------------------------------------------------------

interface Item {
  id: string;
  label: string;
  hint?: string;
  run: () => void;
  group: string;
}

const NAV: [string, string][] = [
  ["/", "Control Room"],
  ["/run", "Run Page (latest)"],
  ["/timeline", "Session Timeline"],
  ["/patch", "Patch Review"],
  ["/risk", "Command Risk"],
  ["/usage", "Token Usage"],
  ["/benchmarks", "Benchmarks"],
  ["/rollback", "Rollback Points"],
  ["/github", "GitHub"],
  ["/ratify", "Ratify"],
  ["/system", "System & Integrations"],
];

export function CommandPalette() {
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [runs, setRuns] = useState<RunSummary[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);

  const close = useCallback(() => {
    setOpen(false);
    setQuery("");
    setActive(0);
  }, []);

  // Global open shortcut.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && (e.key === "k" || e.key === "K")) {
        e.preventDefault();
        setOpen((o) => !o);
      } else if (e.key === "/" && !open && !isTyping(e.target)) {
        e.preventDefault();
        setOpen(true);
      } else if (e.key === "Escape" && open) {
        close();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, close]);

  // Fetch runs lazily when opened (best-effort; failure just hides run rows).
  useEffect(() => {
    if (!open) return;
    inputRef.current?.focus();
    api
      .runs()
      .then((r) => setRuns(r.slice(0, 8)))
      .catch(() => setRuns([]));
  }, [open]);

  const items: Item[] = useMemo(() => {
    const nav: Item[] = NAV.map(([to, label]) => ({
      id: `nav:${to}`,
      label,
      group: "Go to",
      hint: to,
      run: () => {
        navigate(to);
        close();
      },
    }));
    const runItems: Item[] = runs.map((r) => ({
      id: `run:${r.id}`,
      label: `${r.agent_name ?? "Command"} — ${r.project_name}`,
      hint: r.status.replace("_", " "),
      group: "Recent runs",
      run: () => {
        navigate(`/run/${r.id}`);
        close();
      },
    }));
    return [...nav, ...runItems];
  }, [runs, navigate, close]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return items;
    return items.filter(
      (i) => i.label.toLowerCase().includes(q) || (i.hint ?? "").toLowerCase().includes(q)
    );
  }, [items, query]);

  if (!open) return null;

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((a) => Math.min(a + 1, filtered.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((a) => Math.max(a - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      filtered[active]?.run();
    }
  };

  let lastGroup = "";
  return (
    <div className="v4-palette-overlay" onMouseDown={close}>
      <div
        className="v4-palette"
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <input
          ref={inputRef}
          className="v4-palette-input"
          placeholder="Jump to a page or run…  (↑↓ to move, Enter to select, Esc to close)"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setActive(0);
          }}
          onKeyDown={onKeyDown}
          aria-activedescendant={filtered[active]?.id}
          aria-controls="v4-palette-list"
        />
        <ul className="v4-palette-list" id="v4-palette-list" role="listbox">
          {filtered.length === 0 && <li className="v4-palette-empty muted">No matches.</li>}
          {filtered.map((it, i) => {
            const header = it.group !== lastGroup ? it.group : null;
            lastGroup = it.group;
            return (
              <li key={it.id}>
                {header && <div className="v4-palette-group">{header}</div>}
                <button
                  id={it.id}
                  role="option"
                  aria-selected={i === active}
                  className={`v4-palette-item ${i === active ? "active" : ""}`}
                  onMouseEnter={() => setActive(i)}
                  onClick={it.run}
                >
                  <span>{it.label}</span>
                  {it.hint && <span className="v4-palette-hint muted">{it.hint}</span>}
                </button>
              </li>
            );
          })}
        </ul>
      </div>
    </div>
  );
}

function isTyping(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el) return false;
  const tag = el.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || el.isContentEditable;
}
