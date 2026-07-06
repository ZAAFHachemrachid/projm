mod blueprints;
mod completions;
mod init_setup;
mod main_cli;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use main_cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Organize { dir, dry_run } => projm_core::organize::run(&dir, dry_run),
        Commands::G { query, last } => projm_core::go::run(query, last),
        Commands::Init {
            alias,
            non_interactive,
            shell,
            profile_path,
        } => init_setup::run(&alias, non_interactive, shell, profile_path),
        Commands::Completions { shell } => completions::emit(shell),
        Commands::SetBase { path } => projm_core::config::set_base(&path),
        Commands::Editors => {
            print_editors();
            Ok(())
        }
        Commands::Blueprint { sub } => blueprints::run(sub),
        Commands::Check => projm_core::check::run(),
        Commands::Run {
            path_or_query,
            list,
            tui,
            all,
            selftest,
        } => projm_core::runner::dispatch(path_or_query, list, tui, all, selftest),
        Commands::Status { dirty, json } => projm_core::status::run(dirty, json),
        Commands::Clone {
            url,
            name,
            branch,
            open,
        } => projm_core::clone::run(&url, name, branch, open),
    }
}

fn print_editors() {
    let installed = projm_core::editors::detect_installed();
    let installed_set: std::collections::HashSet<&str> =
        installed.iter().map(|e| e.binary).collect();

    println!();
    for (binary, name) in projm_core::editors::KNOWN_EDITORS {
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
            projm_core::editors::KNOWN_EDITORS.len()
        );
    }
    println!();
}
