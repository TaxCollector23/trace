//! Read directly from a GitHub repository — including **private** repos — using
//! the GitHub REST API.
//!
//! Authentication token resolution, in order:
//! 1. `GITHUB_TOKEN` / `GH_TOKEN` environment variables.
//! 2. `gh auth token` (the GitHub CLI's stored credential).
//! 3. `~/.trace/github.json` → `{ "token": "..." }`.
//!
//! This is read-only. Trace never pushes, and a token is only ever sent to
//! `api.github.com` over HTTPS — never to Trace's own surfaces.

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

const API: &str = "https://api.github.com";
const UA: &str = "trace";

/// Where the active token came from (for display; never the token itself).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TokenSource {
    Env,
    GhCli,
    ConfigFile,
    None,
}

impl TokenSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenSource::Env => "env",
            TokenSource::GhCli => "gh_cli",
            TokenSource::ConfigFile => "config_file",
            TokenSource::None => "none",
        }
    }
}

/// Resolve a GitHub token and its source.
pub fn resolve_token() -> (Option<String>, TokenSource) {
    for var in ["GITHUB_TOKEN", "GH_TOKEN"] {
        if let Ok(v) = std::env::var(var) {
            if !v.trim().is_empty() {
                return (Some(v.trim().to_string()), TokenSource::Env);
            }
        }
    }
    if let Ok(out) = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
    {
        if out.status.success() {
            let tok = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !tok.is_empty() {
                return (Some(tok), TokenSource::GhCli);
            }
        }
    }
    if let Some(tok) = token_from_config() {
        return (Some(tok), TokenSource::ConfigFile);
    }
    (None, TokenSource::None)
}

fn token_from_config() -> Option<String> {
    let path = crate::paths::global_dir().ok()?.join("github.json");
    let raw = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    v.get("token")?.as_str().map(|s| s.to_string())
}

/// Owner/repo pair parsed from a git remote URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoRef {
    pub owner: String,
    pub repo: String,
}

