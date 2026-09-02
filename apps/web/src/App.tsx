import { useEffect, useState } from "react";
import { NavLink, Outlet } from "react-router-dom";
import { Mark } from "./Mark";
import { CommandPalette } from "./v4/CommandPalette";

type Theme = "light" | "dark" | "system";
const THEME_KEY = "trace-theme";

function readTheme(): Theme {
  try {
    const v = localStorage.getItem(THEME_KEY);
    if (v === "light" || v === "dark" || v === "system") return v;
  } catch {
    /* localStorage unavailable (private mode / SSR) — fall back to system. */
  }
  return "system";
}

function applyTheme(theme: Theme) {
  const root = document.documentElement;
  // "system" removes the attribute so the @media (prefers-color-scheme) rule wins.
  if (theme === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", theme);
}

const themeIcons: Record<Theme, JSX.Element> = {
  light: (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
    </svg>
  ),
  dark: (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" />
    </svg>
  ),
  system: (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <rect x="3" y="4" width="18" height="12" rx="2" />
      <path d="M8 20h8M12 16v4" />
    </svg>
  ),
};

function ThemeToggle() {
  const [theme, setTheme] = useState<Theme>(readTheme);

  useEffect(() => {
    applyTheme(theme);
    try {
      localStorage.setItem(THEME_KEY, theme);
    } catch {
      /* ignore persistence failures */
    }
  }, [theme]);

  const options: Theme[] = ["light", "system", "dark"];
  return (
    <div className="theme-toggle">
      <div className="tt-label">Theme</div>
      <div className="tt-group" role="group" aria-label="Color theme">
        {options.map((opt) => (
          <button
            key={opt}
            type="button"
            className={theme === opt ? "active" : ""}
            aria-pressed={theme === opt}
            onClick={() => setTheme(opt)}
          >
            {themeIcons[opt]}
            {opt.charAt(0).toUpperCase() + opt.slice(1)}
          </button>
        ))}
      </div>
    </div>
  );
}

const links: [string, string][] = [
  ["/", "Control Room"],
  ["/run", "Run Page"],
  ["/timeline", "Session Timeline"],
  ["/patch", "Patch Review"],
  ["/risk", "Command Risk"],
  ["/usage", "Token Usage"],
  ["/benchmarks", "Benchmarks"],
  ["/rollback", "Rollback Points"],
  ["/github", "GitHub"],
  ["/ratify", "Ratify"],
  ["/system", "System"],
];

// Apply the saved theme as early as possible to avoid a flash of the wrong palette.
applyTheme(readTheme());

export default function App() {
  const [version, setVersion] = useState<string>("");
  useEffect(() => {
    fetch("/api/health")
      .then((r) => r.json())
      .then((d) => setVersion(d.version ?? ""))
      .catch(() => {});
  }, []);

  return (
    <div className="layout">
      <a href="#main" className="skip-link">
        Skip to content
      </a>
      <aside className="sidebar">
        <div className="brand">
          <Mark size={20} />
          Trace
        </div>
        <nav className="nav" aria-label="Primary">
          {links.map(([to, label]) => (
            <NavLink key={to} to={to} end={to === "/"}>
              {label}
            </NavLink>
          ))}
        </nav>
        <ThemeToggle />
        <div className="local-note">
          <span className="dot" /> Local only · 127.0.0.1
          <div className="local-sub">Your data never leaves this machine.</div>
          {version && <div className="local-sub">Trace v{version}</div>}
        </div>
      </aside>
      <main className="content" id="main">
        <Outlet />
      </main>
      <CommandPalette />
    </div>
  );
}
