import { useEffect, useRef, useState } from "react";
import { LOADING, type Resource } from "./data";

/** Subscribe to a `Resource<T>` producer. Handles abort on unmount/dep-change,
 * an explicit `reload()`, and optional polling for live views. The producer
 * receives an AbortSignal so an in-flight request is cancelled cleanly.
 *
 * `stalled` becomes true when a poll cycle has not produced a fresh successful
 * result within `stallAfterMs` — the honest "stream stalled" signal (§121).
 */
export function useResource<T>(
  producer: (signal: AbortSignal) => Promise<Resource<T>>,
  opts: { deps?: unknown[]; pollMs?: number; stallAfterMs?: number } = {}
): { resource: Resource<T>; reload: () => void; stalled: boolean; lastOkAt: number | null } {
  const { deps = [], pollMs, stallAfterMs } = opts;
  const [resource, setResource] = useState<Resource<T>>(LOADING);
  const [nonce, setNonce] = useState(0);
  const [stalled, setStalled] = useState(false);
  const lastOkRef = useRef<number | null>(null);
  const [lastOkAt, setLastOkAt] = useState<number | null>(null);

  // Keep the latest producer without making it a re-subscribe trigger.
  const producerRef = useRef(producer);
  producerRef.current = producer;

  useEffect(() => {
    let alive = true;
    const controller = new AbortController();

    const run = () => {
      producerRef.current(controller.signal)
        .then((r) => {
          if (!alive || r.state === "loading") return;
          setResource(r);
          if (r.state === "ok") {
            const now = Date.now();
            lastOkRef.current = now;
            setLastOkAt(now);
            setStalled(false);
          }
        })
        .catch(() => {
          /* fetchResource never rejects; this is belt-and-braces. */
        });
    };

    run();

    let pollTimer: ReturnType<typeof setInterval> | undefined;
    let stallTimer: ReturnType<typeof setInterval> | undefined;
    if (pollMs && pollMs > 0) {
      pollTimer = setInterval(run, pollMs);
    }
    if (stallAfterMs && stallAfterMs > 0) {
      stallTimer = setInterval(() => {
        if (!alive) return;
        const last = lastOkRef.current;
        if (last != null && Date.now() - last > stallAfterMs) setStalled(true);
      }, Math.min(stallAfterMs, 2000));
    }

    return () => {
      alive = false;
      controller.abort();
      if (pollTimer) clearInterval(pollTimer);
      if (stallTimer) clearInterval(stallTimer);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, nonce, pollMs, stallAfterMs]);

  return { resource, reload: () => setNonce((n) => n + 1), stalled, lastOkAt };
}
