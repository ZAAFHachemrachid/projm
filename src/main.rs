mod classify;
mod config;
mod editors;
mod go;
mod organize;
mod prefs;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "projm",
    about = "Project organizer & navigator",
    version = "0.2.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a directory and move projects into ~/projects/<category>/
    Organize {
        /// Directory to scan
        dir: PathBuf,
        /// Preview only — no files moved
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
    /// Fuzzy-pick a project and jump to it  (wrap with `eval` in shell)
    G,
    /// Print the shell function to add to ~/.zshrc
    Init,
    /// Override the base projects directory (default: ~/projects)
    SetBase { path: PathBuf },
    /// List detected editors on this machine
    Editors,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Organize { dir, dry_run } => organize::run(&dir, dry_run),
        Commands::G => go::run(),
        Commands::Init => {
            print_init();
            Ok(())
        }
        Commands::SetBase { path } => config::set_base(&path),
        Commands::Editors => {
            print_editors();
            Ok(())
        }
    }
}

fn print_init() {
    print!(
        r#"# ─── projm shell integration ─────────────────────────────
# Paste into ~/.zshrc (or source this file directly)
pg() {{
    local cmd
    cmd=$(projm g 2>/dev/tty) || return
    [ -n "$cmd" ] && eval "$cmd"
}}
# ───────────────────────────────────────────────────────────
"#
    );
}

fn print_editors() {
    let installed = editors::detect_installed();
    let installed_set: std::collections::HashSet<&str> =
        installed.iter().map(|e| e.binary).collect();

    println!();
    for (binary, name) in editors::KNOWN_EDITORS {
        if installed_set.contains(binary) {
            let path = installed
                .iter()
                .find(|e| e.binary == *binary)
                .map(|e| e.path.display().to_string())
                .unwrap_or_default();
            println!(
                "  {}  {:<12}  {}",
                "✓".green().bold(),
                name.bold(),
                path.dimmed()
            );
        } else {
            println!("  {}  {}", "✗".dimmed(), name.dimmed());
        }
    }

    println!();
    if installed.is_empty() {
        println!("{}", "  no supported editors found on $PATH".yellow());
    } else {
        println!(
            "  {}/{} editors installed",
            installed.len().to_string().bold(),
            editors::KNOWN_EDITORS.len()
        );
    }
    println!();
}

