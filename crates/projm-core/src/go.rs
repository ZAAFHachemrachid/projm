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
    git_info: Option<GitInfo>,
}

struct GitInfo {
    branch: String,
    is_dirty: bool,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(query: Option<String>, last: bool) -> Result<()> {
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
                    let child_path = child.path();
                    let git_info = get_git_info(&child_path);
                    projects.push(Project {
                        name: child.file_name().to_string_lossy().to_string(),
                        path: child_path,
                        category: cat.clone(),
                        git_info,
                    });
                }
            } else {
                let entry_path = entry.path();
                let git_info = get_git_info(&entry_path);
                projects.push(Project {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry_path,
                    category: cat.clone(),
                    git_info,
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

    let mut selected_project: Option<&Project> = None;
    let mut prefs = Prefs::load()?;

    // 1. Check if we need to jump to the last entered project
    let is_last_shortcut = last || query.as_deref() == Some("-");
    if is_last_shortcut {
        if let Some(ref last_path_str) = prefs.last_project {
            selected_project = projects.iter().find(|p| {
                p.path
                    .canonicalize()
                    .unwrap_or_else(|_| p.path.to_path_buf())
                    .to_string_lossy()
                    == *last_path_str
            });
            if selected_project.is_none() {
                anyhow::bail!("No previously entered project found or it no longer exists.");
            }
        } else {
            anyhow::bail!("No previously entered project found.");
        }
    }

    // 2. Check if a query is provided and it has a single exact case-insensitive match on name
    if selected_project.is_none() {
        if let Some(ref q) = query {
            let q_lower = q.to_lowercase();
            let matches: Vec<&Project> = projects
                .iter()
                .filter(|p| p.name.to_lowercase() == q_lower)
                .collect();
            if matches.len() == 1 {
                selected_project = Some(matches[0]);
            }
        }
    }

    let term = Term::stderr();

    // 3. Fallback to interactive picker
    let project = match selected_project {
        Some(p) => p,
        None => {
            // Find the longest project name to compute perfect padding (default to 20)
            let max_name_len = projects
                .iter()
                .map(|p| p.name.len())
                .max()
                .unwrap_or(20)
                .max(20);

            // ── Fuzzy-pick project ────────────────────────────────────────────────────
            let labels: Vec<String> = projects
                .iter()
                .map(|p| {
                    let cat_lbl = p.category.label();
                    
                    let git_part = if let Some(git) = &p.git_info {
                        let status_indicator = if git.is_dirty {
                            "*".yellow().bold().to_string()
                        } else {
                            "✓".green().to_string()
                        };
                        format!("  {:<15}  {}", git.branch.dimmed(), status_indicator)
                    } else {
                        "".to_string()
                    };

                    format!("  {}  {:<width$}  {}", cat_lbl, p.name, git_part, width = max_name_len)
                })
                .collect();

            let theme = ColorfulTheme::default();
            let mut select = FuzzySelect::with_theme(&theme)
                .with_prompt("jump to")
                .items(&labels);
            if let Some(ref q) = query {
                select = select.with_initial_text(q);
            }
            let idx = select.interact_on(&term)?;
            &projects[idx]
        }
    };

    // ── Save last entered project preference ─────────────────────────────────
    prefs.set_last_project(&project.path)?;

    // ── Pick editor (v0.2) ────────────────────────────────────────────────────
    let editor_binary = pick_editor(&project.path, &term)?;

    // ── Emit eval-able command ────────────────────────────────────────────────
    let path = shell_quote(&project.path.to_string_lossy());
    let cd_cmd = detect_cd();

    println!("{} {} && {} .", cd_cmd, path, editor_binary);

    Ok(())
}

/// Retrieve Git branch and dirty status of a project directory if it is a Git repository.
fn get_git_info(path: &Path) -> Option<GitInfo> {
    if !path.join(".git").exists() {
        return None;
    }

    let branch_output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(path)
        .output()
        .ok()?;

    if !branch_output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&branch_output.stdout).trim().to_string();
    if branch.is_empty() {
        return None;
    }

    let status_output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()
        .ok()?;

    let is_dirty = status_output.status.success() && !status_output.stdout.is_empty();

    Some(GitInfo { branch, is_dirty })
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
             Install one of: nvim, vim, hx, zed, zeditor, code, cursor, idea, emacs"
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
    if std::process::Command::new("zoxide")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success()) { "z" } else { "cd" }
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_git_repo() -> (TempDir, std::path::PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_path_buf();

        // Initialize git repo
        std::process::Command::new("git")
            .arg("init")
            .current_dir(&path)
            .output()
            .unwrap();

        // Git requires config user.name and user.email to commit
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&path)
            .output()
            .unwrap();

        (temp, path)
    }

    #[test]
    fn test_non_git_dir_returns_none() {
        let temp = tempfile::tempdir().unwrap();
        assert!(get_git_info(temp.path()).is_none());
    }

    #[test]
    fn test_git_repo_clean() {
        let (_temp, path) = setup_git_repo();

        // Create a file and commit it so we are on a valid branch (default branch, e.g. main/master)
        let file_path = path.join("hello.txt");
        std::fs::write(&file_path, "hello").unwrap();

        std::process::Command::new("git")
            .args(["add", "hello.txt"])
            .current_dir(&path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["commit", "-m", "initial commit"])
            .current_dir(&path)
            .output()
            .unwrap();

        let info = get_git_info(&path).expect("should find git info");
        assert!(!info.branch.is_empty());
        assert!(!info.is_dirty);
    }

    #[test]
    fn test_git_repo_dirty() {
        let (_temp, path) = setup_git_repo();

        // Create a file and commit it first
        let file_path = path.join("hello.txt");
        std::fs::write(&file_path, "hello").unwrap();

        std::process::Command::new("git")
            .args(["add", "hello.txt"])
            .current_dir(&path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["commit", "-m", "initial commit"])
            .current_dir(&path)
            .output()
            .unwrap();

        // Create a new untracked file to make the repo dirty
        let dirty_path = path.join("dirty.txt");
        std::fs::write(&dirty_path, "uncommitted").unwrap();

        let info = get_git_info(&path).expect("should find git info");
        assert!(!info.branch.is_empty());
        assert!(info.is_dirty);
    }
}

