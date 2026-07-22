import { useState } from "react";
import type { Checkpoint, RunSummary } from "../api";
import { api } from "../api";
import { Loading, StatusBadge, fmtTime, useAsync } from "../components";
import Tracey from "../Tracey";

interface Row {
  run: RunSummary;
  checkpoint: Checkpoint;
}

export default function RollbackCenter() {
  const runsQ = useAsync(() => api.runs());
  const rowsQ = useAsync<Row[]>(async () => {
    const runs = runsQ.data ?? [];
    const rows: Row[] = [];
    for (const run of runs) {
      const cps = await api.checkpoints(run.id);
      const cp = [...cps].reverse().find((c) => c.git_ref);
      if (cp) rows.push({ run, checkpoint: cp });
    }
    return rows;
  }, [runsQ.data]);

  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [succeeded, setSucceeded] = useState(false);

  async function doRollback(run: RunSummary) {
    const ok = window.confirm(
      `Roll back the working tree to the checkpoint for:\n\n${run.command}\n\nThis is Git-based and will restore files. Continue?`
    );
    if (!ok) return;
    setBusy(run.id);
    setMessage(null);
    setSucceeded(false);
    try {
      const res = await api.rollback(run.id);
      setMessage(`Rollback completed (ref ${res.git_ref.slice(0, 10)}).`);
      setSucceeded(true);
      runsQ.reload();
      rowsQ.reload();
    } catch (e) {
      setMessage(`Rollback failed: ${e}`);
    } finally {
      setBusy(null);
    }
  }

  const rows = rowsQ.data ?? [];

  return (
    <div>
      <h1 className="page-title">Rollback Points</h1>
      <p className="page-sub">
        Restore your working tree to a checkpoint. Rollback is Git-based and asks
        for confirmation before changing files.
      </p>

      {message && (
        <div className="note" style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {succeeded && <Tracey size={22} expression="success" />}
          {message}
        </div>
      )}

      {runsQ.loading || rowsQ.loading ? (
        <Loading error={runsQ.error ?? rowsQ.error} />
      ) : rows.length === 0 ? (
        <div className="empty">
          Rollback requires Git and a checkpoint created before a monitored run.
          Checkpoints are created before
          each run in a Git repository.
        </div>
      ) : (
        <table>
          <thead>
            <tr>
              <th>Run</th>
              <th>Status</th>
              <th>Checkpoint</th>
              <th>Ref</th>
              <th>Created</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {rows.map(({ run, checkpoint }) => (
              <tr key={checkpoint.id}>
                <td className="mono">
                  {run.command.length > 40
                    ? run.command.slice(0, 40) + "…"
                    : run.command}
                </td>
                <td>
                  <StatusBadge status={run.status} />
                </td>
                <td>{checkpoint.checkpoint_type}</td>
                <td className="mono">{checkpoint.git_ref?.slice(0, 10)}</td>
                <td className="muted">{fmtTime(checkpoint.created_at)}</td>
                <td>
                  <button
                    className="btn danger"
                    disabled={busy === run.id}
                    onClick={() => doRollback(run)}
                  >
                    {busy === run.id ? "Rolling back…" : "Roll back"}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
