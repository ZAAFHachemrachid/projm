use crate::{classify::Category, config, editors, prefs::Prefs};
use anyhow::Result;
use colored::Colorize;
use console::Term;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Select};
use std::path::{Path, PathBuf};

// ── Types ─────────────────────────────────────────────────────────────────────

struct Project {
    name: String,
    path: PathBuf,
    category: Category,
}

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
    let mut projects: Vec<Project> = Vec::new();

    for cat in Category::all() {
        let cat_dir = base.join(cat.dir_name());
        if !cat_dir.exists() {
            continue;
        }

        let mut top: Vec<_> = std::fs::read_dir(&cat_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.'))
            .collect();
        top.sort_by_key(|e| e.file_name());

        for entry in top {
            if is_group_folder(&entry.path()) {
                let mut children: Vec<_> = std::fs::read_dir(entry.path())?
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.')
                    })
                    .collect();
                children.sort_by_key(|e| e.file_name());
                for child in children {
                    projects.push(Project {
                        name: child.file_name().to_string_lossy().to_string(),
                        path: child.path(),
                        category: cat.clone(),
                    });
                }
            } else {
                projects.push(Project {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry.path(),
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

    // ── Fuzzy-pick project ────────────────────────────────────────────────────
    let labels: Vec<String> = projects
        .iter()
        .map(|p| format!("{}  {}", p.category.label(), p.name))
        .collect();

    let term = Term::stderr();

    let idx = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("jump to")
        .items(&labels)
        .interact_on(&term)?;

    let project = &projects[idx];

    // ── Pick editor (v0.2) ────────────────────────────────────────────────────
    let editor_binary = pick_editor(&project.path, &term)?;

    // ── Emit eval-able command ────────────────────────────────────────────────
    let path = shell_quote(&project.path.to_string_lossy());
    let cd_cmd = detect_cd();

    println!("{} {} && {} .", cd_cmd, path, editor_binary);

    Ok(())
}

// ── Editor selection (v0.2) ───────────────────────────────────────────────────

/// Returns the binary name of the chosen editor.
///
/// Decision tree:
///   0 installed → bail with helpful hint
///   1 installed → use it silently, skip the picker
///   2+          → show picker, pre-select last-used, persist choice
fn pick_editor(project_path: &Path, term: &Term) -> Result<String> {
    let installed = editors::detect_installed();

    match installed.len() {
        0 => anyhow::bail!(
            "no supported editors found on $PATH.\n\
             Install one of: nvim, vim, hx, zed, code, cursor, idea, emacs"
        ),

        1 => {
            let e = &installed[0];
            term.write_line(&format!("  {} {}", "→".dimmed(), e.name.bold()))?;
            Ok(e.binary.to_owned())
        }

        _ => {
            let mut prefs = Prefs::load()?;

            // Pre-select the last-used editor if it's still installed.
            let default_idx = prefs
                .last_editor_for(project_path)
                .and_then(|saved| installed.iter().position(|e| e.binary == saved))
                .unwrap_or(0);

            let labels: Vec<String> = installed.iter().map(|e| format!("  {}", e.name)).collect();

            let chosen = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("open with")
                .items(&labels)
                .default(default_idx)
                .interact_on(term)?;

            let editor = &installed[chosen];
            prefs.set_last_editor(project_path, editor.binary)?;

            Ok(editor.binary.to_owned())
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn is_group_folder(path: &Path) -> bool {
    let parent_name = match path.file_name() {
        Some(n) => n.to_string_lossy().to_lowercase(),
        None => return false,
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

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

