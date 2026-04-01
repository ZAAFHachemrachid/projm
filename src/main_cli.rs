use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::completions::CompletionShell;

#[derive(Parser)]
#[command(
    name = "projm",
    about = "Project organizer & navigator",
    version = "0.3.0"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan a directory and move projects into ~/projects/<category>/
    Organize {
        /// Directory to scan
        pub dir: PathBuf,
        /// Preview only — no files moved
        #[arg(short = 'n', long)]
        pub dry_run: bool,
    },
    /// Fuzzy-pick a project and jump to it  (wrap with `eval` in shell)
    G,
    /// Install shell integration, completions, and zoxide setup
    Init,
    /// Print shell completion script for a shell
    Completions {
        #[arg(value_enum)]
        pub shell: CompletionShell,
    },
    /// Override the base projects directory (default: ~/projects)
    SetBase { path: PathBuf },
    /// List detected editors on this machine
    Editors,
}

