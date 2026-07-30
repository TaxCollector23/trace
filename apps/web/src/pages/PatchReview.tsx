import { useParams } from "react-router-dom";
import type { FileChange } from "../api";
import { api } from "../api";
import { DiffView, Loading, RunPicker, useAsync } from "../components";

const DEP_FILES = [
  "package.json", "package-lock.json", "pnpm-lock.yaml", "yarn.lock",
  "Cargo.toml", "Cargo.lock", "requirements.txt", "poetry.lock", "pyproject.toml",
  "go.mod", "go.sum",
];
const CONFIG_FILES = [
  "tsconfig.json", "vite.config", "webpack.config", "babel.config",
  ".eslintrc", "next.config", "rollup.config", "jest.config",
];

function baseName(p: string) {
  return p.split(/[\\/]/).pop() ?? p;
}
function isDep(p: string) {
  return DEP_FILES.includes(baseName(p));
}
function isConfig(p: string) {
  return CONFIG_FILES.some((c) => baseName(p).startsWith(c));
}
function isEnv(p: string) {
  const n = baseName(p);
  return n === ".env" || n.startsWith(".env.") || n === "id_rsa" || n.endsWith(".pem");
}

function group(changes: FileChange[], type: FileChange["change_type"]) {
  return changes.filter((c) => c.change_type === type);
}

export default function PatchReview() {
  const { runId } = useParams();
  const runsQ = useAsync(() => api.runs());
  const runs = runsQ.data ?? [];
  const current = runId ?? runs[0]?.id;

  const changesQ = useAsync(
    () => (current ? api.fileChanges(current) : Promise.resolve([])),
    [current]
  );
  const diffQ = useAsync(
    () => (current ? api.diff(current) : Promise.resolve({ diff: "" })),
    [current]
  );
  const changes = changesQ.data ?? [];

  const deps = changes.filter((c) => isDep(c.path));
  const configs = changes.filter((c) => isConfig(c.path));
  const envs = changes.filter((c) => isEnv(c.path));

  return (
    <div>
      <h1 className="page-title">Patch Review</h1>
      <p className="page-sub">
        Code changes for a run, derived from the actual Git diff.
      </p>

      {runsQ.loading ? (
        <Loading error={runsQ.error} />
      ) : runs.length === 0 ? (
        <div className="empty">No runs recorded yet.</div>
      ) : (
        <>
          <RunPicker runs={runs} current={current} base="/patch" />

          {changesQ.loading ? (
            <Loading error={changesQ.error} />
          ) : (
            <>
              <div className="note">{summary(changes, deps, configs, envs)}</div>

              <Section title="Files added" items={group(changes, "created")} cls="created" />
              <Section title="Files modified" items={group(changes, "modified")} cls="modified" />
              <Section title="Files deleted" items={group(changes, "deleted")} cls="deleted" />

              {deps.length > 0 && (
                <Section title="Dependency files changed" items={deps} cls="modified" />
              )}
              {configs.length > 0 && (
                <Section title="Config files changed" items={configs} cls="modified" />
              )}
              {envs.length > 0 && (
                <div className="note warn-note">
                  Environment files touched: {envs.map((e) => e.path).join(", ")}
                </div>
              )}

              <div className="section-title">Git diff</div>
              {diffQ.loading ? (
                <Loading error={diffQ.error} />
              ) : (
                <DiffView diff={diffQ.data?.diff ?? ""} />
              )}
            </>
          )}
        </>
      )}
    </div>
  );
}

function summary(
  all: FileChange[],
  deps: FileChange[],
  configs: FileChange[],
  envs: FileChange[]
): string {
  if (all.length === 0) return "No file changes were detected for this run.";
  const parts = [`${all.length} file(s) changed`];
  const notes: string[] = [];
  if (deps.length) notes.push("dependency files changed");
  if (configs.length) notes.push("build config changed");
  if (envs.length) notes.push("environment files touched");
  return notes.length ? `${parts}. Notable: ${notes.join("; ")}.` : `${parts}.`;
}

function Section({
  title,
  items,
  cls,
}: {
  title: string;
  items: FileChange[];
  cls: string;
}) {
  if (items.length === 0) return null;
  return (
    <div>
      <div className="section-title">{title}</div>
      <table>
        <tbody>
          {items.map((c) => (
            <tr key={c.id}>
              <td>
                <span className={`pill ${cls}`}>{c.change_type}</span>
              </td>
              <td className="mono">{c.path}</td>
              <td className="muted mono">{c.diff_summary ?? ""}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
