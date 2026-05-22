use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::main_cli::BlueprintSubcommands;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Blueprint {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintsStore {
    pub blueprints: Vec<Blueprint>,
}

impl BlueprintsStore {
    pub fn load() -> Result<Self> {
        Self::load_from(default_path()?)
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&default_path()?)
    }

    pub fn load_from(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let store = serde_json::from_str(&content).unwrap_or_default();
        Ok(store)
    }

    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

fn default_path() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve XDG config dir"))?
        .join("projm/blueprints.json"))
}

pub fn run(sub: Option<BlueprintSubcommands>) -> Result<()> {
    match sub {
        Some(BlueprintSubcommands::Add) => add()?,
        Some(BlueprintSubcommands::List) => list()?,
        Some(BlueprintSubcommands::Run { name }) => run_blueprint(name)?,
        None => run_blueprint(None)?,
    }
    Ok(())
}

fn add() -> Result<()> {
    use dialoguer::{theme::ColorfulTheme, Input};

    let theme = ColorfulTheme::default();
    println!();
    println!("{}", "  ✨ Add New Project Blueprint ✨".bold().cyan());
    println!();

    let name: String = Input::with_theme(&theme)
        .with_prompt("Blueprint Name (e.g. better-t-stack)")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.trim().is_empty() {
                return Err("Name cannot be empty.");
            }
            if !input
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                return Err("Name must be alphanumeric, dashes, or underscores.");
            }
            Ok(())
        })
        .interact()?;

    let mut store = BlueprintsStore::load()?;
    if store.blueprints.iter().any(|b| b.name == name) {
        println!("  {} Blueprint '{}' already exists.", "✗".red(), name);
        return Ok(());
    }

    let command: String = Input::with_theme(&theme)
        .with_prompt("Command Template (use {name} as project name placeholder)")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.trim().is_empty() {
                return Err("Command cannot be empty.");
            }
            if !input.contains("{name}") {
                return Err("Command must contain '{name}' placeholder.");
            }
            Ok(())
        })
        .interact()?;

    store.blueprints.push(Blueprint {
        name: name.clone(),
        command: command.clone(),
    });
    store.save()?;

    println!();
    println!(
        "  {} Saved blueprint '{}' successfully!",
        "✓".green(),
        name.bold()
    );
    println!();

    Ok(())
}

fn list() -> Result<()> {
    let store = BlueprintsStore::load()?;
    println!();
    if store.blueprints.is_empty() {
        println!("  No blueprints saved yet. Run `projm blueprint add` to save one.");
        println!();
        return Ok(());
    }

    println!(
        "  {:<20}  {}",
        "blueprint".bold().underline(),
        "command template".bold().underline(),
    );
    println!("  {}", "─".repeat(70).dimmed());

    for bp in &store.blueprints {
        println!("  {:<20}  {}", bp.name.cyan().bold(), bp.command);
    }
    println!();
    Ok(())
}

fn run_blueprint(name: Option<String>) -> Result<()> {
    use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
    use std::process::Command;

    let theme = ColorfulTheme::default();
    let store = BlueprintsStore::load()?;

    let blueprint = match name {
        Some(ref n) => store.blueprints.iter().find(|b| b.name == *n).cloned(),
        None => {
            if store.blueprints.is_empty() {
                println!();
                println!("  No blueprints saved yet. Run `projm blueprint add` to save one.");
                println!();
                return Ok(());
            }
            let items: Vec<String> = store
                .blueprints
                .iter()
                .map(|b| format!("{}  ({})", b.name.bold(), b.command.dimmed()))
                .collect();

            let selection = Select::with_theme(&theme)
                .with_prompt("Choose a blueprint to run")
                .items(&items)
                .default(0)
                .interact()?;
            Some(store.blueprints[selection].clone())
        }
    };

    let blueprint = match blueprint {
        Some(bp) => bp,
        None => {
            println!("  {} Blueprint not found.", "✗".red());
            return Ok(());
        }
    };

    println!();
    println!("  Running blueprint: {}", blueprint.name.cyan().bold());
    println!();

    let project_name: String = Input::with_theme(&theme)
        .with_prompt("Enter name for your new project")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.trim().is_empty() {
                return Err("Project name cannot be empty.");
            }
            if !input
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                return Err("Project name must be alphanumeric, dashes, or underscores.");
            }
            Ok(())
        })
        .interact()?;

    let resolved_command = blueprint.command.replace("{name}", &project_name);

    println!();
    println!("  Executing command: {}", resolved_command.bold().yellow());
    println!("  {}", "─".repeat(70).dimmed());
    println!();

    // Determine command execution shell
    #[cfg(unix)]
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&resolved_command)
        .spawn()?;

    #[cfg(windows)]
    let mut child = Command::new("cmd")
        .arg("/C")
        .arg(&resolved_command)
        .spawn()?;

    let status = child.wait()?;

    println!();
    println!("  {}", "─".repeat(70).dimmed());

    if !status.success() {
        println!();
        println!("  {} Command failed with exit status: {}", "✗".red(), status);
        println!();
        return Ok(());
    }

    println!();
    println!("  {} Command finished successfully!", "✓".green());
    println!();

    // Check if the directory exists in the CWD
    let new_project_path = std::env::current_dir()?.join(&project_name);
    if new_project_path.exists() && new_project_path.is_dir() {
        let organize = Confirm::with_theme(&theme)
            .with_prompt(format!(
                "Automatically run 'projm organize' on '{}'?",
                project_name.cyan()
            ))
            .default(true)
            .interact()?;

        if organize {
            println!();
            println!("  Organising project '{}'...", project_name);
            match crate::organize::organize_single(&new_project_path) {
                Ok(dest) => {
                    println!(
                        "  {} Successfully organized! → {}",
                        "✓".green(),
                        dest.display().to_string().cyan()
                    );
                }
                Err(e) => {
                    println!("  {} Failed to organize project: {}", "✗".red(), e);
                }
            }
            println!();
        }
    }

    Ok(())
}
