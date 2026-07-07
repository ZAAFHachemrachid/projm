use crate::main_cli::RulesSubcommands;
use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Select};
use projm_core::classify::{classify_explained, Category, ClassificationSource};
use projm_core::marker::{ProjectMarker, MARKER_FILE};
use projm_core::rules::{load_rules, read_rules_raw, rules_path, CustomRule, RawMatcher};
use projm_core::rules_edit::{
    append_rule, export_rules, import_rules, list_rules, remove_rule, ImportMode, RuleSelector,
};
use std::path::{Path, PathBuf};

pub fn run(sub: RulesSubcommands) -> Result<()> {
    match sub {
        RulesSubcommands::List { json } => list(json),
        RulesSubcommands::Add {
            name,
            name_contains,
            name_glob,
            name_regex,
            suffix,
            parent_dir,
            marker,
            has_dep,
            stack,
            description,
            category,
            at,
        } => {
            let rule = CustomRule {
                matcher: RawMatcher {
                    name,
                    name_contains,
                    name_glob,
                    name_regex,
                    suffix,
                    parent_dir,
                    marker,
                    has_dep,
                    stack,
                    ..Default::default()
                },
                description,
                category,
                ..Default::default()
            };
            let pos = append_rule(&rule, at).map_err(anyhow::Error::msg)?;
            println!(
                "  {} added rule #{} → {}",
                "✓".green().bold(),
                pos,
                Category::from(rule.category.clone()).label().trim_end()
            );
            Ok(())
        }
        RulesSubcommands::Remove { selector } => {
            let removed =
                remove_rule(&RuleSelector::parse(&selector)).map_err(anyhow::Error::msg)?;
            println!(
                "  {} removed rule: {}  → {}",
                "✓".green().bold(),
                summarize_matchers(&removed),
                removed.category
            );
            Ok(())
        }
        RulesSubcommands::Test { path, json } => test(path, json),
        RulesSubcommands::Edit => edit(),
        RulesSubcommands::Export { file } => {
            let content = export_rules(file.as_deref()).map_err(anyhow::Error::msg)?;
            match file {
                Some(f) => println!("  {} exported rules to {}", "✓".green().bold(), f.display()),
                None => print!("{}", content),
            }
            Ok(())
        }
        RulesSubcommands::Import { file, replace } => {
            let mode = if replace {
                ImportMode::Replace
            } else {
                ImportMode::Merge
            };
            let report = import_rules(&file, mode).map_err(anyhow::Error::msg)?;
            if report.replaced {
                println!(
                    "  {} replaced rules file with {} ({} rule{})",
                    "✓".green().bold(),
                    file.display(),
                    report.added,
                    if report.added == 1 { "" } else { "s" }
                );
            } else {
                println!(
                    "  {} imported {} rule{}, skipped {} duplicate{}",
                    "✓".green().bold(),
                    report.added,
                    if report.added == 1 { "" } else { "s" },
                    report.skipped_duplicates,
                    if report.skipped_duplicates == 1 { "" } else { "s" }
                );
            }
            Ok(())
        }
        RulesSubcommands::Pin {
            path,
            category,
            group,
            hidden,
        } => pin(path, category, group, hidden),
        RulesSubcommands::Assign { query } => assign(query),
    }
}

// ── list ──────────────────────────────────────────────────────────────────────

fn list(json: bool) -> Result<()> {
    let rules = list_rules().map_err(anyhow::Error::msg)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&rules)?);
        return Ok(());
    }

    if rules.is_empty() {
        println!(
            "  No rules defined. Add one with `projm rules add` or edit {}",
            rules_path().display()
        );
        return Ok(());
    }

    println!();
    for (i, rule) in rules.iter().enumerate() {
        let cat = Category::from(rule.category.clone());
        let flags = if rule.enabled { "" } else { "  (disabled)" };
        let prio = rule
            .priority
            .map(|p| format!("  priority={}", p))
            .unwrap_or_default();
        println!(
            "  {}  {}  {}{}{}",
            format!("#{:<3}", i + 1).bold(),
            cat.label(),
            summarize_matchers(rule),
            prio.dimmed(),
            flags.yellow()
        );
        if let Some(desc) = &rule.description {
            println!("        {}", desc.dimmed());
        }
    }
    println!();
    println!("  {}", rules_path().display().to_string().dimmed());
    Ok(())
}