/// Parse `owner/repo` from common GitHub remote URL forms (https, ssh, git).
pub fn parse_remote(url: &str) -> Option<RepoRef> {
    let u = url.trim();
    let rest = if let Some(r) = u.strip_prefix("git@github.com:") {
        r.to_string()
    } else if let Some(r) = u.strip_prefix("ssh://git@github.com/") {
        r.to_string()
    } else if let Some(r) = u.strip_prefix("https://github.com/") {
        r.to_string()
    } else if let Some(r) = u.strip_prefix("http://github.com/") {
        r.to_string()
    } else {
        host_anchored_github_path(u)?
    };
    let rest = rest.trim_end_matches('/').trim_end_matches(".git");
    let mut parts = rest.splitn(2, '/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(RepoRef { owner, repo })
}

/// Fallback for URL forms not covered by the explicit prefixes (embedded
/// credentials, `git://`, etc.). Matches `github.com` only at a real host
/// boundary: preceded by start-of-string, the scheme's `//`, or userinfo `@`,
/// and followed by `/` (path) or `:` (scp-style). This rejects lookalike hosts
/// such as `notgithub.com` and `mygithub.company.com` that a plain substring
/// search would wrongly accept.
fn host_anchored_github_path(u: &str) -> Option<String> {
    static HOST: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:^|//|@)github\.com[:/](.+)$").unwrap());
    HOST.captures(u).map(|c| c[1].to_string())
}

fn build_req(url: &str, token: Option<&str>) -> ureq::Request {
    let mut req = ureq::get(url)
        .set("User-Agent", UA)
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28");
    if let Some(t) = token {
        req = req.set("Authorization", &format!("Bearer {t}"));
    }
    req
}

/// Turn a non-2xx GitHub response into an actionable error: distinguish
/// auth/permission failures (incl. private-repo 404s) and rate limiting from a
/// generic API error, so the CLI/dashboard can tell the user what to actually do.
fn status_error(path: &str, code: u16, resp: ureq::Response) -> anyhow::Error {
    let rate_remaining = resp
        .header("x-ratelimit-remaining")
        .and_then(|v| v.parse::<i64>().ok());
    let body = resp.into_string().unwrap_or_default();
    let snippet = body.chars().take(200).collect::<String>();
    match code {
        401 | 403 if rate_remaining == Some(0) => anyhow!(
            "GitHub rate limit exceeded for {path}. Set a token (GITHUB_TOKEN, `gh auth login`, \
             or ~/.trace/github.json) to raise the limit, or wait for it to reset."
        ),
        401 | 403 => anyhow!(
            "GitHub denied access ({code}) for {path}. Check your token (GITHUB_TOKEN, \
             `gh auth login`, or ~/.trace/github.json) and that it can read this repo."
        ),
        404 => anyhow!(
            "GitHub returned 404 for {path}. If this is a private repo, the token may be \
             missing or lack access (GitHub returns 404, not 403, to hide private repos)."
        ),
        _ => anyhow!("GitHub API {code} for {path}: {snippet}"),
    }
}

fn get_json(path: &str, token: Option<&str>) -> Result<serde_json::Value> {
    let url = format!("{API}{path}");
    match build_req(&url, token).call() {
        Ok(resp) => resp.into_json().context("decoding GitHub response"),
        Err(ureq::Error::Status(code, resp)) => Err(status_error(path, code, resp)),
        Err(e) => Err(anyhow!("GitHub request failed: {e}")),
    }
}

/// Parse the `rel="next"` URL from a GitHub `Link` header, if present.
fn next_link(header: Option<&str>) -> Option<String> {
    let header = header?;
    header.split(',').find_map(|part| {
        if !part.contains("rel=\"next\"") {
            return None;
        }
        let start = part.find('<')?;
        let end = part.find('>')?;
        (end > start).then(|| part[start + 1..end].to_string())
    })
}

/// GET an array endpoint, following `Link: rel="next"` across pages until the
/// endpoint is exhausted or `max_items` is reached (a safety bound against a
/// runaway PR). Returns the concatenated array elements.
///
/// This closes a real correctness hole: a single-page fetch of a pull request's
/// files capped review at 100 files, so a high-severity change (e.g. a
/// committed secret) in file #101 produced a clean `pass` verdict.
fn get_json_array_paged(
    first_path: &str,
    token: Option<&str>,
    max_items: usize,
) -> Result<Vec<serde_json::Value>> {
    let mut out: Vec<serde_json::Value> = Vec::new();
    let mut url = format!("{API}{first_path}");
    loop {
        let resp = match build_req(&url, token).call() {
            Ok(r) => r,
            Err(ureq::Error::Status(code, resp)) => {
                return Err(status_error(first_path, code, resp))
            }
            Err(e) => return Err(anyhow!("GitHub request failed: {e}")),
        };
        let next = next_link(resp.header("link"));
        let page: serde_json::Value = resp.into_json().context("decoding GitHub response")?;
        if let Some(arr) = page.as_array() {
            out.extend(arr.iter().cloned());
        }
        if out.len() >= max_items {
            out.truncate(max_items);
            break;
        }
        match next {
            Some(u) => url = u,
            None => break,
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthedUser {
    pub login: String,
}

/// The authenticated user for the current token, if any.
pub fn whoami(token: &str) -> Result<AuthedUser> {
    let v = get_json("/user", Some(token))?;
    Ok(AuthedUser {
        login: v
            .get("login")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    pub full_name: String,
    pub private: bool,
    pub default_branch: String,
    pub description: Option<String>,
    pub html_url: String,
    pub stargazers_count: i64,
    pub open_issues_count: i64,
}

pub fn get_repo(r: &RepoRef, token: Option<&str>) -> Result<RepoInfo> {
    let v = get_json(&format!("/repos/{}/{}", r.owner, r.repo), token)?;
    Ok(RepoInfo {
        full_name: v["full_name"].as_str().unwrap_or("").to_string(),
        private: v["private"].as_bool().unwrap_or(false),
        default_branch: v["default_branch"].as_str().unwrap_or("main").to_string(),
        description: v["description"].as_str().map(|s| s.to_string()),
        html_url: v["html_url"].as_str().unwrap_or("").to_string(),
        stargazers_count: v["stargazers_count"].as_i64().unwrap_or(0),
        open_issues_count: v["open_issues_count"].as_i64().unwrap_or(0),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

pub fn list_commits(r: &RepoRef, token: Option<&str>, limit: usize) -> Result<Vec<CommitInfo>> {
    let v = get_json(
        &format!(
            "/repos/{}/{}/commits?per_page={}",
            r.owner,
            r.repo,
            limit.min(100)
        ),
        token,
    )?;
    let arr = v.as_array().cloned().unwrap_or_default();
    Ok(arr
        .iter()
        .map(|c| CommitInfo {
            sha: c["sha"].as_str().unwrap_or("").chars().take(10).collect(),
            message: c["commit"]["message"]
                .as_str()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_string(),
            author: c["commit"]["author"]["name"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            date: c["commit"]["author"]["date"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullInfo {
    pub number: i64,
    pub title: String,
    pub state: String,
    pub user: String,
    pub html_url: String,
}

pub fn list_pulls(r: &RepoRef, token: Option<&str>) -> Result<Vec<PullInfo>> {
    let v = get_json(
        &format!("/repos/{}/{}/pulls?state=open&per_page=30", r.owner, r.repo),
        token,
    )?;
    let arr = v.as_array().cloned().unwrap_or_default();
    Ok(arr
        .iter()
        .map(|p| PullInfo {
            number: p["number"].as_i64().unwrap_or(0),
            title: p["title"].as_str().unwrap_or("").to_string(),
            state: p["state"].as_str().unwrap_or("").to_string(),
            user: p["user"]["login"].as_str().unwrap_or("").to_string(),
            html_url: p["html_url"].as_str().unwrap_or("").to_string(),
        })
        .collect())
}

/// Recently *merged* pull requests, most-recently-updated first — the raw
/// material for doctrine mining. Filters out PRs that were closed without
/// merging (those don't reflect enforced doctrine, just abandoned work).
pub fn list_recent_merged_pulls(
    r: &RepoRef,
    token: Option<&str>,
    limit: usize,
) -> Result<Vec<PullInfo>> {
    let v = get_json(
        &format!(
            "/repos/{}/{}/pulls?state=closed&sort=updated&direction=desc&per_page={}",
            r.owner,
            r.repo,
            (limit * 2).min(100) // over-fetch since some closed PRs aren't merged
        ),
        token,
    )?;
    let arr = v.as_array().cloned().unwrap_or_default();
    Ok(arr
        .iter()
        .filter(|p| !p["merged_at"].is_null())
        .take(limit)
        .map(|p| PullInfo {
            number: p["number"].as_i64().unwrap_or(0),
            title: p["title"].as_str().unwrap_or("").to_string(),
            state: p["state"].as_str().unwrap_or("").to_string(),
            user: p["user"]["login"].as_str().unwrap_or("").to_string(),
            html_url: p["html_url"].as_str().unwrap_or("").to_string(),
        })
        .collect())
}

/// A single review or issue comment on a pull request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrComment {
    pub author: String,
    pub body: String,
}

fn parse_comments(v: &serde_json::Value) -> Vec<PrComment> {
    v.as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|c| {
            let body = c["body"].as_str()?.trim();
            if body.is_empty() {
                return None;
            }
            Some(PrComment {
                author: c["user"]["login"].as_str().unwrap_or("unknown").to_string(),
                body: body.to_string(),
            })
        })
        .collect()
}

/// Inline code-review comments on a pull request (the ones attached to a
/// specific diff line) — usually the highest-signal source for doctrine.
pub fn list_pr_review_comments(
    r: &RepoRef,
    token: Option<&str>,
    pr_number: i64,
) -> Result<Vec<PrComment>> {
    let v = get_json(
        &format!(
            "/repos/{}/{}/pulls/{}/comments?per_page=100",
            r.owner, r.repo, pr_number
        ),
        token,
    )?;
    Ok(parse_comments(&v))
}

/// Top-level conversation comments on a pull request (the PR is an issue too).
pub fn list_pr_issue_comments(
    r: &RepoRef,
    token: Option<&str>,
    pr_number: i64,
) -> Result<Vec<PrComment>> {
    let v = get_json(
        &format!(
            "/repos/{}/{}/issues/{}/comments?per_page=100",
            r.owner, r.repo, pr_number
        ),
        token,
    )?;
    Ok(parse_comments(&v))
}

/// One changed file in a pull request, in the same shape `policy.rs` expects.
/// GitHub caps the PR-files endpoint at 3000 files total; we page through all
/// of them so the policy engine never reviews only a truncated prefix.
const MAX_PR_FILES: usize = 3000;

pub fn list_pr_files(
    r: &RepoRef,
    token: Option<&str>,
    pr_number: i64,
) -> Result<Vec<crate::policy::FileDiff>> {
    let files = get_json_array_paged(
        &format!(
            "/repos/{}/{}/pulls/{}/files?per_page=100",
            r.owner, r.repo, pr_number
        ),
        token,
        MAX_PR_FILES,
    )?;
    // Resolved at most once, only if we actually hit an added file with no
    // patch. `None` = unresolved yet; `Some(x)` = resolved (x may itself be
    // `None` if the lookup failed, so we don't retry it per-file).
    let mut head_ref: Option<Option<String>> = None;
    let mut out = Vec::with_capacity(files.len());
    for f in &files {
        let filename = f["filename"].as_str().unwrap_or("").to_string();
        let status = f["status"].as_str().unwrap_or("modified").to_string();
        let additions = f["additions"].as_i64().unwrap_or(0);
        let deletions = f["deletions"].as_i64().unwrap_or(0);
        let mut patch = f["patch"].as_str().map(|s| s.to_string());

        // A newly ADDED text file that GitHub returned without a patch (binary
        // heuristic, or simply too large for the files endpoint to inline)
        // would otherwise be skipped by every content check — including the
        // secret scanner. Fetch its content at the PR head and synthesize an
        // all-added patch so the policy engine still scans it. Bounded: binary
        // or very large content is skipped rather than scanned.
        if patch.is_none() && status == "added" && !filename.is_empty() {
            let hr = head_ref.get_or_insert_with(|| pr_head_ref(r, token, pr_number).ok());
            if let Some(rf) = hr.as_deref() {
                if let Ok(content) = get_file(r, &filename, Some(rf), token) {
                    patch = synthesize_added_patch(&content);
                }
            }
        }

        out.push(crate::policy::FileDiff {
            filename,
            status,
            additions,
            deletions,
            patch,
        });
    }
    Ok(out)
}

/// Upper bound on synthesized content — a single added file larger than this is
/// skipped rather than pulled into memory and scanned line-by-line.
const MAX_SYNTH_BYTES: usize = 512 * 1024;

/// Turn a whole file's content into an all-added unified-diff patch so the
/// policy engine's added-line checks can run on it. Returns `None` when the
/// content looks binary or exceeds [`MAX_SYNTH_BYTES`].
fn synthesize_added_patch(content: &str) -> Option<String> {
    if content.len() > MAX_SYNTH_BYTES {
        return None;
    }
    // A NUL byte is a strong binary signal; `get_file` decodes lossily, so
    // genuinely binary payloads also carry U+FFFD replacement chars.
    if content.contains('\0') || content.contains('\u{FFFD}') {
        return None;
    }
    Some(content.lines().map(|l| format!("+{l}\n")).collect())
}

/// The head commit SHA for a pull request (what the added-file content should
/// be read at). One extra request, made only when needed.
fn pr_head_ref(r: &RepoRef, token: Option<&str>, pr_number: i64) -> Result<String> {
    let v = get_json(
        &format!("/repos/{}/{}/pulls/{}", r.owner, r.repo, pr_number),
        token,
    )?;
    v["head"]["sha"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("pull request {pr_number} response missing head.sha"))
}

/// Read a file's contents from the repo at an optional ref. Works for private
/// repos the token can access. Decodes the base64 the contents API returns.
pub fn get_file(
    r: &RepoRef,
    path: &str,
    git_ref: Option<&str>,
    token: Option<&str>,
) -> Result<String> {
    let mut url = format!("/repos/{}/{}/contents/{}", r.owner, r.repo, path);
    if let Some(rf) = git_ref {
        url.push_str(&format!("?ref={rf}"));
    }
    let v = get_json(&url, token)?;
    let encoding = v["encoding"].as_str().unwrap_or("");
    let content = v["content"].as_str().unwrap_or("");
    if encoding == "base64" {
        let cleaned: String = content.chars().filter(|c| !c.is_whitespace()).collect();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(cleaned)
            .context("decoding base64 file content")?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    } else {
        Ok(content.to_string())
    }
}

/// Full status for a project's GitHub connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubStatus {
    pub authenticated: bool,
    pub token_source: String,
    pub login: Option<String>,
    pub repo: Option<RepoInfo>,
    pub repo_ref: Option<RepoRef>,
    pub error: Option<String>,
}

/// Resolve auth + repo for a project directory (reads the git remote).
pub fn status_for_path(project_path: &std::path::Path) -> GithubStatus {
    let (token, source) = resolve_token();
    let token_ref = token.as_deref();

    let repo_ref = crate::git::remote_url(project_path).and_then(|u| parse_remote(&u));

    let login = token_ref.and_then(|t| whoami(t).ok().map(|u| u.login));

    let (repo, error) = match &repo_ref {
        Some(r) => match get_repo(r, token_ref) {
            Ok(info) => (Some(info), None),
            Err(e) => (None, Some(e.to_string())),
        },
        None => (None, None),
    };

    GithubStatus {
        authenticated: token.is_some() && login.is_some(),
        token_source: source.as_str().to_string(),
        login,
        repo,
        repo_ref,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_https_remote() {
        let r = parse_remote("https://github.com/TaxCollector23/trace.git").unwrap();
        assert_eq!(r.owner, "TaxCollector23");
        assert_eq!(r.repo, "trace");
    }

    #[test]
    fn parses_ssh_remote() {
        let r = parse_remote("git@github.com:owner/repo.git").unwrap();
        assert_eq!(r.owner, "owner");
        assert_eq!(r.repo, "repo");
    }

    #[test]
    fn parses_https_without_git_suffix() {
        let r = parse_remote("https://github.com/a/b").unwrap();
        assert_eq!(r.repo, "b");
    }

    #[test]
    fn rejects_non_github() {
        assert!(parse_remote("https://gitlab.com/a/b.git").is_none());
    }

    #[test]
    fn rejects_lookalike_hosts() {
        // The old substring fallback wrongly matched any host containing
        // "github.com/"; these must all be rejected.
        assert!(parse_remote("https://notgithub.com/a/b").is_none());
        assert!(parse_remote("https://notgithub.com/a/b.git").is_none());
        assert!(parse_remote("https://mygithub.company.com/a/b").is_none());
        assert!(parse_remote("https://github.com.evil.example/a/b").is_none());
    }

    #[test]
    fn parses_embedded_credentials_host() {
        // A real github.com host behind userinfo still parses via the anchored
        // fallback (not one of the explicit prefixes).
        let r = parse_remote("https://x-access-token:tok@github.com/owner/repo.git").unwrap();
        assert_eq!(r.owner, "owner");
        assert_eq!(r.repo, "repo");
    }

    #[test]
    fn next_link_extracts_rel_next() {
        let h = "<https://api.github.com/repositories/1/pulls/2/files?page=2>; rel=\"next\", \
                 <https://api.github.com/repositories/1/pulls/2/files?page=9>; rel=\"last\"";
        assert_eq!(
            next_link(Some(h)).as_deref(),
            Some("https://api.github.com/repositories/1/pulls/2/files?page=2")
        );
    }

    #[test]
    fn next_link_none_on_last_page() {
        // Only rel="prev"/rel="first" present (typical of the final page).
        let h = "<https://api.github.com/x?page=8>; rel=\"prev\", \
                 <https://api.github.com/x?page=1>; rel=\"first\"";
        assert_eq!(next_link(Some(h)), None);
        assert_eq!(next_link(None), None);
    }
}
