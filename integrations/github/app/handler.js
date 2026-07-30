// TraceGuard GitHub App — event handler skeleton.
//
// This is an intentionally minimal, dependency-free skeleton showing the event
// surface. Wire it to a webhook receiver (e.g. a small Express/Lambda) and the
// GitHub REST API client of your choice. It must only ever post SANITIZED
// summaries — never raw files, secrets, or the local SQLite database.

/**
 * @param {string} event   GitHub event name (e.g. "pull_request")
 * @param {object} payload Parsed webhook payload
 * @param {object} octokit Authenticated GitHub API client (caller-provided)
 */
export async function handleEvent(event, payload, octokit) {
  switch (event) {
    case "pull_request":
      if (["opened", "synchronize"].includes(payload.action)) {
        await createTraceGuardCheck(payload, octokit);
      }
      break;
    case "push":
      // Link the local TraceGuard run to the pushed commit SHA (out of band).
      break;
    default:
      break;
  }
}

async function createTraceGuardCheck(payload, octokit) {
  const { repository, pull_request } = payload;
  const summary = await loadSanitizedSummary(); // from the CI artifact or local import

  await octokit.checks.create({
    owner: repository.owner.login,
    repo: repository.name,
    name: "TraceGuard",
    head_sha: pull_request.head.sha,
    status: "completed",
    conclusion: summary.checks_status === "failed" ? "failure" : "neutral",
    output: {
      title: "TraceGuard summary",
      summary: [
        `Files changed: ${summary.files_changed}`,
        `Risky file warnings: ${summary.risky_file_warnings}`,
        `Secret-like findings: ${summary.secret_like_findings}`,
        `Checks: ${summary.checks_status}`,
      ].join("\n"),
    },
  });
}

// Replace with reading the uploaded `traceguard-summary.json` artifact.
async function loadSanitizedSummary() {
  return {
    files_changed: 0,
    risky_file_warnings: 0,
    secret_like_findings: 0,
    checks_status: "skipped",
  };
}
