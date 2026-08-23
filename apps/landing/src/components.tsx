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
 * height, radius, and press animation, no more one-off className drift. */
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
// Hoverable nav link. Primary click goes to /download; hover reveals a
// clean, spacious menu with each OS on its own row (macOS/Windows/Linux),
// plus a "CLI" row to /cli and a "All releases" footer link. No OS
// detection, no per-row icons, the OS name is the affordance, and hover
// state is the download hint. A short delay before close prevents the
// menu vanishing when the cursor briefly crosses a gap.

const OS_ROWS: Array<{ label: string; sub: string; href: string }> = [
  { label: "macOS", sub: "Apple Silicon · .dmg", href: DOWNLOADS.macOS },
  { label: "Windows", sub: "x64 · installer .exe", href: DOWNLOADS.windows },
  { label: "Linux", sub: ".deb  ·  AppImage", href: DOWNLOADS.linuxDeb },
];

export function DownloadMenu() {
  const [open, setOpen] = useState(false);
  // Small close delay so the menu doesn't blink shut when the cursor
  // travels from the trigger down to the panel (there's a 8px gap the
  // browser reports as a mouseleave otherwise).
  const closeTimer = { current: null as ReturnType<typeof setTimeout> | null };
  const openNow = () => {
    if (closeTimer.current) clearTimeout(closeTimer.current);
    setOpen(true);
  };
  const closeSoon = () => {
    closeTimer.current = setTimeout(() => setOpen(false), 120);
  };

  return (
    <div className="relative" onMouseEnter={openNow} onMouseLeave={closeSoon}>
      <Link
        to="/download"
        className="inline-flex h-9 items-center gap-1.5 rounded-full bg-brand px-4 text-sm font-medium text-white shadow-sm transition-transform hover:-translate-y-[1px]"
        onFocus={openNow}
        onBlur={closeSoon}
      >
        Download
        <svg width="9" height="9" viewBox="0 0 10 10" aria-hidden className="opacity-80">
          <path d="M2 3.5L5 6.5L8 3.5" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </Link>

      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0, y: -4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -4 }}
            transition={{ duration: 0.12 }}
            className="absolute right-0 top-full z-50 mt-2 w-72 overflow-hidden rounded-2xl border border-border bg-white shadow-[0_12px_36px_-12px_rgba(0,0,0,0.18)]"
            onMouseEnter={openNow}
            onMouseLeave={closeSoon}
          >
            <div className="px-5 pt-4 pb-2 text-[10px] font-semibold uppercase tracking-[0.09em] text-text-dim">
              Desktop app
            </div>
            <ul className="px-2 pb-2">
              {OS_ROWS.map((r) => (
                <li key={r.label}>
                  <a
                    href={r.href}
                    className="block rounded-lg px-3 py-2.5 transition-colors hover:bg-brand/[0.06]"
                  >
                    <div className="text-sm font-medium text-text">{r.label}</div>
                    <div className="mt-0.5 text-xs text-text-dim">{r.sub}</div>
                  </a>
                </li>
              ))}
            </ul>
            <div className="border-t border-border px-5 pt-3 pb-2 text-[10px] font-semibold uppercase tracking-[0.09em] text-text-dim">
              Command line
            </div>
            <div className="px-2 pb-2">
              <a
                href="/#install"
                className="block rounded-lg px-3 py-2.5 transition-colors hover:bg-brand/[0.06]"
              >
                <div className="text-sm font-medium text-text">CLI</div>
                <div className="mt-0.5 text-xs text-text-dim">brew  ·  curl | sh  ·  irm | iex  ·  npm</div>
              </a>
            </div>
            <a
              href={DOWNLOADS.releases}
              target="_blank"
              rel="noreferrer"
              className="block border-t border-border px-5 py-2.5 text-xs text-text-dim transition-colors hover:bg-black/[0.02] hover:text-text"
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
