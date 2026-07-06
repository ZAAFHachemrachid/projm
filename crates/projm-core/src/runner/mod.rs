//! Multi-app, polyglot dev runner with a tabbed-log TUI.
//!
//! `projm run` with no target and ≥2 runnable apps (or a monorepo root) launches an
//! interactive tabbed dashboard that discovers, starts, and supervises every app in the
//! current repo — JS/TS workspaces, Cargo workspace members, Python, C/C++ (CMake/Make),
//! and Arduino/ESP32 (PlatformIO). The classic single-project inline path in
//! [`crate::run::run`] is left untouched and reached as a fallback.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use colored::Colorize;
use console::Term;

pub mod discover;
pub mod process;
pub mod tui;

pub use discover::discover_apps;

/// A single runnable app discovered inside a repo.
#[derive(Debug, Clone)]
pub struct RunnableApp {
    /// Stable, unique identifier (also the digit-select label).
    pub id: String,
    /// Short human label shown in the tab bar.
    pub label: String,
    /// Working directory to spawn the command in.
    pub dir: PathBuf,
    /// Shell command line (run via `sh -c` / `cmd /C`).
    pub command: String,
    /// Best-effort dev port, if one could be inferred.
    pub port: Option<u16>,
    /// One-line description (ecosystem / board / etc).
    pub hint: String,
}

/// Entry point for `projm run` once flags are parsed.
pub fn dispatch(
    path_or_query: Option<String>,
    list: bool,
    tui_flag: bool,
    all: bool,
    selftest: bool,
) -> Result<()> {
    if selftest {
        return process::self_test();
    }

    let root = repo_root(resolve_root(path_or_query.as_deref())?);

    if list {
        let apps = discover_apps(&root);
        print_list(&root, &apps);
        return Ok(());
    }

    let force = tui_flag || all;
    let auto = path_or_query.is_none();

    if force || auto {
        let apps = discover_apps(&root);
        let multi = apps.len() >= 2 || crate::run::find_monorepo_root(&root).is_some();
        if force || multi {
            if apps.is_empty() {
                bail!(
                    "No runnable apps discovered in {}.\nTry `projm run <path>` for a single project.",
                    root.display()
                );
            }
            if !Term::stdout().is_term() {
                eprintln!("projm run: refusing to launch the TUI in a non-interactive shell.");
                eprintln!(
                    "Run `projm run --list` to see the {} discovered apps.",
                    apps.len()
                );
                std::process::exit(1);
            }
            return tui::run_tui(&root, apps, all);
        }
    }

    // Fall through to the classic single-project inline runner (unchanged).
    crate::run::run(path_or_query)
}

/// Resolve the directory to inspect: an explicit dir arg, or the current dir.
fn resolve_root(arg: Option<&str>) -> Result<PathBuf> {
    match arg {
        Some(".") => Ok(std::env::current_dir()?),
        Some(p) if Path::new(p).is_dir() => Ok(PathBuf::from(p)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(p))),
        _ => Ok(std::env::current_dir()?),
    }
}

/// Climb to the enclosing repo root (dir containing `.git`) so discovery covers the whole
/// repo regardless of how deep the current directory is. Falls back to `start`.
fn repo_root(start: PathBuf) -> PathBuf {
    let mut cur = start.as_path();
    loop {
        if cur.join(".git").exists() {
            return cur.to_path_buf();
        }
        match cur.parent() {
            Some(p) => cur = p,
            None => return start,
        }
    }
}

/// Print the discovered app catalog as a table (used by `--list`, TTY-agnostic).
fn print_list(root: &Path, apps: &[RunnableApp]) {
    if apps.is_empty() {
        println!("\n  {}\n", "no runnable apps discovered".yellow());
        return;
    }
    let id_w = apps.iter().map(|a| a.id.len()).max().unwrap_or(2).max(2);
    println!(
        "\n  {} ({} apps)\n",
        root.display().to_string().bold(),
        apps.len()
    );
    for a in apps {
        let port = a.port.map(|p| format!(":{p}")).unwrap_or_default();
        let rel = a
            .dir
            .strip_prefix(root)
            .map(|p| {
                let s = p.display().to_string();
                if s.is_empty() {
                    ".".to_string()
                } else {
                    s
                }
            })
            .unwrap_or_else(|_| a.dir.display().to_string());
        println!(
            "  {:<id_w$}  {:<6}  {}  {}",
            a.id.cyan().bold(),
            port.dimmed(),
            rel.dimmed(),
            a.command,
        );
    }
    println!();
}
