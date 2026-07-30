//! Deterministic patch diagnosis.
//!
//! No AI here. Given the changed files and captured check output, produce simple,
//! explainable hypotheses about what likely broke. Everything is grounded in
//! actual diffs and output text.

use crate::git::DiffEntry;
use crate::models::ChangeType;

const DEPENDENCY_FILES: &[&str] = &[
    "package.json",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "Cargo.toml",
    "Cargo.lock",
    "requirements.txt",
    "poetry.lock",
    "pyproject.toml",
    "go.mod",
    "go.sum",
];

const CONFIG_FILES: &[&str] = &[
    "tsconfig.json",
    "vite.config.ts",
    "vite.config.js",
    "webpack.config.js",
    "babel.config.js",
    ".eslintrc",
    "next.config.js",
    "rollup.config.js",
    "jest.config.js",
];

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// True for dependency manifests / lockfiles.
pub fn is_dependency_file(path: &str) -> bool {
    DEPENDENCY_FILES.contains(&file_name(path))
}

/// True for build/tooling config files.
pub fn is_config_file(path: &str) -> bool {
    let name = file_name(path);
    CONFIG_FILES.iter().any(|c| name.starts_with(c))
}

/// True for environment files.
pub fn is_env_file(path: &str) -> bool {
    crate::secrets::is_env_like_filename(path)
}

/// A single human-readable hypothesis about a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hypothesis {
    pub summary: String,
}

/// Given the changed files and combined check/command output, return ordered
/// hypotheses for what likely caused a failure. Empty when nothing lines up.
pub fn diagnose(changes: &[DiffEntry], output: &str) -> Vec<Hypothesis> {
    let mut hyps: Vec<Hypothesis> = Vec::new();
    let lower = output.to_lowercase();

    let dep_changes: Vec<&DiffEntry> = changes
        .iter()
        .filter(|c| is_dependency_file(&c.path))
        .collect();
    let config_changes: Vec<&DiffEntry> =
        changes.iter().filter(|c| is_config_file(&c.path)).collect();

    // Missing module after a dependency change.
    if (lower.contains("cannot find module")
        || lower.contains("module not found")
        || lower.contains("modulenotfounderror")
        || lower.contains("no module named"))
        && !dep_changes.is_empty()
    {
        let files: Vec<String> = dep_changes.iter().map(|c| c.path.clone()).collect();
        hyps.push(Hypothesis {
            summary: format!(
                "A module could not be resolved and dependency files changed in this run ({}). The install may be out of sync — try reinstalling dependencies.",
                files.join(", ")
            ),
        });
    }

    // Output references a specific changed file.
    for change in changes {
        if change.change_type == ChangeType::Deleted {
            if lower.contains(&change.path.to_lowercase()) {
                hyps.push(Hypothesis {
                    summary: format!(
                        "`{}` was deleted in this run and is referenced in the failure output.",
                        change.path
                    ),
                });
            }
            continue;
        }
        let name = file_name(&change.path).to_lowercase();
        if !name.is_empty() && lower.contains(&name) && lower.contains("error") {
            hyps.push(Hypothesis {
                summary: format!(
                    "The error output mentions `{}`, which was {} in this run.",
                    change.path,
                    change.change_type.as_str()
                ),
            });
        }
    }

    // Build/config relationship.
    if (lower.contains("build failed")
        || lower.contains("error")
        || lower.contains("failed to compile"))
        && !config_changes.is_empty()
    {
        let files: Vec<String> = config_changes.iter().map(|c| c.path.clone()).collect();
        hyps.push(Hypothesis {
            summary: format!(
                "Build/tooling config changed in this run ({}); a failed build may stem from those changes.",
                files.join(", ")
            ),
        });
    }

    // TypeScript errors against changed .ts files.
    if lower.contains("ts(") || lower.contains("error ts") {
        let ts_files: Vec<String> = changes
            .iter()
            .filter(|c| c.path.ends_with(".ts") || c.path.ends_with(".tsx"))
            .map(|c| c.path.clone())
            .collect();
        if !ts_files.is_empty() {
            hyps.push(Hypothesis {
                summary: format!(
                    "TypeScript errors were reported and these TS files changed: {}.",
                    ts_files.join(", ")
                ),
            });
        }
    }

    // De-duplicate while preserving order.
    let mut seen = std::collections::HashSet::new();
    hyps.retain(|h| seen.insert(h.summary.clone()));
    hyps
}

/// A short, plain-English summary of a set of changes for the patch review page.
pub fn change_summary(changes: &[DiffEntry]) -> String {
    if changes.is_empty() {
        return "No file changes were detected for this run.".to_string();
    }
    let mut created = 0;
    let mut modified = 0;
    let mut deleted = 0;
    let mut renamed = 0;
    for c in changes {
        match c.change_type {
            ChangeType::Created => created += 1,
            ChangeType::Modified => modified += 1,
            ChangeType::Deleted => deleted += 1,
            ChangeType::Renamed => renamed += 1,
        }
    }
    let mut parts = Vec::new();
    if created > 0 {
        parts.push(format!("{created} added"));
    }
    if modified > 0 {
        parts.push(format!("{modified} modified"));
    }
    if deleted > 0 {
        parts.push(format!("{deleted} deleted"));
    }
    if renamed > 0 {
        parts.push(format!("{renamed} renamed"));
    }

    let mut notes = Vec::new();
    if changes.iter().any(|c| is_dependency_file(&c.path)) {
        notes.push("dependency files changed");
    }
    if changes.iter().any(|c| is_config_file(&c.path)) {
        notes.push("build config changed");
    }
    if changes.iter().any(|c| is_env_file(&c.path)) {
        notes.push("environment files touched");
    }

    let base = format!("{} file(s): {}.", changes.len(), parts.join(", "));
    if notes.is_empty() {
        base
    } else {
        format!("{base} Notable: {}.", notes.join("; "))
    }
}
