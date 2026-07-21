import { useMemo, useState } from "react";
import { NavLink, Outlet, useNavigate } from "react-router-dom";
import { getNav, search } from "./content";

const GITHUB = "https://github.com/TaxCollector23/trace";

export default function App() {
  const nav = useMemo(() => getNav(), []);
  const [q, setQ] = useState("");
  const results = useMemo(() => search(q), [q]);
  const navigate = useNavigate();

  return (
    <div className="layout">
      <aside className="sidebar">
        <a className="brand" href={`#/`}>
          Trace
          <em>docs</em>
        </a>

        <div className="search">
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Search docs…"
            aria-label="Search docs"
          />
          {q && (
            <div className="search-results">
              {results.length === 0 ? (
                <div className="sr-empty">No matches</div>
              ) : (
                results.map((r) => (
                  <button
                    key={r.slug}
                    onClick={() => {
                      navigate(`/${r.slug}`);
                      setQ("");
                    }}
                  >
                    <b>{r.title}</b>
                    <span>{r.description}</span>
                  </button>
                ))
              )}
            </div>
          )}
        </div>

        <nav>
          {nav.map((g) => (
            <div className="nav-group" key={g.group}>
              <div className="nav-group-title">{g.group}</div>
              {g.pages.map((p) => (
                <NavLink key={p.slug} to={`/${p.slug}`}>
                  {p.title}
                </NavLink>
              ))}
            </div>
          ))}
        </nav>

        <div className="side-foot">
          <a href={GITHUB} target="_blank" rel="noreferrer">
            GitHub
          </a>
        </div>
      </aside>

      <Outlet />
    </div>
  );
}
