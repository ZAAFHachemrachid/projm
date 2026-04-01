mod classify;
mod completions;
mod config;
mod editors;
mod go;
mod init_setup;
mod main_cli;
mod organize;
mod prefs;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use main_cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Organize { dir, dry_run } => organize::run(&dir, dry_run),
        Commands::G => go::run(),
        Commands::Init => init_setup::run(),
        Commands::Completions { shell } => completions::emit(shell),
        Commands::SetBase { path } => config::set_base(&path),
        Commands::Editors => {
            print_editors();
            Ok(())
        }
    }
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

