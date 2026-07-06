use crate::{classify::Category, config, go};
use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    thread,
};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct GitStatus {
    pub branch: String,
    pub changed: usize,
    pub untracked: usize,
    pub ahead: usize,
    pub behind: usize,
    pub upstream: bool,
}

impl GitStatus {
    pub fn is_dirty(&self) -> bool {
        self.changed + self.untracked > 0
    }

    pub fn needs_sync(&self) -> bool {
        self.ahead > 0 || self.behind > 0
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectStatus {
    pub name: String,
    pub category: Category,
    pub path: PathBuf,
    pub git: Option<GitStatus>,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(dirty_only: bool, json: bool) -> Result<()> {
    let base = config::load().base;

    if !base.exists() {
        anyhow::bail!(
            "Base directory {} not found.\n\
             Run `projm organize <dir>` or `projm set-base <path>` first.",
            base.display()
        );
    }

    let mut projects = collect(&base)?;

    if projects.is_empty() {
        eprintln!(
            "  No projects found under {}.\n  Run `projm organize <dir>` or add projects manually.",
            base.display()
        );
        return Ok(());
    }

    if dirty_only {
        projects.retain(|p| {
            p.git
                .as_ref()
                .is_some_and(|g| g.is_dirty() || g.needs_sync())
        });
        if projects.is_empty() {
            println!("  {}", "all repositories clean and in sync ✓".green());
            return Ok(());
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&projects)?);
        return Ok(());
    }

    render_table(&projects);
    Ok(())
}

// ── Collection ────────────────────────────────────────────────────────────────

/// Enumerate all organized projects under `base` and gather their git status
/// in parallel. Walking rules (categories, group folders) mirror `go::run`.
pub fn collect(base: &Path) -> Result<Vec<ProjectStatus>> {
    let mut found: Vec<(String, Category, PathBuf)> = Vec::new();

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
            if go::is_group_folder(&entry.path()) {
                let mut children: Vec<_> = std::fs::read_dir(entry.path())?
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.')
                    })
                    .collect();
                children.sort_by_key(|e| e.file_name());
                for child in children {
                    found.push((
                        child.file_name().to_string_lossy().to_string(),
                        cat.clone(),
                        child.path(),
                    ));
                }
            } else {
                found.push((
                    entry.file_name().to_string_lossy().to_string(),
                    cat.clone(),
                    entry.path(),
                ));
            }
        }
    }

    // Query git across projects in parallel; preserve walking order via index.
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<(usize, ProjectStatus)>> = Mutex::new(Vec::with_capacity(found.len()));

    thread::scope(|s| {
        let workers = found.len().clamp(1, 8);
        for _ in 0..workers {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((name, category, path)) = found.get(i) else {
                    break;
                };
                let status = ProjectStatus {
                    name: name.clone(),
                    category: category.clone(),
                    path: path.clone(),
                    git: git_status(path),
                };
                out.lock().expect("status worker poisoned").push((i, status));
            });
        }
    });

    let mut results = out.into_inner().expect("status workers poisoned");
    results.sort_by_key(|(i, _)| *i);
    Ok(results.into_iter().map(|(_, p)| p).collect())
}

