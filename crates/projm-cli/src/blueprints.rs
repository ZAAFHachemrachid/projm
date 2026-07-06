use anyhow::Result;
use colored::Colorize;

use crate::main_cli::BlueprintSubcommands;
use projm_core::blueprints::{Blueprint, BlueprintsStore};
use projm_core::organize;

pub fn run(sub: Option<BlueprintSubcommands>) -> Result<()> {
    match sub {
        Some(BlueprintSubcommands::Add) => add()?,
        Some(BlueprintSubcommands::List) => list()?,
        Some(BlueprintSubcommands::Run { name }) => run_blueprint(name)?,
        Some(BlueprintSubcommands::Edit { name }) => edit(name)?,
        Some(BlueprintSubcommands::Delete { name }) => delete(name)?,
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
        .with_prompt("Command Template (use {name} as optional project name placeholder)")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.trim().is_empty() {
                return Err("Command cannot be empty.");
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

    let has_name_placeholder = blueprint.command.contains("{name}");

    let project_name: Option<String> = if has_name_placeholder {
        let name: String = Input::with_theme(&theme)
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
        Some(name)
    } else {
        None
    };

    let resolved_command = match &project_name {
        Some(name) => blueprint.command.replace("{name}", name),
        None => blueprint.command.clone(),
    };

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
        println!(
            "  {} Command failed with exit status: {}",
            "✗".red(),
            status
        );
        println!();
        return Ok(());
    }

    println!();
    println!("  {} Command finished successfully!", "✓".green());
    println!();

    // Check if the directory exists in the CWD
    let project_name_str = project_name.clone().unwrap_or_default();
    let new_project_path = std::env::current_dir()?.join(&project_name_str);
    if !project_name_str.is_empty() && new_project_path.exists() && new_project_path.is_dir() {
        let organize = Confirm::with_theme(&theme)
            .with_prompt(format!(
                "Automatically run 'projm organize' on '{}'?",
                project_name_str.cyan()
            ))
            .default(true)
            .interact()?;

        if organize {
            println!();
            println!("  Organising project '{}'...", project_name_str);
            match organize::organize_single(&new_project_path) {
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

fn delete(name: Option<String>) -> Result<()> {
    use dialoguer::{theme::ColorfulTheme, Confirm, Select};

    let theme = ColorfulTheme::default();
    let mut store = BlueprintsStore::load()?;

    if store.blueprints.is_empty() {
        println!();
        println!("  No blueprints saved yet. Run `projm blueprint add` to save one.");
        println!();
        return Ok(());
    }

    // Resolve which blueprint to delete
    let blueprint_name = match name {
        Some(n) => {
            if !store.blueprints.iter().any(|b| b.name == n) {
                println!("  {} Blueprint '{}' not found.", "✗".red(), n);
                return Ok(());
            }
            n
        }
        None => {
            let items: Vec<String> = store
                .blueprints
                .iter()
                .map(|b| format!("{}  ({})", b.name.bold(), b.command.dimmed()))
                .collect();

            let selection = Select::with_theme(&theme)
                .with_prompt("Choose a blueprint to delete")
                .items(&items)
                .default(0)
                .interact()?;
            store.blueprints[selection].name.clone()
        }
    };

    println!();
    let ok = Confirm::with_theme(&theme)
        .with_prompt(format!(
            "Are you sure you want to delete blueprint '{}'?",
            blueprint_name.cyan()
        ))
        .default(false)
        .interact()?;

    if ok {
        store.blueprints.retain(|b| b.name != blueprint_name);
        store.save()?;
        println!();
        println!(
            "  {} Blueprint '{}' deleted successfully!",
            "✓".green(),
            blueprint_name.bold()
        );
        println!();
    } else {
        println!();
        println!("  {} Deletion aborted.", "✗".red());
        println!();
    }

    Ok(())
}

fn edit(name: Option<String>) -> Result<()> {
    use dialoguer::{theme::ColorfulTheme, Input, Select};

    let theme = ColorfulTheme::default();
    let mut store = BlueprintsStore::load()?;

    if store.blueprints.is_empty() {
        println!();
        println!("  No blueprints saved yet. Run `projm blueprint add` to save one.");
        println!();
        return Ok(());
    }

    // Resolve which blueprint to edit
    let index = match name {
        Some(ref n) => match store.blueprints.iter().position(|b| b.name == *n) {
            Some(idx) => idx,
            None => {
                println!("  {} Blueprint '{}' not found.", "✗".red(), n);
                return Ok(());
            }
        },
        None => {
            let items: Vec<String> = store
                .blueprints
                .iter()
                .map(|b| format!("{}  ({})", b.name.bold(), b.command.dimmed()))
                .collect();

            Select::with_theme(&theme)
                .with_prompt("Choose a blueprint to edit")
                .items(&items)
                .default(0)
                .interact()?
        }
    };

    let old_name = store.blueprints[index].name.clone();
    let old_command = store.blueprints[index].command.clone();

    println!();
    println!(
        "{}",
        format!("  ✨ Edit Blueprint: {} ✨", old_name.bold()).cyan()
    );
    println!();

    let new_name: String = Input::with_theme(&theme)
        .with_prompt("Blueprint Name")
        .with_initial_text(old_name.clone())
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

    // Check conflict if name changed
    if new_name != old_name && store.blueprints.iter().any(|b| b.name == new_name) {
        println!("  {} Blueprint '{}' already exists.", "✗".red(), new_name);
        return Ok(());
    }

    let new_command: String = Input::with_theme(&theme)
        .with_prompt("Command Template")
        .with_initial_text(old_command)
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.trim().is_empty() {
                return Err("Command cannot be empty.");
            }
            Ok(())
        })
        .interact()?;

    store.blueprints[index].name = new_name.clone();
    store.blueprints[index].command = new_command;
    store.save()?;

    println!();
    println!(
        "  {} Blueprint '{}' updated successfully!",
        "✓".green(),
        new_name.bold()
    );
    println!();

    Ok(())
}
