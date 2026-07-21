//! Project scanning: detect the stack so prompts can be tailored automatically.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectScan {
    pub root: String,
    pub package_manager: Option<String>,
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub test_framework: Option<String>,
    pub typescript: bool,
    pub scripts: Vec<String>,
    pub has_readme: bool,
    pub env_files: Vec<String>,
    pub deployment: Vec<String>,
    pub git_branch: Option<String>,
    pub git_dirty: bool,
}

fn exists(root: &Path, p: &str) -> bool {
    root.join(p).exists()
}

fn read(root: &Path, p: &str) -> Option<String> {
    std::fs::read_to_string(root.join(p)).ok()
}

/// Scan a project directory and detect its stack.
pub fn scan(root: &Path) -> ProjectScan {
    let mut s = ProjectScan {
        root: root.display().to_string(),
        ..Default::default()
    };

    // Package manager (lockfile-driven).
    s.package_manager = if exists(root, "pnpm-lock.yaml") {
        Some("pnpm".into())
    } else if exists(root, "yarn.lock") {
        Some("yarn".into())
    } else if exists(root, "bun.lockb") {
        Some("bun".into())
    } else if exists(root, "package-lock.json") || exists(root, "package.json") {
        Some("npm".into())
    } else if exists(root, "Cargo.toml") {
        Some("cargo".into())
    } else if exists(root, "pyproject.toml") || exists(root, "requirements.txt") {
        Some("pip".into())
    } else if exists(root, "go.mod") {
        Some("go".into())
    } else {
        None
    };

    // Languages.
    if exists(root, "Cargo.toml") {
        s.languages.push("Rust".into());
    }
    if exists(root, "go.mod") {
        s.languages.push("Go".into());
    }
    if exists(root, "pyproject.toml") || exists(root, "requirements.txt") {
        s.languages.push("Python".into());
    }
    s.typescript = exists(root, "tsconfig.json");
    if s.typescript {
        s.languages.push("TypeScript".into());
    }

    // package.json-driven detection.
    if let Some(pkg) = read(root, "package.json") {
        if !s.typescript && (pkg.contains("\"typescript\"")) {
            s.typescript = true;
            s.languages.push("TypeScript".into());
        }
        if !s.languages.iter().any(|l| l == "TypeScript") {
            s.languages.push("JavaScript".into());
        }
        let fw = [
            ("next", "Next.js"),
            ("\"react\"", "React"),
            ("\"vue\"", "Vue"),
            ("svelte", "Svelte"),
            ("\"vite\"", "Vite"),
            ("\"express\"", "Express"),
            ("\"@angular/core\"", "Angular"),
            ("astro", "Astro"),
        ];
        for (needle, name) in fw {
            if pkg.contains(needle) {
                s.frameworks.push(name.to_string());
            }
        }
        for (needle, name) in [
            ("vitest", "vitest"),
            ("jest", "jest"),
            ("mocha", "mocha"),
            ("playwright", "playwright"),
        ] {
            if pkg.contains(needle) {
                s.test_framework.get_or_insert(name.to_string());
            }
        }
        // Scripts: pull keys from the "scripts" object (best-effort, no JSON dep beyond serde_json).
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&pkg) {
            if let Some(scripts) = v.get("scripts").and_then(|s| s.as_object()) {
                s.scripts = scripts.keys().cloned().collect();
                s.scripts.sort();
            }
        }
    }

    // Rust test framework.
    if s.test_framework.is_none() && exists(root, "Cargo.toml") {
        s.test_framework = Some("cargo test".into());
    }

    s.has_readme = exists(root, "README.md") || exists(root, "readme.md");

    // Env files.
    for f in [".env", ".env.local", ".env.production", ".env.development"] {
        if exists(root, f) {
            s.env_files.push(f.to_string());
        }
    }

    // Deployment config.
    for (f, name) in [
        ("vercel.json", "Vercel"),
        ("netlify.toml", "Netlify"),
        ("Dockerfile", "Docker"),
        ("fly.toml", "Fly.io"),
        ("firebase.json", "Firebase"),
        (".github/workflows", "GitHub Actions"),
    ] {
        if exists(root, f) {
            s.deployment.push(name.to_string());
        }
    }

    // Git.
    let git = crate::git::capture_state(root);
    if git.is_repo {
        s.git_dirty = git.dirty;
        if let Ok(out) = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(root)
            .output()
        {
            if out.status.success() {
                let b = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !b.is_empty() {
                    s.git_branch = Some(b);
                }
            }
        }
    }

    s
}

impl ProjectScan {
    /// Render a compact project-context block for injecting into prompts.
    pub fn to_context(&self) -> String {
        let mut lines = Vec::new();
        if let Some(pm) = &self.package_manager {
            lines.push(format!("Package manager: {pm}"));
        }
        if !self.languages.is_empty() {
            lines.push(format!("Languages: {}", self.languages.join(", ")));
        }
        if !self.frameworks.is_empty() {
            lines.push(format!("Frameworks: {}", self.frameworks.join(", ")));
        }
        if let Some(t) = &self.test_framework {
            lines.push(format!("Tests: {t}"));
        }
        if !self.scripts.is_empty() {
            let shown: Vec<_> = self.scripts.iter().take(12).cloned().collect();
            lines.push(format!("Scripts: {}", shown.join(", ")));
        }
        if !self.deployment.is_empty() {
            lines.push(format!("Deployment: {}", self.deployment.join(", ")));
        }
        if let Some(b) = &self.git_branch {
            lines.push(format!(
                "Git branch: {b}{}",
                if self.git_dirty {
                    " (uncommitted changes)"
                } else {
                    ""
                }
            ));
        }
        if !self.env_files.is_empty() {
            lines.push(format!(
                "Env files present: {} (do not print their contents)",
                self.env_files.join(", ")
            ));
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_a_node_project() {
        let dir = std::env::temp_dir().join(format!("tg-scan-{}", uuid_like()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("package.json"),
            r#"{"scripts":{"build":"vite build","test":"vitest"},"devDependencies":{"vite":"5","vitest":"1","typescript":"5"},"dependencies":{"react":"18"}}"#,
        )
        .unwrap();
        std::fs::write(dir.join("README.md"), "# x").unwrap();
        let s = scan(&dir);
        assert_eq!(s.package_manager.as_deref(), Some("npm"));
        assert!(s.frameworks.iter().any(|f| f == "React"));
        assert!(s.frameworks.iter().any(|f| f == "Vite"));
        assert_eq!(s.test_framework.as_deref(), Some("vitest"));
        assert!(s.typescript);
        assert!(s.has_readme);
        assert!(s.scripts.contains(&"build".to_string()));
        let ctx = s.to_context();
        assert!(ctx.contains("React"));
        std::fs::remove_dir_all(&dir).ok();
    }

    fn uuid_like() -> u128 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }
}
