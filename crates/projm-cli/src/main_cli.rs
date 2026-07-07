use std::path::PathBuf;

use clap::{Parser, Subcommand};

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

// Variant size spread is inherent to clap subcommand enums (one short-lived
// value per process); boxing the big variants only complicates the match arms.
#[allow(clippy::large_enum_variant)]
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
        /// Override the shell target to configure
        #[arg(short = 's', long, value_enum)]
        shell: Option<crate::completions::CompletionShell>,
        /// Override the shell profile path to update
        #[arg(short = 'p', long)]
        profile_path: Option<PathBuf>,
    },
    /// Print shell completion script for a shell
    Completions {
        #[arg(value_enum)]
        shell: crate::completions::CompletionShell,
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
    /// Detect and run the project's dev command
    Run {
        /// Optional path or project name to run
        path_or_query: Option<String>,
        /// List the discovered runnable apps and exit (works without a TTY)
        #[arg(short = 'l', long)]
        list: bool,
        /// Force the multi-app tabbed-log TUI runner
        #[arg(long)]
        tui: bool,
        /// Launch the TUI and start every discovered app immediately
        #[arg(short = 'a', long)]
        all: bool,
        /// Verify that stopping a project reaps its whole process group, then exit
        #[arg(long)]
        selftest: bool,
    },
    /// Show git health for all organized projects (branch, dirty, ahead/behind)
    #[command(alias = "st")]
    Status {
        /// Show only projects that are dirty or out of sync with upstream
        #[arg(short, long)]
        dirty: bool,
        /// Emit machine-readable JSON instead of a table
        #[arg(long)]
        json: bool,
    },
    /// Manage classification rules and per-project category pins
    Rules {
        #[command(subcommand)]
        sub: RulesSubcommands,
    },
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

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug, Clone)]
pub enum RulesSubcommands {
    /// List rules in evaluation order
    List {
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Add a rule (at least one matcher flag is required)
    Add {
        /// Exact directory-name match
        #[arg(long)]
        name: Option<String>,
        /// Substring match on the directory name
        #[arg(long)]
        name_contains: Option<String>,
        /// Glob on the directory name (e.g. "*-api")
        #[arg(long)]
        name_glob: Option<String>,
        /// Regex on the directory name
        #[arg(long)]
        name_regex: Option<String>,
        /// Recognised project suffix (e.g. "fw")
        #[arg(long)]
        suffix: Option<String>,
        /// Immediate parent directory name
        #[arg(long)]
        parent_dir: Option<String>,
        /// A file/dir that must exist in the project root
        #[arg(long)]
        marker: Option<String>,
        /// Dependency that must be present in a manifest
        #[arg(long)]
        has_dep: Option<String>,
        /// Detected stack (rust, js, tauri, go, python, …)
        #[arg(long)]
        stack: Option<String>,
        /// Free-text note shown in listings
        #[arg(long)]
        description: Option<String>,
        /// Target category
        #[arg(short, long)]
        category: String,
        /// Insert at this position (1 = evaluated first); default: append
        #[arg(long)]
        at: Option<usize>,
    },
    /// Remove a rule by list index or exact-name value
    #[command(alias = "rm", alias = "delete")]
    Remove {
        /// Rule index from `projm rules list` (e.g. 2) or a `name` value
        selector: String,
    },
    /// Explain how a path would be classified and why
    Test {
        /// Project directory to test (default: current dir)
        path: Option<PathBuf>,
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Open rules.toml in $EDITOR (validated after save)
    Edit,
    /// Write rules.toml to a file, or stdout if no file given
    Export {
        /// Destination file
        file: Option<PathBuf>,
    },
    /// Import rules from a file (merge by default, skipping duplicates)
    Import {
        /// Source rules file
        file: PathBuf,
        /// Replace the entire rules file instead of merging
        #[arg(long)]
        replace: bool,
    },
    /// Pin a project's category via a .projm.toml marker in the project dir
    Pin {
        /// Project directory (default: current dir)
        path: Option<PathBuf>,
        /// Category to pin
        #[arg(short, long)]
        category: String,
        /// Optional group folder override
        #[arg(long)]
        group: Option<String>,
        /// Hide this project from listings
        #[arg(long)]
        hidden: bool,
    },
    /// Interactively pick an organized project and assign a category
    Assign {
        /// Optional search query to pre-filter the picker
        query: Option<String>,
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