fn summarize_matchers(rule: &CustomRule) -> String {
    let m = &rule.matcher;
    let mut parts: Vec<String> = Vec::new();
    let mut push = |label: &str, v: &Option<String>| {
        if let Some(v) = v {
            parts.push(format!("{} = \"{}\"", label, v));
        }
    };
    push("name", &m.name);
    push("name_contains", &m.name_contains);
    push("name_glob", &m.name_glob);
    push("name_regex", &m.name_regex);
    push("suffix", &m.suffix);
    push("parent_dir", &m.parent_dir);
    push("path_glob", &m.path_glob);
    push("marker", &m.marker);
    push("has_dep", &m.has_dep);
    push("stack", &m.stack);
    if let Some(v) = &m.markers {
        parts.push(format!("markers = {:?}", v));
    }
    if let Some(v) = &m.any_marker {
        parts.push(format!("any_marker = {:?}", v));
    }
    if let Some(v) = &m.has_deps {
        parts.push(format!("has_deps = {:?}", v));
    }
    if let Some(dv) = &m.dep_version {
        parts.push(format!("dep_version = {} {}", dv.name, dv.req));
    }
    if let Some(v) = &rule.any_of {
        parts.push(format!("any_of ({} branch{})", v.len(), if v.len() == 1 { "" } else { "es" }));
    }
    if let Some(v) = &rule.none_of {
        parts.push(format!("none_of ({})", v.len()));
    }
    if parts.is_empty() {
        "(matches everything)".to_string()
    } else {
        parts.join(", ")
    }
}

// ── test ──────────────────────────────────────────────────────────────────────

fn test(path: Option<PathBuf>, json: bool) -> Result<()> {
    let path = match path {
        Some(p) => p,
        None => std::env::current_dir()?,
    };
    let path = path
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("cannot resolve {}: {}", path.display(), e))?;
    if !path.is_dir() {
        anyhow::bail!("{} is not a directory", path.display());
    }

    let rules = load_rules();
    let result = classify_explained(&path, &rules);

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());
    println!(
        "\n  {}  →  {}  {}",
        name.bold(),
        result.category.label().trim_end(),
        result.reason().dimmed()
    );
    if matches!(result.source, ClassificationSource::Heuristic(_)) {
        println!(
            "  {} pin it: projm rules pin {} --category {}",
            "hint:".dimmed(),
            path.display(),
            result.category.dir_name()
        );
    }
    println!();
    Ok(())
}

// ── edit ──────────────────────────────────────────────────────────────────────

fn edit() -> Result<()> {
    let original = read_rules_raw().map_err(anyhow::Error::msg)?;
    let path = rules_path();
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    loop {
        let status = std::process::Command::new(&editor).arg(&path).status()?;
        if !status.success() {
            anyhow::bail!("editor exited with an error; rules.toml unchanged");
        }

        let edited = std::fs::read_to_string(&path)?;
        match projm_core::rules::validate_rules_content(&edited) {
            Ok(_) => {
                println!("  {} rules.toml saved and validated", "✓".green().bold());
                return Ok(());
            }
            Err(e) => {
                eprintln!("\n  {} invalid rules file: {}\n", "error:".red().bold(), e);
                let choice = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("rules.toml has errors")
                    .items(["Re-edit", "Keep anyway", "Revert to previous"])
                    .default(0)
                    .interact()?;
                match choice {
                    0 => continue,
                    1 => {
                        eprintln!(
                            "  {} kept invalid file — rules will be ignored until fixed",
                            "warning:".yellow().bold()
                        );
                        return Ok(());
                    }
                    _ => {
                        std::fs::write(&path, &original)?;
                        println!("  {} reverted rules.toml", "✓".green().bold());
                        return Ok(());
                    }
                }
            }
        }
    }
}

