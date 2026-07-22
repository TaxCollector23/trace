import { Link, NavLink, Outlet } from "react-router-dom";
import Tracey from "./Tracey";
import { Mark } from "./Mark";
import { DOCS_URL, GITHUB_REPO } from "./config";

export default function App() {
  return (
    <div className="flex min-h-screen flex-col">
      <header>
        <div className="mx-auto flex max-w-content items-center justify-between px-6 py-4">
          <Link to="/" className="flex items-center gap-2 text-[17px] font-semibold text-text">
            <Mark size={26} />
            Trace
            <Tracey size={24} expression="happy" />
          </Link>
          <nav className="flex items-center gap-6">
            <NavLink
              to="/about"
              className="text-sm text-text-dim hover:text-text"
            >
              About
            </NavLink>
            <a
              href={DOCS_URL}
              target="_blank"
              rel="noreferrer"
              className="text-sm text-text-dim hover:text-text"
            >
              Docs
            </a>
            <a
              href={GITHUB_REPO}
              target="_blank"
              rel="noreferrer"
              className="text-sm text-text-dim hover:text-text"
            >
              GitHub
            </a>
            <a
              href="#download"
              className="rounded bg-surface px-3 py-1.5 text-sm text-text hover:bg-surface-2"
            >
              Install
            </a>
          </nav>
        </div>
      </header>

      <main className="flex-1">
        <div className="mx-auto max-w-content px-6">
          <Outlet />
        </div>
      </main>

      <footer className="relative">
        <Tracey size={48} expression="peek" className="pointer-events-none absolute -top-6 right-8 opacity-90" />
        <div className="mx-auto grid max-w-content grid-cols-1 gap-8 px-6 py-10 sm:grid-cols-[1.4fr_1fr_1fr]">
          <div>
            <div className="flex items-center gap-2 text-[15px] font-semibold text-text">
              <Mark size={20} />
              Trace
            </div>
            <p className="mt-2 max-w-[320px] text-sm text-text-dim">
              The trust layer for AI software engineering. The real dashboard
              runs only on <code className="text-text">127.0.0.1</code>; this
              site is for installation and docs, and never connects to your
              local daemon.
            </p>
          </div>
          <div>
            <div className="mb-2 text-xs uppercase tracking-wide text-text-dim">Product</div>
            <ul className="space-y-1.5 text-sm">
              <li><a className="text-text-dim hover:text-text" href="#features">Features</a></li>
              <li><a className="text-text-dim hover:text-text" href="#download">Install</a></li>
              <li><Link className="text-text-dim hover:text-text" to="/about">About</Link></li>
            </ul>
          </div>
          <div>
            <div className="mb-2 text-xs uppercase tracking-wide text-text-dim">Resources</div>
            <ul className="space-y-1.5 text-sm">
              <li><a className="text-text-dim hover:text-text" href={DOCS_URL} target="_blank" rel="noreferrer">Documentation</a></li>
              <li><a className="text-text-dim hover:text-text" href={GITHUB_REPO} target="_blank" rel="noreferrer">GitHub</a></li>
              <li><a className="text-text-dim hover:text-text" href={`${GITHUB_REPO}/releases`} target="_blank" rel="noreferrer">Releases</a></li>
            </ul>
          </div>
        </div>
      </footer>
    </div>
  );
}
