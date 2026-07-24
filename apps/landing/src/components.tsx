import type { AnchorHTMLAttributes, ReactNode } from "react";
import { Link } from "react-router-dom";
import { motion } from "framer-motion";

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
