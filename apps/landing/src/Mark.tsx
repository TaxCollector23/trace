/** The Trace logotype mark — a rounded square with a blue "T", matching
 * public/favicon.svg. Inline so it can be sized/recolored without an <img>. */
export function Mark({ size = 24, className }: { size?: number; className?: string }) {
  return (
    <svg viewBox="0 0 256 256" width={size} height={size} className={className} aria-hidden="true">
      <rect width="256" height="256" rx="56" fill="#f5f5f6" />
      <rect x="62" y="70" width="132" height="34" rx="8" fill="#5b93f5" />
      <rect x="111" y="104" width="34" height="94" rx="8" fill="#5b93f5" />
    </svg>
  );
}
