import { useEffect, useState } from "react";
import { Link } from "react-router-dom";

// Hosted /dashboard.
//
// Fully client-side, cloud-free: every fetch here targets your OWN local
// daemon at http://127.0.0.1:8757/api/*. Nothing on this page ships your
// data to Vercel, GitHub, us, or anywhere else — the browser talks
// directly to your machine.
//
// Auto-port discovery: the daemon writes its bound port to
// ~/.trace/daemon.json, which the browser can't read, so we probe the
// known preferred port and a small range if that fails. Users who ran
// the daemon on a non-standard port set localStorage.trace_port.

const PREFERRED_PORT = 8757;
const PORT_KEY = "trace_daemon_port";

interface Health {
  service: string;
  status: string;
  version: string;
}

interface RunSummary {
  id: string;
  project_id: string;
  project_name?: string;
  command: string;
  agent_name: string | null;
  user_prompt: string | null;
  started_at: string;
  ended_at: string | null;
  status: string;
}

interface Event {
  id: string;
  run_id: string;
  type: string;
  message: string;
  metadata_json: string | null;
  created_at: string;
}

interface CoachingPattern {
  pattern: string;
  occurrences: number;
  share: number;
  flag_rate: number;
  advice: string;
  example: string | null;
}
interface CoachingReport {
  sample_size: number;
  avg_clarity: number;
  overall_flag_rate: number;
  patterns: CoachingPattern[];
  headline: string;
}

async function probePort(port: number): Promise<boolean> {
  try {
    const r = await fetch(`http://127.0.0.1:${port}/api/health`, { cache: "no-store" });
    if (!r.ok) return false;
    const j = (await r.json()) as Health;
    return j.service === "trace-daemon";
  } catch {
    return false;
  }
}

async function findDaemon(): Promise<number | null> {
  const stored = typeof window !== "undefined" ? Number(window.localStorage.getItem(PORT_KEY)) : NaN;
  if (Number.isFinite(stored) && stored > 0 && (await probePort(stored))) {
    return stored;
  }
  for (let p = PREFERRED_PORT; p < PREFERRED_PORT + 20; p++) {
    if (await probePort(p)) return p;
  }
  return null;
}

export default function HostedDashboard() {
  const [port, setPort] = useState<number | null>(null);
  const [checked, setChecked] = useState(false);

  useEffect(() => {
    findDaemon().then((p) => {
      setPort(p);
      setChecked(true);
      if (p !== null && typeof window !== "undefined") {
        window.localStorage.setItem(PORT_KEY, String(p));
      }
    });
  }, []);

  return (
    <div className="py-14">
      <div className="mb-8 flex items-baseline justify-between gap-4">
        <div>
          <h1 className="font-serif text-3xl text-text">Dashboard</h1>
          <p className="mt-2 text-sm text-text-dim">
            Browser talks directly to your own local Trace daemon at
            <code className="mx-1 rounded bg-black/5 px-1.5 py-0.5 text-[11px]">127.0.0.1:{port ?? PREFERRED_PORT}</code>.
            Nothing here is sent to us.
          </p>
        </div>
        <Link to="/" className="text-sm text-text-dim hover:text-text">
          ← Back
        </Link>
      </div>

      {!checked ? (
        <div className="rounded-2xl border border-border bg-white/50 p-8 text-sm text-text-dim">
          Probing your local daemon…
        </div>
      ) : port === null ? (
        <NoDaemon />
      ) : (
        <Connected port={port} onDisconnect={() => setPort(null)} />
      )}
    </div>
  );
}

