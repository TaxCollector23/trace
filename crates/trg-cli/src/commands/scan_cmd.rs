//! `trg scan` — detect the current project's stack and print it.

use anyhow::Result;
use traceguard_core::scan;

use crate::project;

pub fn run() -> Result<()> {
    let root = project::load_current()
        .map(|p| p.root)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());

    let s = scan::scan(&root);
    println!("Project scan — {}\n", s.root);
    show(
        "Package manager",
        s.package_manager.clone().unwrap_or_else(|| "—".into()),
    );
    show("Languages", join(&s.languages));
    show("Frameworks", join(&s.frameworks));
    show(
        "TypeScript",
        if s.typescript {
            "yes".into()
        } else {
            "no".into()
        },
    );
    show(
        "Test framework",
        s.test_framework.clone().unwrap_or_else(|| "—".into()),
    );
    show("Scripts", join(&s.scripts));
    show(
        "README",
        if s.has_readme {
            "present".into()
        } else {
            "missing".into()
        },
    );
    show("Env files", join(&s.env_files));
    show("Deployment", join(&s.deployment));
    show(
        "Git",
        match &s.git_branch {
            Some(b) => format!(
                "{b}{}",
                if s.git_dirty {
                    " (uncommitted changes)"
                } else {
                    ""
                }
            ),
            None => "not a git repo".into(),
        },
    );

    println!(
        "\nTailor a prompt to this project with:\n  trg guard --project \"fix the dashboard\""
    );
    Ok(())
}

fn show(label: &str, value: String) {
    println!(
        "  {label:<16} {}",
        if value.is_empty() {
            "—".to_string()
        } else {
            value
        }
    );
}

fn join(v: &[String]) -> String {
    v.join(", ")
}
