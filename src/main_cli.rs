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
    G {
        /// Optional search query or project name to match
        query: Option<String>,
        /// Jump to the last entered project
        #[arg(short = 'l', long)]
        last: bool,
    },
    /// Install shell integration, completions, and zoxide setup
    Init {
        /// Shell function/alias name
        #[arg(short = 'a', long, default_value = "pg")]
        alias: String,
        /// Run in non-interactive mode, bypassing the onboarding wizard
        #[arg(long)]
        non_interactive: bool,
    },
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
    /// Clone a git repository directly and organize it
    Clone {
        /// Git repository URL (HTTPS or SSH)
        url: String,
        /// Optional custom project name override
        name: Option<String>,
        /// Optional branch or tag to clone
        #[arg(short, long)]
        branch: Option<String>,
        /// Open in preferred editor after cloning
        #[arg(short, long)]
        open: bool,
    },
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


