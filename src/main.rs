mod classify;
mod config;
mod go;
mod organize;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name    = "projm",
    about   = "Project organizer & navigator",
    version = "0.1.0"
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Organize { dir, dry_run } => organize::run(&dir, dry_run),
        Commands::G                         => go::run(),
        Commands::Init                      => { print_init(); Ok(()) }
        Commands::SetBase { path }          => config::set_base(&path),
    }
}

fn print_init() {
    // Intentional: stdout so user can redirect/pipe this snippet
    print!(r#"# ─── projm shell integration ─────────────────────────────
# Paste into ~/.zshrc (or source this file directly)

pg() {{
    local cmd
    # projm writes the interactive UI to /dev/tty (stderr),
    # only the final eval-able command goes to stdout
    cmd=$(projm g 2>/dev/tty) || return
    [ -n "$cmd" ] && eval "$cmd"
}}

# Optional: alias for quick access
# alias p='pg'
# ───────────────────────────────────────────────────────────
"#);
}