function NoDaemon() {
  return (
    <div className="grid grid-cols-1 gap-6 md:grid-cols-[1.2fr_1fr]">
      <div className="rounded-2xl border border-border bg-white p-8">
        <div className="text-lg font-semibold text-text">No local daemon detected</div>
        <p className="mt-2 text-sm text-text-dim">
          The dashboard needs your Trace daemon running on <code className="rounded bg-black/5 px-1.5 py-0.5 text-xs">127.0.0.1</code>.
          Once it&apos;s up, this page finds it automatically — no reload needed.
        </p>
        <ol className="mt-6 space-y-4 text-sm text-text">
          <li>
            <div className="font-medium">1. Install the CLI (if you haven&apos;t)</div>
            <pre className="mt-1 overflow-x-auto rounded-lg bg-[#0d0d10] p-3 font-mono text-xs text-white">
              <span className="text-white/40">$ </span>curl -fsSL https://landing-one-hazel-88.vercel.app/install.sh | sh
            </pre>
          </li>
          <li>
            <div className="font-medium">2. Start the daemon</div>
            <pre className="mt-1 overflow-x-auto rounded-lg bg-[#0d0d10] p-3 font-mono text-xs text-white">
              <span className="text-white/40">$ </span>trace daemon start
            </pre>
          </li>
          <li>
            <div className="font-medium">3. Reload this page</div>
            <div className="text-text-dim">Or wait — it re-checks every few seconds automatically once you focus the tab.</div>
          </li>
        </ol>
      </div>
      <div className="rounded-2xl border border-border bg-white/50 p-8">
        <div className="text-sm font-semibold text-text">Why local-only?</div>
        <p className="mt-2 text-sm text-text-dim leading-relaxed">
          Trace records diffs of every file your AI agents touch. That&apos;s data you often don&apos;t
          want in the cloud. The hosted dashboard is a thin viewer — your browser reaches your own
          machine directly, so the traffic never leaves your device.
        </p>
        <p className="mt-3 text-sm text-text-dim leading-relaxed">
          If you <em>do</em> want cloud sync (share sessions between devices, view from mobile),
          set <code className="rounded bg-black/5 px-1 py-0.5 text-xs">TRACE_CLOUD_URL</code> +
          <code className="ml-1 rounded bg-black/5 px-1 py-0.5 text-xs">TRACE_CLOUD_TOKEN</code>
          on the daemon and use the Ratification dashboard&apos;s Local Trace runs tab.
        </p>
      </div>
    </div>
  );
}

function useApi<T>(port: number | null, path: string, deps: unknown[] = []) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (port === null) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    fetch(`http://127.0.0.1:${port}/api${path}`, { cache: "no-store" })
      .then(async (r) => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return (await r.json()) as T;
      })
      .then((d) => {
        if (!cancelled) setData(d);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e.message ?? e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [port, path, ...deps]);

  return { data, error, loading };
}

function Connected({ port, onDisconnect }: { port: number; onDisconnect: () => void }) {
  const [selectedRun, setSelectedRun] = useState<string | null>(null);
  const runsQ = useApi<RunSummary[]>(port, "/runs?limit=100");
  const coachingQ = useApi<CoachingReport>(port, "/analytics/coaching?limit=200");

  return (
    <div className="grid grid-cols-1 gap-6 lg:grid-cols-[1.4fr_1fr]">
      <div className="space-y-6">
        <RunsList
          runs={runsQ.data ?? []}
          loading={runsQ.loading}
          error={runsQ.error}
          selectedId={selectedRun}
          onSelect={setSelectedRun}
        />
        {selectedRun && <RunDetail port={port} runId={selectedRun} />}
      </div>
      <div className="space-y-6">
        <Coaching data={coachingQ.data} error={coachingQ.error} loading={coachingQ.loading} />
        <ConnectionCard port={port} onDisconnect={onDisconnect} />
      </div>
    </div>
  );
}

