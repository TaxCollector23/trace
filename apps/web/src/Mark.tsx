/** The Trace logotype mark — a rounded blue square with a white "T". */
export function Mark({ size = 24, className }: { size?: number; className?: string }) {
  return (
    <svg viewBox="0 0 256 256" width={size} height={size} className={className} aria-hidden="true">
      <rect width="256" height="256" rx="56" fill="#2f6fed" />
      <rect x="62" y="64" width="132" height="30" rx="8" fill="#ffffff" />
      <rect x="113" y="106" width="30" height="90" rx="8" fill="#ffffff" />
    </svg>
  );
}
