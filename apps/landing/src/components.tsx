import type { AnchorHTMLAttributes, ReactNode } from "react";
import { useState } from "react";
import { Link } from "react-router-dom";
import { motion, AnimatePresence } from "framer-motion";
import { DOWNLOADS } from "./config";

interface ButtonProps extends AnchorHTMLAttributes<HTMLAnchorElement> {
  variant?: "primary" | "secondary";
  to?: string;
  children: ReactNode;
}

/** Every big pill-shaped button on the site, guaranteed identical padding,
 * height, radius, and press animation — no more one-off className drift. */
export function Button({ variant = "primary", to, children, className = "", ...rest }: ButtonProps) {
  const base =
    "btn-pop flex h-12 items-center justify-center gap-2.5 rounded-full px-6 text-[15px] font-medium whitespace-nowrap";
  const tone =
    variant === "primary"
      ? "bg-brand text-white shadow"
      : "border border-border bg-white text-text";
  const cls = `${base} ${tone} ${className}`;

  return (
    <motion.div whileHover={{ y: -2 }} whileTap={{ scale: 0.96 }} className="inline-block">
      {to ? (
        <Link to={to} className={cls}>
          {children}
        </Link>
      ) : (
        <a className={cls} {...rest}>
          {children}
        </a>
      )}
    </motion.div>
  );
}

export function Section({
  id,
  title,
  lede,
  children,
}: {
  id?: string;
  title?: string;
  lede?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section id={id} className="relative py-16">
      {title && <h2 className="font-serif text-3xl text-text">{title}</h2>}
      {lede && <p className="mt-2.5 max-w-[640px] text-text-dim">{lede}</p>}
      <div className="relative mt-8">{children}</div>
    </section>
  );
}

// -- Download menu ----------------------------------------------------------
// Hoverable nav link. Primary click goes to /download (the on-site page);
// hover augments with direct one-click downloads for each OS and a link to
// the CLI page. No OS-detection pill — every option gets equal weight so
// the menu reads cleanly.

const OS_ROWS: Array<{ label: string; sub: string; href: string }> = [
  { label: "macOS", sub: "Apple Silicon · .dmg", href: DOWNLOADS.macOS },
  { label: "Windows", sub: "x64 · installer .exe", href: DOWNLOADS.windows },
  { label: "Linux", sub: ".deb + AppImage", href: DOWNLOADS.linuxDeb },
];

export function DownloadMenu() {
  const [open, setOpen] = useState(false);

  return (
    <div
      className="relative"
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => setOpen(false)}
      onFocus={() => setOpen(true)}
      onBlur={() => setOpen(false)}
    >
      <Link
        to="/download"
        className="inline-flex items-center gap-1 text-base font-medium text-brand transition-colors hover:text-brand-dim"
      >
        Download
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden className="opacity-60">
          <path d="M2 3.5L5 6.5L8 3.5" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </Link>

      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0, y: -6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -6 }}
            transition={{ duration: 0.14 }}
            className="absolute right-0 top-full z-50 mt-2 w-64 overflow-hidden rounded-2xl border border-border bg-white shadow-lg"
          >
            <div className="border-b border-border px-4 py-2 text-[11px] font-medium uppercase tracking-wide text-text-dim">
              Desktop app
            </div>
            <ul>
              {OS_ROWS.map((r) => (
                <li key={r.label}>
                  <a
                    href={r.href}
                    className="flex items-center justify-between px-4 py-3 transition-colors hover:bg-brand/5"
                  >
                    <div>
                      <div className="text-sm font-medium text-text">{r.label}</div>
                      <div className="text-xs text-text-dim">{r.sub}</div>
                    </div>
                    <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden className="text-text-dim">
                      <path d="M7 2v8m0 0L4 7m3 3l3-3M2 12h10" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
                    </svg>
                  </a>
                </li>
              ))}
            </ul>
            <Link
              to="/cli"
              className="flex items-center justify-between border-t border-border px-4 py-3 transition-colors hover:bg-brand/5"
            >
              <div>
                <div className="text-sm font-medium text-text">CLI</div>
                <div className="text-xs text-text-dim">brew · curl · PowerShell</div>
              </div>
              <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden className="text-text-dim">
                <path d="M5 3l4 4-4 4" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
            </Link>
            <a
              href={DOWNLOADS.releases}
              target="_blank"
              rel="noreferrer"
              className="block border-t border-border px-4 py-2 text-xs text-text-dim transition-colors hover:text-text"
            >
              All releases · checksums →
            </a>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

/** Fades a section's contents in as it enters the viewport. One pass, no loop. */
export function Reveal({
  children,
  delay = 0,
  className,
}: {
  children: ReactNode;
  delay?: number;
  className?: string;
}) {
  return (
    <motion.div
      className={className}
      initial={{ opacity: 0, y: 10 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-80px" }}
      transition={{ duration: 0.45, delay, ease: [0.16, 1, 0.3, 1] }}
    >
      {children}
    </motion.div>
  );
}
