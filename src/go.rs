use crate::{classify::Category, config};
use anyhow::Result;
use console::Term;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Select};
use std::path::{Path, PathBuf};

// ── Types ─────────────────────────────────────────────────────────────────────

struct Project {
    name:     String,
    path:     PathBuf,
    category: Category,
}

const EDITORS: &[(&str, &str)] = &[
    ("nvim",         "nvim"),
    ("zed",          "zed"),
    ("code",         "code"),
    ("antigravity",  "antigravity"),
    ("kiro",         "kiro"),
];

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run() -> Result<()> {
    let base = config::load().base;

    if !base.exists() {
        anyhow::bail!(
            "Base directory {} not found.\n\
             Run `projm organize <dir>` or `projm set-base <path>` first.",
            base.display()
        );
    }

    // ── Collect projects ──────────────────────────────────────────────────────
    // Layout handled:
    //   base/apps/trashnet                     ← solo
    //   base/apps/drivetrack/drivetrack-api    ← grouped (one level deeper)
    //   base/apps/drivetrack/drivetrack-web
    let mut projects: Vec<Project> = Vec::new();

    for cat in Category::all() {
        let cat_dir = base.join(cat.dir_name());
        if !cat_dir.exists() {
            continue;
        }

        let mut top: Vec<_> = std::fs::read_dir(&cat_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().is_dir()
                    && !e.file_name().to_string_lossy().starts_with('.')
            })
            .collect();
        top.sort_by_key(|e| e.file_name());

        for entry in top {
            if is_group_folder(&entry.path()) {
                // Expand: each child is a real project
                let mut children: Vec<_> = std::fs::read_dir(entry.path())?
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path().is_dir()
                            && !e.file_name().to_string_lossy().starts_with('.')
                    })
                    .collect();
                children.sort_by_key(|e| e.file_name());
                for child in children {
                    projects.push(Project {
                        name:     child.file_name().to_string_lossy().to_string(),
                        path:     child.path(),
                        category: cat.clone(),
                    });
                }
            } else {
                projects.push(Project {
                    name:     entry.file_name().to_string_lossy().to_string(),
                    path:     entry.path(),
                    category: cat.clone(),
                });
            }
        }
    }

    if projects.is_empty() {
        eprintln!(
            "  No projects found under {}.\n  Run `projm organize <dir>` or add projects manually.",
            base.display()
        );
        return Ok(());
    }

    // ── Build display labels ──────────────────────────────────────────────────
    let labels: Vec<String> = projects
        .iter()
        .map(|p| format!("{}  {}", p.category.label(), p.name))
        .collect();

    // All interactive UI → stderr; only the final eval command → stdout
    let term = Term::stderr();

    let idx = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("jump to")
        .items(&labels)
        .interact_on(&term)?;

    let editor_labels: Vec<&str> = EDITORS.iter().map(|(label, _)| *label).collect();
    let editor_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("editor")
        .items(&editor_labels)
        .default(0)
        .interact_on(&term)?;

    let path   = shell_quote(&projects[idx].path.to_string_lossy());
    let editor = EDITORS[editor_idx].1;
    let cd_cmd = detect_cd();

    // This single line is eval'd by the pg() shell function
    println!("{} {} && {} .", cd_cmd, path, editor);

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// A "group folder" is a directory whose immediate children all share
/// its own name as a prefix, e.g.:
///   drivetrack/
///     drivetrack-api/
///     drivetrack-web/
///
/// Heuristic: ≥1 child dir starts with `parent_name + '-'` or `parent_name + '_'`
fn is_group_folder(path: &Path) -> bool {
    let parent_name = match path.file_name() {
        Some(n) => n.to_string_lossy().to_lowercase(),
        None    => return false,
    };
    std::fs::read_dir(path)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .any(|e| {
                    let child = e.file_name().to_string_lossy().to_lowercase();
                    child.starts_with(&format!("{}-", parent_name))
                        || child.starts_with(&format!("{}_", parent_name))
                })
        })
        .unwrap_or(false)
}

/// Use `z` (zoxide) if available, else `cd`
fn detect_cd() -> &'static str {
    std::process::Command::new("zoxide")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_or(false, |s| s.success())
        .then_some("z")
        .unwrap_or("cd")
}

/// POSIX single-quote escaping
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
