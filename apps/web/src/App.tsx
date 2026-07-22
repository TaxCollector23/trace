import { useEffect, useState } from "react";
import { NavLink, Outlet } from "react-router-dom";
import Tracey from "./Tracey";

const links: [string, string][] = [
  ["/", "Dashboard"],
  ["/timeline", "Session Timeline"],
  ["/patch", "Patch Review"],
  ["/risk", "Command Risk"],
  ["/cost", "Token Spend"],
  ["/rollback", "Rollback Points"],
  ["/github", "Integration Status"],
];

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
      <aside className="sidebar">
        <div className="brand">
          Trace
          <Tracey size={22} expression="watching" />
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
          {version && <div className="local-sub">Trace v{version}</div>}
        </div>
      </aside>
      <main className="content">
        <Outlet />
      </main>
    </div>
  );
}
