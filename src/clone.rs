use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Context, Result};
use colored::Colorize;

/// Parse Git repository URL to extract the base name (default project name).
pub fn extract_repo_name(url: &str) -> Option<String> {
    let mut cleaned = url.trim();
    if cleaned.is_empty() {
        return None;
    }
    // Remove any query or fragment parts first
    if let Some(pos) = cleaned.find('?') {
        cleaned = &cleaned[..pos];
    }
    if let Some(pos) = cleaned.find('#') {
        cleaned = &cleaned[..pos];
    }
    // Remove trailing slashes
    while cleaned.ends_with('/') {
        cleaned = &cleaned[..cleaned.len() - 1];
    }
    // Remove trailing .git
    if cleaned.to_lowercase().ends_with(".git") {
        cleaned = &cleaned[..cleaned.len() - 4];
    }
    // Remove trailing slashes again just in case
    while cleaned.ends_with('/') {
        cleaned = &cleaned[..cleaned.len() - 1];
    }
    // Find last separator: '/' or ':'
    let last_sep = cleaned.rfind('/').or_else(|| cleaned.rfind(':'));
    let name = match last_sep {
        Some(pos) => &cleaned[pos + 1..],
        None => cleaned,
    };
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Check if a directory with the given name already exists anywhere under the base directory.
fn check_target_exists(base: &Path, name: &str) -> Option<PathBuf> {
    for cat in crate::classify::Category::all() {
        let cat_dir = base.join(cat.dir_name());
        if !cat_dir.exists() {
            continue;
        }
        // Check standalone
        let standalone = cat_dir.join(name);
        if standalone.exists() {
            return Some(standalone);
        }
        // Check inside group folders
        if let Ok(rd) = std::fs::read_dir(&cat_dir) {
            for entry in rd.filter_map(|e| e.ok()) {
                if entry.path().is_dir() {
                    let grouped = entry.path().join(name);
                    if grouped.exists() {
                        return Some(grouped);
                    }
                }
            }
        }
    }
    None
}

/// Verify that Git is installed and accessible in the system PATH.
fn check_git_installed() -> Result<()> {
    Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("git command not found on $PATH. Please install Git to use the clone command.")?;
    Ok(())
}

/// Automatically detect the preferred editor and spawn it asynchronously in the background.
fn open_editor(path: &Path) -> Result<()> {
    let installed = crate::editors::detect_installed();
    if installed.is_empty() {
        eprintln!("  {} No supported editors found on $PATH.", "warning:".yellow().bold());
        return Ok(());
    }

    let mut prefs = crate::prefs::Prefs::load()?;
    let editor = prefs
        .last_editor_for(path)
        .map(|s| s.to_string())
        .or_else(|| installed.iter().map(|e| e.binary.to_string()).next())
        .unwrap_or_else(|| installed[0].binary.to_string());

    println!("  Opening in {}...", editor.bold());

    let mut cmd = Command::new(&editor);
    cmd.arg(path);

    // Suppress editor outputs so it runs cleanly in background
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    cmd.spawn().with_context(|| format!("Failed to spawn editor '{}'", editor))?;

    // Update preferences
    prefs.set_last_editor(path, &editor)?;
    Ok(())
}

/// Main entry point for the `projm clone` subcommand.
pub fn run(url: &str, name: Option<String>, branch: Option<String>, open: bool) -> Result<()> {
    // 1. Verify Git installation
    check_git_installed()?;

    // 2. Load configured base directory
    let base = crate::config::load().base;
    if !base.exists() {
        std::fs::create_dir_all(&base)
            .with_context(|| format!("Failed to create base directory: {}", base.display()))?;
    }

    // 3. Extract or validate project name
    let target_name = match name {
        Some(n) => n.trim().to_string(),
        None => extract_repo_name(url).ok_or_else(|| {
            anyhow::anyhow!(
                "Could not extract project name from URL. Please specify a custom name:\n  \
                 projm clone <url> <name>"
            )
        })?,
    };

    if target_name.is_empty() {
        anyhow::bail!("Project name cannot be empty.");
    }

    // 4. Verify target directory does not already exist
    if let Some(existing_path) = check_target_exists(&base, &target_name) {
        anyhow::bail!(
            "A project named '{}' already exists in your organized directory at:\n  {}",
            target_name.bold(),
            existing_path.display().to_string().dimmed()
        );
    }

    println!(
        "  Cloning {} into temporary staging...",
        url.cyan()
    );

    // 5. Create staging directory inside base to ensure fast rename operations
    let staging_root = tempfile::Builder::new()
        .prefix(".tmp_clone_")
        .tempdir_in(&base)
        .context("Failed to create temporary staging directory")?;

    let staging_dir = staging_root.path().join(&target_name);

    // 6. Spawn native git clone
    let mut cmd = Command::new("git");
    cmd.arg("clone");
    if let Some(ref b) = branch {
        cmd.arg("--branch").arg(b);
    }
    cmd.arg(url).arg(&staging_dir);

    let status = cmd.status().context("Failed to execute git clone command")?;
    if !status.success() {
        anyhow::bail!("git clone failed with exit status: {}", status);
    }

    println!("  {} Successfully cloned. Organizing project...", "✓".green().bold());

    // 7. Organize staging folder into the destination
    let dest_path = crate::organize::organize_single(&staging_dir)
        .context("Failed to auto-organize cloned project")?;

    println!(
        "  {} Organized under category: {}",
        "✓".green().bold(),
        dest_path.display().to_string().green()
    );

    // 8. Launch editor if requested
    if open {
        open_editor(&dest_path)?;
    }

    Ok(())
}
