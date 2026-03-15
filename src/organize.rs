use crate::{
    classify::{self, split_suffix, Category},
    config,
};
use anyhow::Result;
use colored::Colorize;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Confirm};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

// ── Plan entry ────────────────────────────────────────────────────────────────

struct Move {
    src:   PathBuf,
    dest:  PathBuf,        // full destination, keeps original name
    cat:   Category,
    name:  String,
    group: Option<String>, // Some("drivetrack") when grouped
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn run(dir: &Path, dry_run: bool) -> Result<()> {
    let base = config::load().base;
    let term = Term::stderr();

    if !dir.exists() {
        anyhow::bail!("Directory not found: {}", dir.display());
    }

    if dry_run {
        term.write_line(&format!("\n  {}", "[dry-run] nothing will be moved".dimmed()))?;
    }

    // ── Collect immediate subdirectories ──────────────────────────────────────
    let mut raw: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.')
        })
        .collect();
    raw.sort_by_key(|e| e.file_name());

    if raw.is_empty() {
        term.write_line(&format!("  {}", "No subdirectories found.".yellow()))?;
        return Ok(());
    }

    // ── Pass 1: classify ──────────────────────────────────────────────────────
    let classified: Vec<(PathBuf, String, Category)> = raw
        .iter()
        .map(|e| {
            let src  = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            let cat  = classify::classify(&src);
            (src, name, cat)
        })
        .collect();

    // ── Pass 2: count how many projects share each prefix ─────────────────────
    //   drivetrack-api + drivetrack-web → prefix "drivetrack" seen 2 times
    let mut prefix_count: HashMap<String, usize> = HashMap::new();
    for (_, name, _) in &classified {
        if let Some((prefix, _)) = split_suffix(name) {
            *prefix_count.entry(prefix.to_string()).or_insert(0) += 1;
        }
    }

    // ── Pass 3: build final move plan ─────────────────────────────────────────
    let moves: Vec<Move> = classified
        .into_iter()
        .map(|(src, name, cat)| {
            let (dest, group) = resolve_dest(&base, &name, &cat, &prefix_count);
            Move { src, dest, cat, name, group }
        })
        .collect();

    // ── Print plan ────────────────────────────────────────────────────────────
    term.write_line("")?;
    term.write_line(&format!(
        "  {:<8}  {:<18}  {}",
        "category".bold().underline(),
        "group".bold().underline(),
        "name".bold().underline(),
    ))?;
    term.write_line(&format!("  {}", "─".repeat(54).dimmed()))?;

    for m in &moves {
        let group_label = m.group.as_deref().unwrap_or("─");
        term.write_line(&format!(
            "  {}  {:<18}  {}",
            m.cat.label(),
            group_label.dimmed(),
            m.name.bold(),
        ))?;
    }

    term.write_line(&format!("  {}", "─".repeat(54).dimmed()))?;
    term.write_line(&format!(
        "  {} project(s)  →  {}\n",
        moves.len().to_string().bold(),
        base.display().to_string().dimmed()
    ))?;

    if dry_run {
        term.write_line(&format!(
            "  {}",
            "Dry-run complete. Re-run without -n to apply.".dimmed()
        ))?;
        return Ok(());
    }

    // ── Skip anything already in the right place ──────────────────────────────
    let pending: Vec<&Move> = moves.iter().filter(|m| m.src != m.dest).collect();

    if pending.is_empty() {
        term.write_line(&format!("  {}", "Everything is already organised.".green()))?;
        return Ok(());
    }

    // ── Confirm ───────────────────────────────────────────────────────────────
    let ok = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Move {} project(s)?", pending.len()))
        .default(false)
        .interact_on(&term)?;

    if !ok {
        term.write_line(&format!("  {}", "Aborted.".red()))?;
        return Ok(());
    }

    // ── Execute ───────────────────────────────────────────────────────────────
    let (mut moved, mut skipped) = (0usize, 0usize);

    for m in pending {
        if let Some(parent) = m.dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if m.dest.exists() {
            term.write_line(&format!(
                "  {}  {} (destination exists, skipping)",
                "─".yellow(),
                m.name
            ))?;
            skipped += 1;
            continue;
        }

        match do_move(&m.src, &m.dest) {
            Ok(()) => {
                term.write_line(&format!("  {}  {}", "✓".green(), m.name))?;
                moved += 1;
            }
            Err(e) => {
                term.write_line(&format!("  {}  {}: {}", "✗".red(), m.name, e))?;
                skipped += 1;
            }
        }
    }

    term.write_line("")?;
    term.write_line(&format!(
        "  {} moved   {} skipped",
        moved.to_string().green().bold(),
        skipped.to_string().yellow(),
    ))?;

    Ok(())
}

// ── Destination resolution ────────────────────────────────────────────────────

/// Where does a project land?
///
/// `drivetrack-api`  + "drivetrack" seen ≥2 times
///   → `base/apps/drivetrack/drivetrack-api`   (group folder = prefix)
///
/// `drivetrack-api`  + "drivetrack" seen once
///   → `base/services/drivetrack-api`          (no grouping)
///
/// `trashnet`  (no known suffix)
///   → `base/ml/trashnet`
fn resolve_dest(
    base: &Path,
    name: &str,
    cat: &Category,
    prefix_count: &HashMap<String, usize>,
) -> (PathBuf, Option<String>) {
    if let Some((prefix, _)) = split_suffix(name) {
        if prefix_count.get(prefix).copied().unwrap_or(0) >= 2 {
            let dest = base
                .join(cat.dir_name())
                .join(prefix)  // group folder — just the prefix, e.g. "drivetrack"
                .join(name);   // full original name kept, e.g. "drivetrack-api"
            return (dest, Some(prefix.to_string()));
        }
    }
    (base.join(cat.dir_name()).join(name), None)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn do_move(src: &Path, dest: &Path) -> Result<()> {
    match std::fs::rename(src, dest) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(18) => {
            copy_dir_all(src, dest)?;
            std::fs::remove_dir_all(src)?;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}