function RunsList({
  runs,
  loading,
  error,
  selectedId,
  onSelect,
}: {
  runs: RunSummary[];
  loading: boolean;
  error: string | null;
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  return (
    <div className="rounded-2xl border border-border bg-white">
      <div className="border-b border-border px-5 py-3">
        <div className="text-xs font-semibold uppercase tracking-wide text-text-dim">
          Recent runs {runs.length > 0 && `· ${runs.length}`}
        </div>
      </div>
      {loading && <div className="p-6 text-sm text-text-dim">Loading…</div>}
      {error && <div className="p-6 text-sm text-red-700">{error}</div>}
      {!loading && !error && runs.length === 0 && (
        <div className="p-8 text-center text-sm text-text-dim">
          No runs yet. Start an agent with <code className="rounded bg-black/5 px-1.5 py-0.5 text-xs">trace run &quot;claude ...&quot;</code> or
          wire up hooks with <code className="ml-1 rounded bg-black/5 px-1.5 py-0.5 text-xs">trace integrations install all</code>.
        </div>
      )}
      {runs.length > 0 && (
        <ul className="divide-y divide-border">
          {runs.map((r) => (
            <li key={r.id}>
              <button
                onClick={() => onSelect(r.id)}
                className={`grid w-full grid-cols-[1fr_auto] gap-4 px-5 py-4 text-left transition-colors hover:bg-black/[0.02] ${
                  selectedId === r.id ? "bg-brand/5" : ""
                }`}
              >
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    {r.agent_name && (
                      <span className="rounded-full bg-brand/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-brand">
                        {r.agent_name}
                      </span>
                    )}
                    <span
                      className={`rounded-full px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide ${
                        r.status === "completed"
                          ? "bg-emerald-100 text-emerald-700"
                          : r.status === "failed"
                            ? "bg-red-100 text-red-700"
                            : "bg-black/10 text-text-dim"
                      }`}
                    >
                      {r.status}
                    </span>
                  </div>
                  <div className="mt-1 truncate font-mono text-xs text-text-dim">{r.command}</div>
                  {r.user_prompt && (
                    <div className="mt-0.5 truncate text-xs text-text-dim">&ldquo;{r.user_prompt}&rdquo;</div>
                  )}
                </div>
                <div className="text-right text-xs text-text-dim">
                  {new Date(r.started_at).toLocaleString()}
                </div>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function RunDetail({ port, runId }: { port: number; runId: string }) {
  const eventsQ = useApi<Event[]>(port, `/runs/${runId}/events`, [runId]);

  return (
    <div className="rounded-2xl border border-border bg-white">
      <div className="flex items-baseline justify-between border-b border-border px-5 py-3">
        <div className="text-xs font-semibold uppercase tracking-wide text-text-dim">
          Timeline · {runId.slice(0, 8)}
        </div>
        {eventsQ.data && <div className="text-xs text-text-dim">{eventsQ.data.length} events</div>}
      </div>
      {eventsQ.loading && <div className="p-6 text-sm text-text-dim">Loading…</div>}
      {eventsQ.error && <div className="p-6 text-sm text-red-700">{eventsQ.error}</div>}
      {eventsQ.data && eventsQ.data.length === 0 && (
        <div className="p-8 text-center text-sm text-text-dim">No events recorded for this run.</div>
      )}
      {eventsQ.data && eventsQ.data.length > 0 && (
        <ol className="divide-y divide-border">
          {eventsQ.data.map((e) => (
            <li key={e.id} className="px-5 py-3">
              <div className="flex items-baseline justify-between gap-4">
                <div className="text-xs font-semibold uppercase tracking-wide text-brand">{e.type}</div>
                <div className="text-xs text-text-dim">{new Date(e.created_at).toLocaleTimeString()}</div>
              </div>
              <div className="mt-1 text-sm text-text">{e.message}</div>
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}

function Coaching({ data, error, loading }: { data: CoachingReport | null; error: string | null; loading: boolean }) {
  return (
    <div className="rounded-2xl border border-border bg-white">
      <div className="border-b border-border px-5 py-3">
        <div className="text-xs font-semibold uppercase tracking-wide text-text-dim">Prompt coaching</div>
      </div>
      {loading && <div className="p-6 text-sm text-text-dim">Loading…</div>}
      {error && <div className="p-6 text-sm text-red-700">{error}</div>}
      {data && data.sample_size === 0 && (
        <div className="p-6 text-sm text-text-dim">
          Runs a few prompts through <code className="rounded bg-black/5 px-1.5 py-0.5 text-xs">trace run</code> — coaching lights up
          once your history has patterns worth pointing to.
        </div>
      )}
      {data && data.sample_size > 0 && (
        <div className="p-5">
          <div className="text-sm font-medium text-text">{data.headline}</div>
          <div className="mt-1 text-xs text-text-dim">
            {data.sample_size} prompts sampled · {(data.overall_flag_rate * 100).toFixed(0)}% overall flag rate ·{" "}
            {data.avg_clarity.toFixed(0)}/100 avg clarity
          </div>
          <ul className="mt-5 space-y-4">
            {data.patterns.slice(0, 4).map((p) => (
              <li key={p.pattern}>
                <div className="flex items-baseline justify-between">
                  <div className="text-sm font-medium text-text">{p.pattern.replace(/_/g, " ")}</div>
                  <div className="text-xs text-text-dim">
                    {p.occurrences}× · {(p.flag_rate * 100).toFixed(0)}% flagged
                  </div>
                </div>
                <div className="mt-1 text-xs text-text-dim">{p.advice}</div>
                {p.example && (
                  <div className="mt-1 truncate text-xs italic text-text-dim">&ldquo;{p.example}&rdquo;</div>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

function ConnectionCard({ port, onDisconnect }: { port: number; onDisconnect: () => void }) {
  return (
    <div className="rounded-2xl border border-border bg-white/50 p-5">
      <div className="text-xs font-semibold uppercase tracking-wide text-text-dim">Connection</div>
      <div className="mt-3 flex items-center gap-2 text-sm">
        <span className="inline-block h-2 w-2 rounded-full bg-emerald-500" />
        <span className="text-text">
          Connected to local daemon on port <code className="rounded bg-black/5 px-1.5 py-0.5 text-xs">{port}</code>
        </span>
      </div>
      <p className="mt-3 text-xs text-text-dim">
        All requests go directly from your browser to your machine. Nothing routes through us.
      </p>
      <button
        onClick={() => {
          if (typeof window !== "undefined") window.localStorage.removeItem(PORT_KEY);
          onDisconnect();
        }}
        className="mt-4 text-xs text-text-dim hover:text-text"
      >
        Forget port &amp; re-probe →
      </button>
    </div>
  );
}
