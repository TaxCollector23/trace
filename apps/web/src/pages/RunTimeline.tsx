import { useParams } from "react-router-dom";
import { api } from "../api";
import { Loading, RunPicker, StatusBadge, fmtTime, useAsync } from "../components";

export default function RunTimeline() {
  const { runId } = useParams();
  const runsQ = useAsync(() => api.runs());
  const runs = runsQ.data ?? [];
  const current = runId ?? runs[0]?.id;

  const eventsQ = useAsync(
    () => (current ? api.timeline(current) : Promise.resolve([])),
    [current]
  );

  const run = runs.find((r) => r.id === current);

  return (
    <div>
      <h1 className="page-title">Session Timeline</h1>
      <p className="page-sub">Chronological events recorded for a run.</p>

      {runsQ.loading ? (
        <Loading error={runsQ.error} />
      ) : runs.length === 0 ? (
        <div className="empty">No runs recorded yet.</div>
      ) : (
        <>
          <RunPicker runs={runs} current={current} base="/timeline" />
          {run && (
            <div className="card">
              <div className="run-head">
                <div className="run-cmd">{run.command}</div>
                <StatusBadge status={run.status} />
              </div>
              <div className="run-meta">
                <span>
                  <b>{run.project_name}</b>
                </span>
                <span>started {fmtTime(run.started_at)}</span>
                <span>exit {run.exit_code ?? "—"}</span>
              </div>
            </div>
          )}

          {eventsQ.loading ? (
            <Loading error={eventsQ.error} />
          ) : (eventsQ.data ?? []).length === 0 ? (
            <div className="empty">No events for this run.</div>
          ) : (
            <div className="timeline">
              {(eventsQ.data ?? []).map((e) => (
                <div className="tl-item" key={e.id}>
                  <div className="tl-time">{fmtTime(e.created_at)}</div>
                  <div className="tl-msg">{e.message}</div>
                  <div className="tl-type">{e.type}</div>
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}
