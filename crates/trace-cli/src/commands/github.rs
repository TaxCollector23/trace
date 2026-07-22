//! `trace github` — read directly from the project's GitHub repo, including
//! private repos, using a token from the environment, the `gh` CLI, or
//! `~/.trace/github.json`. Read-only.

use anyhow::{anyhow, Result};
use trace_core::github;

use crate::colors;
use crate::project;

pub enum GithubCmd {
    Status,
    Commits { limit: usize },
    Pulls,
    Cat { path: String, r#ref: Option<String> },
}

pub fn run(cmd: GithubCmd) -> Result<()> {
    let project = project::load_current()?;
    let root = project.root;

    match cmd {
        GithubCmd::Status => {
            let s = github::status_for_path(&root);
            println!("{}", colors::bold("GitHub status"));
            println!("  token source: {}", s.token_source);
            let auth = if s.authenticated {
                colors::green("true")
            } else {
                colors::dim("false")
            };
            println!("  authenticated: {auth}");
            if let Some(login) = s.login {
                println!("  user: {login}");
            }
            match s.repo_ref {
                Some(r) => println!("  repo: {}/{}", r.owner, r.repo),
                None => println!("  repo: {}", colors::dim("(no GitHub origin remote)")),
            }
            if let Some(repo) = s.repo {
                println!(
                    "  {} · default branch {} · ★ {} · {} open issues",
                    if repo.private { "private" } else { "public" },
                    repo.default_branch,
                    repo.stargazers_count,
                    repo.open_issues_count
                );
            }
            if let Some(err) = s.error {
                println!("  note: {err}");
                if !err.is_empty() {
                    println!(
                        "  {}",
                        colors::dim(
                            "(set GITHUB_TOKEN or run `gh auth login` to read private repos)"
                        )
                    );
                }
            }
        }
        GithubCmd::Commits { limit } => {
            let (r, token) = resolve(&root)?;
            for c in github::list_commits(&r, token.as_deref(), limit)? {
                println!(
                    "{}  {}  ({}, {})",
                    colors::brand(&c.sha[..8.min(c.sha.len())]),
                    c.message,
                    c.author,
                    colors::dim(&c.date)
                );
            }
        }
        GithubCmd::Pulls => {
            let (r, token) = resolve(&root)?;
            let pulls = github::list_pulls(&r, token.as_deref())?;
            if pulls.is_empty() {
                println!("{}", colors::dim("No open pull requests."));
            }
            for p in pulls {
                println!(
                    "#{} [{}] {} — @{}",
                    p.number,
                    colors::green(&p.state),
                    p.title,
                    p.user
                );
            }
        }
        GithubCmd::Cat { path, r#ref } => {
            let (r, token) = resolve(&root)?;
            let content = github::get_file(&r, &path, r#ref.as_deref(), token.as_deref())?;
            print!("{content}");
        }
    }
    Ok(())
}

fn resolve(root: &std::path::Path) -> Result<(github::RepoRef, Option<String>)> {
    let repo_ref = trace_core::git::remote_url(root)
        .and_then(|u| github::parse_remote(&u))
        .ok_or_else(|| anyhow!("this project has no GitHub `origin` remote"))?;
    let (token, _src) = github::resolve_token();
    Ok((repo_ref, token))
}
