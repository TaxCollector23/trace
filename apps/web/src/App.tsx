import { NavLink, Outlet } from "react-router-dom";

const links: [string, string][] = [
  ["/", "Dashboard"],
  ["/timeline", "Run Timeline"],
  ["/patch", "Patch Review"],
  ["/cost", "Cost Center"],
  ["/risk", "Risk Center"],
  ["/rollback", "Rollback Center"],
  ["/prompt-compressor", "Prompt Compressor"],
];

export default function App() {
  return (
    <div className="layout">
      <aside className="sidebar">
        <div className="brand">
          Trace<span>Guard</span>
        </div>
        <nav className="nav">
          {links.map(([to, label]) => (
            <NavLink key={to} to={to} end={to === "/"}>
              {label}
            </NavLink>
          ))}
        </nav>
        <div className="local-note">
          <span className="dot" /> Local only · 127.0.0.1
          <div className="local-sub">Your data never leaves this machine.</div>
        </div>
      </aside>
      <main className="content">
        <Outlet />
      </main>
    </div>
  );
}
