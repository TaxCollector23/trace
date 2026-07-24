import { Link, NavLink, Outlet } from "react-router-dom";
import { motion } from "framer-motion";
import { Mark } from "./Mark";
import { DOCS_URL, GITHUB_REPO } from "./config";

export default function App() {
  return (
    <div className="flex min-h-screen flex-col">
      <header className="sticky top-0 z-30 border-b border-border bg-bg/80 backdrop-blur-md">
        <div className="mx-auto flex max-w-content items-center justify-between px-6 py-4">
          <Link to="/" className="flex items-center gap-2.5 text-[17px] font-semibold text-text">
            <Mark size={28} />
            Trace
          </Link>
          <nav className="flex items-center gap-7">
            <NavLink
              to="/about"
              className={({ isActive }) =>
                `text-sm font-medium transition-colors ${isActive ? "text-text" : "text-text-dim hover:text-text"}`
              }
            >
              About
            </NavLink>
            <a
              href={DOCS_URL}
              target="_blank"
              rel="noreferrer"
              className="text-sm font-medium text-text-dim transition-colors hover:text-text"
            >
              Docs
            </a>
            <a
              href={GITHUB_REPO}
              target="_blank"
              rel="noreferrer"
              className="text-sm font-medium text-text-dim transition-colors hover:text-text"
            >
              GitHub
            </a>
            <motion.div whileHover={{ y: -1 }} whileTap={{ scale: 0.96 }}>
              <Link
                to="/download"
                className="btn-pop rounded-full bg-text px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-brand-dim"
              >
                Download
              </Link>
            </motion.div>
          </nav>
        </div>
      </header>

      <main className="flex-1">
        <div className="mx-auto max-w-content px-6">
          <Outlet />
        </div>
      </main>

      <footer className="border-t border-border">
        <div className="mx-auto grid max-w-content grid-cols-1 gap-8 px-6 py-12 sm:grid-cols-[1.4fr_1fr_1fr]">
          <div>
            <div className="flex items-center gap-2.5 text-[15px] font-semibold text-text">
              <Mark size={22} />
              Trace
            </div>
            <p className="mt-3 max-w-[320px] text-sm leading-relaxed text-text-dim">
              The trust layer for AI software engineering. The dashboard runs only
              on <code className="text-text">127.0.0.1</code> — this site is for
              downloads and docs, and never touches your local daemon.
            </p>
          </div>
          <div>
            <div className="mb-3 text-xs font-medium uppercase tracking-wide text-text-dimmer">Product</div>
            <ul className="space-y-2 text-sm">
              <li><a className="text-text-dim transition-colors hover:text-text" href="#integrations">Integrations</a></li>
              <li><Link className="text-text-dim transition-colors hover:text-text" to="/download">Download</Link></li>
              <li><Link className="text-text-dim transition-colors hover:text-text" to="/about">About</Link></li>
            </ul>
          </div>
          <div>
            <div className="mb-3 text-xs font-medium uppercase tracking-wide text-text-dimmer">Resources</div>
            <ul className="space-y-2 text-sm">
              <li><a className="text-text-dim transition-colors hover:text-text" href={DOCS_URL} target="_blank" rel="noreferrer">Documentation</a></li>
              <li><a className="text-text-dim transition-colors hover:text-text" href={GITHUB_REPO} target="_blank" rel="noreferrer">GitHub</a></li>
              <li><a className="text-text-dim transition-colors hover:text-text" href={`${GITHUB_REPO}/releases`} target="_blank" rel="noreferrer">Releases</a></li>
            </ul>
          </div>
        </div>
      </footer>
    </div>
  );
}