// ── pin ───────────────────────────────────────────────────────────────────────

fn pin(path: Option<PathBuf>, category: String, group: Option<String>, hidden: bool) -> Result<()> {
    let dir = match path {
        Some(p) => p,
        None => std::env::current_dir()?,
    };
    if !dir.is_dir() {
        anyhow::bail!("{} is not a directory", dir.display());
    }

    write_pin(&dir, &category, group.as_deref(), hidden)?;
    println!(
        "  {} pinned {} → {} (via {})",
        "✓".green().bold(),
        dir.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| dir.display().to_string())
            .bold(),
        Category::from(category).label().trim_end(),
        MARKER_FILE
    );
    Ok(())
}

fn write_pin(dir: &Path, category: &str, group: Option<&str>, hidden: bool) -> Result<()> {
    // Preserve an existing group/hidden unless explicitly overridden.
    let existing = projm_core::marker::read_marker(dir).unwrap_or_default();
    let marker = ProjectMarker {
        category: Some(category.to_string()),
        group: group.map(|g| g.to_string()).or(existing.group),
        hidden: if hidden { Some(true) } else { existing.hidden },
    };
    projm_core::marker::write_marker(dir, &marker).map_err(anyhow::Error::msg)
}

// ── assign ────────────────────────────────────────────────────────────────────

fn assign(query: Option<String>) -> Result<()> {
    let projects = projm_core::go::collect_projects()?;
    if projects.is_empty() {
        anyhow::bail!("No organized projects found. Run `projm organize <dir>` first.");
    }

    let theme = ColorfulTheme::default();

    // 1. Pick a project
    let labels: Vec<String> = projects
        .iter()
        .map(|p| format!("  {}  {}", p.category.label(), p.name))
        .collect();
    let mut select = FuzzySelect::with_theme(&theme)
        .with_prompt("Assign category to")
        .items(&labels)
        .default(0);
    if let Some(q) = &query {
        select = select.with_initial_text(q);
    }
    let idx = select.interact()?;
    let project = &projects[idx];

    // 2. Pick a category
    let categories = Category::all();
    let cat_labels: Vec<String> = categories.iter().map(|c| c.dir_name().to_string()).collect();
    let current = categories
        .iter()
        .position(|c| *c == project.category)
        .unwrap_or(0);
    let cat_idx = Select::with_theme(&theme)
        .with_prompt(format!("Category for {}", project.name))
        .items(&cat_labels)
        .default(current)
        .interact()?;
    let category = &categories[cat_idx];

    // 3. Pick how to persist it
    let method = Select::with_theme(&theme)
        .with_prompt("Save as")
        .items(&[
            format!("Pin with {} (travels with the repo)", MARKER_FILE),
            "Add global exact-name rule (evaluated first)".to_string(),
        ])
        .default(0)
        .interact()?;

    if method == 0 {
        write_pin(&project.path, category.dir_name(), None, false)?;
        println!(
            "  {} pinned {} → {}",
            "✓".green().bold(),
            project.name.bold(),
            category.label().trim_end()
        );
    } else {
        let rule = CustomRule {
            matcher: RawMatcher {
                name: Some(project.name.clone()),
                ..Default::default()
            },
            category: category.dir_name().to_string(),
            ..Default::default()
        };
        append_rule(&rule, Some(1)).map_err(anyhow::Error::msg)?;
        println!(
            "  {} added rule #1: name = \"{}\" → {}",
            "✓".green().bold(),
            project.name,
            category.label().trim_end()
        );
    }

    // 4. Optionally move the folder now
    if *category != project.category {
        let move_now = Confirm::with_theme(&theme)
            .with_prompt(format!(
                "Move {} to {}/ now?",
                project.name,
                category.dir_name()
            ))
            .default(true)
            .interact()?;
        if move_now {
            let dest = projm_core::organize::organize_single(&project.path)?;
            println!("  {} moved to {}", "✓".green().bold(), dest.display());
        }
    }

    Ok(())
}
