import { useState } from "react";

/** A copyable command block. Developer-tool look, no decoration. */
export function Cmd({ children }: { children: string }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(children);
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    } catch {
      /* clipboard may be unavailable; ignore */
    }
  };
  return (
    <div className="cmd">
      <code>
        <span className="prompt">$</span> {children}
      </code>
      <button onClick={copy} aria-label="Copy command">
        {copied ? "copied" : "copy"}
      </button>
    </div>
  );
}

export function Section({
  id,
  title,
  kicker,
  children,
}: {
  id?: string;
  title: string;
  kicker?: string;
  children: React.ReactNode;
}) {
  return (
    <section id={id} className="section">
      {kicker && <div className="kicker">{kicker}</div>}
      <h2>{title}</h2>
      {children}
    </section>
  );
}
