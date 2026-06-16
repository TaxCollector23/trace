import { useEffect, useMemo } from "react";
import { Link, useLocation } from "react-router-dom";
import Markdown from "./Markdown";
import { getPage, orderedSlugs } from "./content";

interface Heading {
  id: string;
  text: string;
  level: number;
}

/** Extract h2/h3 headings from markdown for the right-hand "On this page" TOC. */
function extractHeadings(md: string): Heading[] {
  const out: Heading[] = [];
  let inCode = false;
  for (const line of md.split("\n")) {
    if (line.trim().startsWith("```")) inCode = !inCode;
    if (inCode) continue;
    const m = line.match(/^(#{2,3})\s+(.*)$/);
    if (m) {
      const text = m[2].replace(/[`*_]/g, "").trim();
      const id = text
        .toLowerCase()
        .replace(/[^a-z0-9 -]/g, "")
        .replace(/\s+/g, "-");
      out.push({ id, text, level: m[1].length });
    }
  }
  return out;
}

export default function Doc() {
  const location = useLocation();
  const slug = location.pathname.replace(/^\//, "") || "overview";
  const page = getPage(slug);

  const headings = useMemo(() => (page ? extractHeadings(page.markdown) : []), [page]);

  const order = orderedSlugs();
  const idx = order.indexOf(slug);
  const prev = idx > 0 ? order[idx - 1] : null;
  const next = idx >= 0 && idx < order.length - 1 ? order[idx + 1] : null;

  useEffect(() => {
    window.scrollTo(0, 0);
  }, [slug]);

  if (!page) {
    return (
      <main className="content">
        <div className="notfound">
          <h1>Page not found</h1>
          <p>
            <Link to="/">Back to docs</Link>
          </p>
        </div>
      </main>
    );
  }

  return (
    <>
      <main className="content">
        <div className="eyebrow">Documentation</div>
        <h1 className="doc-title">{page.title}</h1>
        {page.description && <p className="doc-desc">{page.description}</p>}

        <Markdown source={page.markdown} />

        <div className="pager">
          {prev ? (
            <Link className="pager-link prev" to={`/${prev}`}>
              <span>Previous</span>
              {getPage(prev)?.title}
            </Link>
          ) : (
            <span />
          )}
          {next ? (
            <Link className="pager-link next" to={`/${next}`}>
              <span>Next</span>
              {getPage(next)?.title}
            </Link>
          ) : (
            <span />
          )}
        </div>
      </main>

      <aside className="toc">
        {headings.length > 0 && (
          <>
            <div className="toc-title">On this page</div>
            {headings.map((h) => (
              <button
                key={h.id}
                className={`toc-l${h.level}`}
                onClick={() =>
                  document
                    .getElementById(h.id)
                    ?.scrollIntoView({ behavior: "smooth", block: "start" })
                }
              >
                {h.text}
              </button>
            ))}
          </>
        )}
      </aside>
    </>
  );
}
