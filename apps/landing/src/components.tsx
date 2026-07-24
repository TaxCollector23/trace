import type { ReactNode } from "react";
import { motion } from "framer-motion";

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