/// Snapshot a repository's health with a single `git status` invocation.
/// Ahead/behind comes from the local tracking ref — no network access.
fn git_status(path: &Path) -> Option<GitStatus> {
    if !path.join(".git").exists() {
        return None;
    }

    let output = std::process::Command::new("git")
        .args(["status", "--porcelain=v2", "--branch"])
        .current_dir(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(parse_porcelain(&String::from_utf8_lossy(&output.stdout)))
}

/// Parse `git status --porcelain=v2 --branch` output.
fn parse_porcelain(output: &str) -> GitStatus {
    let mut status = GitStatus {
        branch: String::new(),
        changed: 0,
        untracked: 0,
        ahead: 0,
        behind: 0,
        upstream: false,
    };

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            status.branch = rest.trim().to_string();
        } else if line.starts_with("# branch.upstream ") {
            status.upstream = true;
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            for part in rest.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    status.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    status.behind = n.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("1 ") || line.starts_with("2 ") || line.starts_with("u ") {
            status.changed += 1;
        } else if line.starts_with("? ") {
            status.untracked += 1;
        }
    }

    status
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn render_table(projects: &[ProjectStatus]) {
    let max_name_len = projects
        .iter()
        .map(|p| p.name.len())
        .max()
        .unwrap_or(20)
        .max(20);
    let max_branch_len = projects
        .iter()
        .filter_map(|p| p.git.as_ref().map(|g| g.branch.len()))
        .max()
        .unwrap_or(15)
        .max(15);

    println!();
    let mut dirty = 0usize;
    let mut out_of_sync = 0usize;
    let mut non_git = 0usize;

    for p in projects {
        match &p.git {
            Some(git) => {
                // Pad before colorizing — ANSI escapes would otherwise eat the pad width.
                let state = if git.is_dirty() {
                    dirty += 1;
                    format!("{:<4}", format!("*{}", git.changed + git.untracked))
                        .yellow()
                        .bold()
                        .to_string()
                } else {
                    format!("{:<4}", "✓").green().to_string()
                };

                let sync = if !git.upstream {
                    "no upstream".dimmed().to_string()
                } else if git.needs_sync() {
                    out_of_sync += 1;
                    let mut parts = Vec::new();
                    if git.ahead > 0 {
                        parts.push(format!("↑{}", git.ahead).cyan().bold().to_string());
                    }
                    if git.behind > 0 {
                        parts.push(format!("↓{}", git.behind).red().bold().to_string());
                    }
                    parts.join(" ")
                } else {
                    "·".dimmed().to_string()
                };

                println!(
                    "  {}  {:<name_w$}  {}  {}  {}",
                    p.category.label(),
                    p.name,
                    format!("{:<w$}", git.branch, w = max_branch_len).dimmed(),
                    state,
                    sync,
                    name_w = max_name_len,
                );
            }
            None => {
                non_git += 1;
                println!(
                    "  {}  {}  {}",
                    p.category.label(),
                    format!("{:<name_w$}", p.name, name_w = max_name_len).dimmed(),
                    "not a git repo".dimmed(),
                );
            }
        }
    }

    println!();
    println!(
        "  {} projects · {} dirty · {} out of sync · {} non-git",
        projects.len().to_string().bold(),
        dirty.to_string().yellow().bold(),
        out_of_sync.to_string().cyan().bold(),
        non_git.to_string().dimmed(),
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── Parser unit tests (pure, no git required) ─────────────────────────────

    #[test]
    fn test_parse_clean_repo_with_upstream() {
        let output = "\
# branch.oid 1234567890abcdef
# branch.head main
# branch.upstream origin/main
# branch.ab +0 -0
";
        let s = parse_porcelain(output);
        assert_eq!(s.branch, "main");
        assert!(s.upstream);
        assert_eq!(s.ahead, 0);
        assert_eq!(s.behind, 0);
        assert!(!s.is_dirty());
        assert!(!s.needs_sync());
    }

    #[test]
    fn test_parse_counts_changed_and_untracked() {
        let output = "\
# branch.oid 1234567890abcdef
# branch.head main
1 .M N... 100644 100644 100644 abc def src/lib.rs
2 R. N... 100644 100644 100644 abc def R100 new.rs\told.rs
u UU N... 100644 100644 100644 100644 abc def ghi conflict.rs
? scratch.txt
? notes.md
";
        let s = parse_porcelain(output);
        assert_eq!(s.changed, 3);
        assert_eq!(s.untracked, 2);
        assert!(s.is_dirty());
    }

    #[test]
    fn test_parse_ahead_behind() {
        let output = "\
# branch.oid 1234567890abcdef
# branch.head feature
# branch.upstream origin/feature
# branch.ab +3 -2
";
        let s = parse_porcelain(output);
        assert_eq!(s.ahead, 3);
        assert_eq!(s.behind, 2);
        assert!(s.needs_sync());
    }

    #[test]
    fn test_parse_detached_head_and_no_upstream() {
        let output = "\
# branch.oid 1234567890abcdef
# branch.head (detached)
";
        let s = parse_porcelain(output);
        assert_eq!(s.branch, "(detached)");
        assert!(!s.upstream);
        assert_eq!(s.ahead, 0);
        assert_eq!(s.behind, 0);
    }

    // ── Integration tests (real git, cross-platform) ──────────────────────────

    fn setup_git_repo() -> (TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_path_buf();

        std::process::Command::new("git")
            .arg("init")
            .current_dir(&path)
            .output()
            .unwrap();
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

    fn commit_file(path: &Path, name: &str, message: &str) {
        std::fs::write(path.join(name), "content").unwrap();
        std::process::Command::new("git")
            .args(["add", name])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(path)
            .output()
            .unwrap();
    }

    #[test]
    fn test_git_status_non_git_dir() {
        let temp = tempfile::tempdir().unwrap();
        assert!(git_status(temp.path()).is_none());
    }

    #[test]
    fn test_git_status_clean_then_dirty() {
        let (_temp, path) = setup_git_repo();
        commit_file(&path, "hello.txt", "initial commit");

        let clean = git_status(&path).expect("should read git status");
        assert!(!clean.branch.is_empty());
        assert!(!clean.is_dirty());

        std::fs::write(path.join("dirty.txt"), "uncommitted").unwrap();
        let dirty = git_status(&path).expect("should read git status");
        assert_eq!(dirty.untracked, 1);
        assert!(dirty.is_dirty());
    }

    #[test]
    fn test_git_status_ahead_of_local_remote() {
        let (_temp, path) = setup_git_repo();
        commit_file(&path, "hello.txt", "initial commit");

        let remote_temp = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_temp.path())
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                &remote_temp.path().to_string_lossy(),
            ])
            .current_dir(&path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["push", "-u", "origin", "HEAD"])
            .current_dir(&path)
            .output()
            .unwrap();

        let synced = git_status(&path).expect("should read git status");
        assert!(synced.upstream);
        assert_eq!(synced.ahead, 0);

        commit_file(&path, "second.txt", "second commit");
        let ahead = git_status(&path).expect("should read git status");
        assert_eq!(ahead.ahead, 1);
        assert!(ahead.needs_sync());
    }
}
