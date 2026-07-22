import { useParams } from "react-router-dom";
import { api } from "../api";
import { Loading, RunPicker, fmtCost, fmtNum, stagger, useAsync } from "../components";

export default function CostCenter() {
  const { runId } = useParams();
  const runsQ = useAsync(() => api.runs());
  const runs = runsQ.data ?? [];
  const current = runId ?? runs[0]?.id;

  const costQ = useAsync(
    () => (current ? api.cost(current) : Promise.resolve(null)),
    [current]
  );
  const cost = costQ.data;

  return (
    <div>
      <h1 className="page-title">Token Spend</h1>
      <p className="page-sub">
        AI/API usage and estimated cost. Partial data is shown honestly — cost is
        labelled "unavailable" when it cannot be computed.
      </p>

      {runsQ.loading ? (
        <Loading error={runsQ.error} variant="cards" rows={1} />
      ) : runs.length === 0 ? (
        <div className="empty">No runs recorded yet.</div>
      ) : (
        <>
          <RunPicker runs={runs} current={current} base="/cost" />

          {costQ.loading ? (
            <Loading error={costQ.error} variant="table" rows={3} />
          ) : !cost || cost.usage.length === 0 ? (
            <div className="empty">
              Cost data appears when the agent reports usage or traffic flows
              through a Trace adapter.
            </div>
          ) : (
            <>
              <div className="note">
                Total estimated cost: <b>{fmtCost(cost.total_estimated)}</b>
                {cost.has_unavailable &&
                  " (some entries have unavailable cost and are excluded)"}
              </div>
              <table>
                <thead>
                  <tr>
                    <th>Provider</th>
                    <th>Model</th>
                    <th>Input</th>
                    <th>Output</th>
                    <th>Cached</th>
                    <th>Latency</th>
                    <th>Est. cost</th>
                  </tr>
                </thead>
                <tbody>
                  {cost.usage.map((u, i) => (
                    <tr key={u.id} className="enter" style={stagger(i, 20, 160)}>
                      <td>{u.provider}</td>
                      <td className="mono">{u.model}</td>
                      <td>{fmtNum(u.input_tokens)}</td>
                      <td>{fmtNum(u.output_tokens)}</td>
                      <td>{fmtNum(u.cached_tokens)}</td>
                      <td>{u.latency_ms ? `${u.latency_ms} ms` : "—"}</td>
                      <td>{fmtCost(u.estimated_cost)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </>
          )}
        </>
      )}
    </div>
  );
}
