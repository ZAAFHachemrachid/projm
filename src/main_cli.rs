use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::completions::CompletionShell;

#[derive(Parser)]
#[command(
    name = "projm",
    about = "Project organizer & navigator",
    version = env!("CARGO_PKG_VERSION")
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
        dir: PathBuf,
        /// Preview only — no files moved
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
    /// Fuzzy-pick a project and jump to it  (wrap with `eval` in shell)
    G,
    /// Install shell integration, completions, and zoxide setup
    Init,
    /// Print shell completion script for a shell
    Completions {
        #[arg(value_enum)]
        shell: CompletionShell,
    },
    /// Override the base projects directory (default: ~/projects)
    SetBase { path: PathBuf },
    /// List detected editors on this machine
    Editors,
    /// Manage and run project creation blueprints
    Blueprint {
        #[command(subcommand)]
        sub: Option<BlueprintSubcommands>,
    },
    /// Verify active development tools and environment health
    Check,
}

#[derive(Subcommand, Debug, Clone)]
pub enum BlueprintSubcommands {
    /// Add a new blueprint interactively
    Add,
    /// List all saved blueprints
    List,
    /// Run a blueprint to create a new project
    Run {
        /// Optional name of the blueprint to run
        name: Option<String>,
    },
    /// Edit an existing blueprint
    #[command(alias = "update")]
    Edit {
        /// Optional name of the blueprint to edit
        name: Option<String>,
    },
    /// Delete an existing blueprint
    #[command(alias = "rm", alias = "remove")]
    Delete {
        /// Optional name of the blueprint to delete
        name: Option<String>,
    },
}


