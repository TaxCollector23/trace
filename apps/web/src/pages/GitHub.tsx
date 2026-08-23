import { useState } from "react";
import type { Project } from "../api";
import { api } from "../api";
import { Loading, stagger, useAsync } from "../components";

export default function GitHub() {
  const projectsQ = useAsync(() => api.dashboard());
  const projects: Project[] = projectsQ.data?.projects ?? [];
  const [projectId, setProjectId] = useState<string>("");
  const current = projectId || projects[0]?.id;

  const statusQ = useAsync(
    () => (current ? api.githubStatus(current) : Promise.resolve(null)),
    [current]
  );
  const commitsQ = useAsync(
    () => (current ? api.githubCommits(current, 15) : Promise.resolve([])),
    [current]
  );
  const pullsQ = useAsync(
    () => (current ? api.githubPulls(current) : Promise.resolve([])),
    [current]
  );

  const [filePath, setFilePath] = useState("README.md");
  const [fileContent, setFileContent] = useState<string | null>(null);
  const [fileErr, setFileErr] = useState<string | null>(null);
  const [fileBusy, setFileBusy] = useState(false);

  // Switching projects must drop the previously-read file — otherwise repo A's
  // file contents linger on screen while the header shows repo B.
  function changeProject(id: string) {
    setProjectId(id);
    setFileContent(null);
    setFileErr(null);
  }

  async function readFile() {
    if (!current) return;
    setFileBusy(true);
    setFileErr(null);
    try {
      const res = await api.githubFile(current, filePath);
      setFileContent(res.content);
    } catch (e) {
      setFileErr(String(e));
      setFileContent(null);
    } finally {
      setFileBusy(false);
    }
  }

  const status = statusQ.data;

  return (
    <div>
      <h1 className="page-title">GitHub</h1>
      <p className="page-sub">
        Trace reads directly from your project's GitHub repository —
        including private repos — using a token from the environment, the{" "}
        <span className="mono">gh</span> CLI, or{" "}
        <span className="mono">~/.trace/github.json</span>. Read-only; the
        token only goes to api.github.com.
      </p>

      {projectsQ.loading ? (
        <Loading error={projectsQ.error} variant="cards" rows={1} />
      ) : projects.length === 0 ? (
        <div className="empty">No projects yet. Run `trc init` in a repo.</div>
      ) : (
        <>
          {projects.length > 1 && (
            <div className="run-picker">
              <label className="muted" style={{ marginRight: 8 }}>
                Project:
              </label>
              <select
                aria-label="Project"
                value={current}
                onChange={(e) => changeProject(e.target.value)}
              >
                {projects.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </div>
          )}

          {/* Status */}
          {statusQ.loading ? (
            <Loading error={statusQ.error} variant="cards" rows={1} onRetry={statusQ.reload} />
          ) : statusQ.error ? (
            <div className="empty" role="alert">
              Couldn't reach the daemon for GitHub status: {statusQ.error}
              <div style={{ marginTop: 12 }}>
                <button className="btn" onClick={statusQ.reload}>Retry</button>
              </div>
            </div>
          ) : status ? (
            <div className="card">
              <div className="run-meta" style={{ marginTop: 0 }}>
                <span>
                  auth{" "}
                  <b style={{ color: status.authenticated ? "var(--green)" : "var(--red)" }}>
                    {status.authenticated ? "connected" : "not connected"}
                  </b>
                </span>
                <span>token: <b>{status.token_source}</b></span>
                {status.login && <span>user: <b>{status.login}</b></span>}
                {status.repo_ref && (
                  <span>
                    repo: <b>{status.repo_ref.owner}/{status.repo_ref.repo}</b>
                  </span>
                )}
                {status.repo && (
                  <span>
                    <b>{status.repo.private ? "private" : "public"}</b> · ★{" "}
                    {status.repo.stargazers_count} · {status.repo.open_issues_count} issues
                  </span>
                )}
              </div>
              {!status.repo_ref && (
                <div className="muted" style={{ marginTop: 8 }}>
                  This project has no GitHub <span className="mono">origin</span> remote.
                </div>
              )}
              {status.error && (
                <div className="note warn-note" style={{ marginTop: 8 }}>
                  {status.error} — set <span className="mono">GITHUB_TOKEN</span> or run{" "}
                  <span className="mono">gh auth login</span> to read private repos.
                </div>
              )}
            </div>
          ) : null}

          {/* Commits */}
          <div className="section-title">Recent commits</div>
          {commitsQ.loading ? (
            <Loading error={commitsQ.error} variant="table" rows={4} />
          ) : (commitsQ.data ?? []).length === 0 ? (
            <div className="empty">No commits (or not authorized).</div>
          ) : (
            <table>
              <thead>
                <tr>
                  <th>SHA</th>
                  <th>Message</th>
                  <th>Author</th>
                </tr>
              </thead>
              <tbody>
                {(commitsQ.data ?? []).map((c, i) => (
                  <tr key={c.sha} className="enter" style={stagger(i, 20, 160)}>
                    <td className="mono">{c.sha}</td>
                    <td>{c.message}</td>
                    <td className="muted">{c.author}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}

          {/* Pull requests */}
          <div className="section-title">Open pull requests</div>
          {pullsQ.loading ? (
            <Loading error={pullsQ.error} variant="table" rows={2} />
          ) : (pullsQ.data ?? []).length === 0 ? (
            <div className="empty">No open pull requests.</div>
          ) : (
            <table>
              <thead>
                <tr>
                  <th>PR</th>
                  <th>Title</th>
                  <th>Author</th>
                </tr>
              </thead>
              <tbody>
                {(pullsQ.data ?? []).map((p, i) => (
                  <tr key={p.number} className="enter" style={stagger(i, 20, 160)}>
                    <td className="mono">#{p.number}</td>
                    <td>
                      <a href={p.html_url} target="_blank" rel="noreferrer">
                        {p.title}
                      </a>
                    </td>
                    <td className="muted">@{p.user}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}

          {/* File reader */}
          <div className="section-title">Read a file from the repo</div>
          <form
            className="btn-row"
            onSubmit={(e) => {
              e.preventDefault();
              if (current && status?.repo_ref) readFile();
            }}
          >
            <input
              className="num"
              style={{ width: 320 }}
              aria-label="File path in repo"
              value={filePath}
              onChange={(e) => setFilePath(e.target.value)}
              placeholder="path/in/repo.ts"
            />
            <button
              className="btn"
              type="submit"
              disabled={fileBusy || !current || !status?.repo_ref}
            >
              {fileBusy ? "Reading…" : "Read file"}
            </button>
          </form>
          {fileErr && (
            <div className="empty" role="alert">
              Error: {fileErr}
            </div>
          )}
          {fileContent !== null && (
            <pre className="diff" style={{ whiteSpace: "pre" }}>
              {fileContent}
            </pre>
          )}
        </>
      )}
    </div>
  );
}
