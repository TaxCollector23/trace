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
        let idx = u.find("github.com/")?;
        u[idx + "github.com/".len()..].to_string()
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

fn get_json(path: &str, token: Option<&str>) -> Result<serde_json::Value> {
    let url = format!("{API}{path}");
    let mut req = ureq::get(&url)
        .set("User-Agent", UA)
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28");
    if let Some(t) = token {
        req = req.set("Authorization", &format!("Bearer {t}"));
    }
    match req.call() {
        Ok(resp) => resp.into_json().context("decoding GitHub response"),
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            Err(anyhow!(
                "GitHub API {code} for {path}: {}",
                body.chars().take(200).collect::<String>()
            ))
        }
        Err(e) => Err(anyhow!("GitHub request failed: {e}")),
    }
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
}
