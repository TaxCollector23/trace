import { Link, NavLink, Outlet } from "react-router-dom";
import { DOCS_URL, GITHUB_REPO } from "./config";

export default function App() {
  return (
    <div className="site">
      <header className="topbar">
        <Link to="/" className="logo">
          Trace<span>Guard</span>
        </Link>
        <nav className="topnav">
          <NavLink to="/about">About</NavLink>
          <a href={DOCS_URL} target="_blank" rel="noreferrer">
            Docs
          </a>
          <a href={GITHUB_REPO} target="_blank" rel="noreferrer">
            GitHub
          </a>
          <a className="btn-primary" href="#download" style={{ padding: "8px 16px", fontSize: 14 }}>
            Install
          </a>
        </nav>
      </header>

      <main>
        <Outlet />
      </main>

      <footer className="footer">
        <div className="f-brand">
          <div className="logo">
            Trace<span>Guard</span>
          </div>
          <p>
            The local-first control layer for AI coding agents. The real
            dashboard runs only on <code>127.0.0.1</code>; this site is for
            installation and docs and never connects to your local daemon.
          </p>
        </div>
        <div className="f-col">
          <h4>Product</h4>
          <a href="#features">Features</a>
          <a href="#download">Install</a>
          <Link to="/about">About</Link>
        </div>
        <div className="f-col">
          <h4>Resources</h4>
          <a href={DOCS_URL} target="_blank" rel="noreferrer">
            Documentation
          </a>
          <a href={GITHUB_REPO} target="_blank" rel="noreferrer">
            GitHub
          </a>
          <a href={`${GITHUB_REPO}/releases`} target="_blank" rel="noreferrer">
            Releases
          </a>
        </div>
        <div className="f-col">
          <h4>Local-first</h4>
          <span className="muted" style={{ fontSize: 13 }}>
            Zero-cloud by default. Redacted secrets. MIT licensed.
          </span>
        </div>
      </footer>
    </div>
  );
}
