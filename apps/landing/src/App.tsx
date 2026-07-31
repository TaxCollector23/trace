import { Link, NavLink, Outlet } from "react-router-dom";
import { Mark } from "./Mark";
import { DownloadMenu } from "./components";
import { DOCS_URL, GITHUB_REPO } from "./config";

export default function App() {
  return (
    <div className="flex min-h-screen flex-col">
      <header className="sticky top-0 z-30 border-b border-border bg-bg/80 backdrop-blur-md">
        <div className="mx-auto flex max-w-content items-center justify-between px-6 py-5">
          <Link to="/" className="flex items-center gap-4 text-lg font-semibold text-text">
            <Mark size={30} />
            Trace
          </Link>
          <nav className="flex items-center gap-8 md:gap-10">
            <NavLink
              to="/dashboard"
              className={({ isActive }) =>
                `text-base font-medium transition-colors ${isActive ? "text-text" : "text-text-dim hover:text-text"}`
              }
            >
              Dashboard
            </NavLink>
            <NavLink
              to="/about"
              className={({ isActive }) =>
                `text-base font-medium transition-colors ${isActive ? "text-text" : "text-text-dim hover:text-text"}`
              }
            >
              About
            </NavLink>
            <a
              href={DOCS_URL}
              target="_blank"
              rel="noreferrer"
              className="text-base font-medium text-text-dim transition-colors hover:text-text"
            >
              Docs
            </a>
            <a
              href={GITHUB_REPO}
              target="_blank"
              rel="noreferrer"
              aria-label="GitHub"
              className="opacity-70 transition-opacity hover:opacity-100"
            >
              <img src="/logos/github.png" alt="GitHub" className="h-6 w-6 object-contain" />
            </a>
            <DownloadMenu />
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
            <div className="flex items-center gap-4 text-base font-semibold text-text">
              <Mark size={26} />
              Trace
            </div>
          </div>
          <div>
            <div className="mb-3 text-xs font-medium uppercase tracking-wide text-text-dim">Product</div>
            <ul className="space-y-2 text-sm">
              <li><Link className="text-text-dim transition-colors hover:text-text" to="/dashboard">Dashboard</Link></li>
              <li><Link className="text-text-dim transition-colors hover:text-text" to="/download">Download desktop</Link></li>
              <li><Link className="text-text-dim transition-colors hover:text-text" to="/cli">CLI install</Link></li>
              <li><Link className="text-text-dim transition-colors hover:text-text" to="/about">About</Link></li>
            </ul>
          </div>
          <div>
            <div className="mb-3 text-xs font-medium uppercase tracking-wide text-text-dim">Resources</div>
            <ul className="space-y-2.5 text-sm">
              <li><a className="text-text-dim transition-colors hover:text-text" href={DOCS_URL} target="_blank" rel="noreferrer">Documentation</a></li>
              <li>
                <a href={GITHUB_REPO} target="_blank" rel="noreferrer" aria-label="GitHub" className="inline-flex opacity-70 transition-opacity hover:opacity-100">
                  <img src="/logos/github.png" alt="GitHub" className="h-4 w-4 object-contain" />
                </a>
              </li>
              <li><a className="text-text-dim transition-colors hover:text-text" href={`${GITHUB_REPO}/releases`} target="_blank" rel="noreferrer">Releases</a></li>
            </ul>
          </div>
        </div>
      </footer>
    </div>
  );
}
