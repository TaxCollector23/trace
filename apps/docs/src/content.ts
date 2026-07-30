// Loads the docs markdown from the repo's /docs folder (single source of truth),
// strips frontmatter, and converts the handful of Mintlify components into clean
// GitHub-flavoured markdown so react-markdown can render them.

import navConfig from "../../../docs/docs.json";

export interface DocPage {
  slug: string; // e.g. "overview" or "integrations/github"
  title: string;
  description: string;
  markdown: string;
}

export interface NavGroup {
  group: string;
  pages: { slug: string; title: string }[];
}

// Eagerly import every .mdx under /docs as raw text.
const raw = import.meta.glob("../../../docs/**/*.mdx", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/** Map an absolute glob key to a doc slug ("overview", "integrations/github"). */
function keyToSlug(key: string): string {
  const m = key.match(/\/docs\/(.+)\.mdx$/);
  return m ? m[1] : key;
}

/** Pull `title` and `description` from YAML-ish frontmatter; return rest as body. */
function parseFrontmatter(src: string): { title: string; description: string; body: string } {
  const fm = src.match(/^---\n([\s\S]*?)\n---\n?/);
  let title = "";
  let description = "";
  let body = src;
  if (fm) {
    body = src.slice(fm[0].length);
    for (const line of fm[1].split("\n")) {
      const t = line.match(/^title:\s*(.*)$/);
      const d = line.match(/^description:\s*(.*)$/);
      if (t) title = t[1].trim().replace(/^["']|["']$/g, "");
      if (d) description = d[1].trim().replace(/^["']|["']$/g, "");
    }
  }
  return { title, description, body };
}

/** Convert Mintlify JSX components to markdown equivalents. */
function transform(md: string): string {
  let out = md;

  // Card groups: drop the wrapper, keep inner cards.
  out = out.replace(/<\/?CardGroup[^>]*>/g, "");
  // <Card title="X" icon="..."> ... </Card>  ->  ### X \n ...
  out = out.replace(
    /<Card\s+title="([^"]*)"[^>]*>([\s\S]*?)<\/Card>/g,
    (_m, title, inner) => `\n### ${title}\n${inner.trim()}\n`
  );
  // Steps / Step
  out = out.replace(/<\/?Steps[^>]*>/g, "");
  out = out.replace(
    /<Step\s+title="([^"]*)"[^>]*>([\s\S]*?)<\/Step>/g,
    (_m, title, inner) => `\n### ${title}\n${inner.trim()}\n`
  );
  // Tabs / Tab
  out = out.replace(/<\/?Tabs[^>]*>/g, "");
  out = out.replace(
    /<Tab\s+title="([^"]*)"[^>]*>([\s\S]*?)<\/Tab>/g,
    (_m, title, inner) => `\n#### ${title}\n${inner.trim()}\n`
  );
  // Callouts: Note / Warning / Info / Tip / Check -> blockquote with a label.
  const callout = (tag: string, label: string) => {
    const re = new RegExp(`<${tag}[^>]*>([\\s\\S]*?)<\\/${tag}>`, "g");
    out = out.replace(re, (_m, inner) => {
      const text = inner.trim().replace(/\n+/g, " ");
      return `\n> **${label}** ${text}\n`;
    });
  };
  callout("Note", "Note");
  callout("Warning", "Warning");
  callout("Info", "Info");
  callout("Tip", "Tip");
  callout("Check", "Check");

  // Any stray self-closing or unknown components: drop the tags, keep text.
  out = out.replace(/<\/?[A-Z][A-Za-z]*[^>]*>/g, "");

  return out.trim();
}

const pages: Record<string, DocPage> = {};
for (const [key, src] of Object.entries(raw)) {
  const slug = keyToSlug(key);
  const { title, description, body } = parseFrontmatter(src);
  pages[slug] = { slug, title: title || slug, description, markdown: transform(body) };
}

export function getPage(slug: string): DocPage | undefined {
  return pages[slug];
}

/** Build sidebar groups from docs.json, falling back gracefully. */
export function getNav(): NavGroup[] {
  const groups = (navConfig as any)?.navigation?.groups ?? [];
  return groups.map((g: any) => ({
    group: g.group,
    pages: (g.pages as string[])
      .map((slug) => ({ slug, title: pages[slug]?.title ?? slug }))
      .filter((p) => pages[p.slug]),
  }));
}

/** Flattened ordered slug list (for prev/next + default route). */
export function orderedSlugs(): string[] {
  return getNav().flatMap((g) => g.pages.map((p) => p.slug));
}

/** Lightweight search index over titles + body text. */
export function search(query: string): DocPage[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  return Object.values(pages)
    .filter(
      (p) =>
        p.title.toLowerCase().includes(q) ||
        p.description.toLowerCase().includes(q) ||
        p.markdown.toLowerCase().includes(q)
    )
    .slice(0, 8);
}
