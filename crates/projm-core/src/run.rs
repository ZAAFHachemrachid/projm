use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use colored::Colorize;
use console::Term;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Select};
use serde::Deserialize;

use crate::classify::Category;
use crate::config;


// ── Types ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackageManager {
    Bun,
    Pnpm,
    Yarn,
    Npm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectStack {
    Rust,
    Js { pm: PackageManager },
    Tauri { pm: PackageManager },
    Flutter,
    Go,
    PythonUv,
    Python,
    Rails,
    Elixir,
    Gradle,
    Maven,
    Laravel,
    Cpp,
    Dotnet,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MonorepoTool {
    Turbo,
    PnpmWorkspace,
    Nx,
    BunWorkspace,
}

// ── .projm.toml ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RunConfig {
    run: Option<RunSection>,
}

#[derive(Debug, Deserialize)]
struct RunSection {
    command: Option<String>,
    scripts: Option<Vec<String>>,
    workspace_packages: Option<HashMap<String, String>>,
}

// ── Project info ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Project {
    name: String,
    path: PathBuf,
    category: Category,
}

// ── Public entry ───────────────────────────────────────────────────────────────

pub fn run(path_or_query: Option<String>) -> Result<()> {
    let project = resolve_project(path_or_query.as_deref())?;
    let project_path = &project.path;

    // Walk up looking for monorepo root
    if let Some(mono_root) = find_monorepo_root(project_path) {
        if mono_root == *project_path {
            // At the monorepo root → show workspace picker (or run all)
            let cfg = load_run_config(&mono_root);
            return run_monorepo(&mono_root, &cfg);
        }
        // Inside a specific workspace package → run it directly
        eprintln!("  {} using workspace package at {}", "→".dimmed(), project_path.display());
    }

    // Check for .projm.toml override in the project dir
    let cfg = load_run_config(project_path);

    // Full command override from config
    if let Some(ref section) = cfg.as_ref().and_then(|c| c.run.as_ref()) {
        if let Some(ref cmd) = section.command {
            eprintln!("  {} {} (via .projm.toml)", "→".dimmed(), cmd.dimmed());
            return spawn_and_wait(cmd, project_path);
        }
    }

    // Auto-detect stack and resolve command
    let cmd = resolve_dev_command(project_path)?;

    // Check if a named script was chosen via config (only if more than one)
    if let Some(ref section) = cfg.as_ref().and_then(|c| c.run.as_ref()) {
        if let Some(ref scripts) = section.scripts {
            if scripts.len() > 1 {
                let chosen = pick_script(scripts)?;
                let resolved_cmd = resolve_named_script(&chosen, project_path);
                return spawn_and_wait(&resolved_cmd, project_path);
            }
        }
    }

    eprintln!("  {} {}", "→".dimmed(), cmd.dimmed());
    spawn_and_wait(&cmd, project_path)
}

// ── Project resolution ─────────────────────────────────────────────────────────

fn resolve_project(arg: Option<&str>) -> Result<Project> {
    match arg {
        // Explicit path: `.`, absolute, or existing relative path
        Some(p) if p == "." || Path::new(p).is_absolute() || Path::new(p).exists() => {
            let path = if p == "." {
                std::env::current_dir().context("Failed to get current directory")?
            } else {
                PathBuf::from(p).canonicalize().unwrap_or_else(|_| PathBuf::from(p))
            };
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "project".to_string());
            Ok(Project { name, path, category: Category::Labs })
        }

        // No arg: try CWD, fall back to picker
        None => {
            let cwd = std::env::current_dir().ok();
            if let Some(ref dir) = cwd {
                if is_project_dir(dir) || is_inside_base(dir) {
                    let name = dir
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    return Ok(Project {
                        name,
                        path: dir.clone(),
                        category: Category::Labs,
                    });
                }
            }
            pick_project(None)
        }

        // Name query: try exact match, fall back to picker
        Some(query) => {
            let scanned = scan_projects()?;

            // Exact match (case-insensitive)
            let q_lower = query.to_lowercase();
            let exact: Vec<&Project> =
                scanned.iter().filter(|p| p.name.to_lowercase() == q_lower).collect();

            if exact.len() == 1 {
                return Ok(exact[0].clone());
            }

            pick_project(Some(query))
        }
    }
}

/// Check if a directory has recognisable project markers
fn is_project_dir(dir: &Path) -> bool {
    let markers = [
        "Cargo.toml",
        "package.json",
        "pubspec.yaml",
        "go.mod",
        "pyproject.toml",
        "Gemfile",
        "mix.exs",
        "build.gradle",
        "build.gradle.kts",
        "pom.xml",
        "composer.json",
        "CMakeLists.txt",
    ];
    markers.iter().any(|m| dir.join(m).exists())
}

/// Check if a directory is inside the configured base directory
fn is_inside_base(dir: &Path) -> bool {
    let base = config::load().base;
    dir.starts_with(&base)
}

// ── Project scanning (reuses go.rs approach) ───────────────────────────────────

fn scan_projects() -> Result<Vec<Project>> {
    let base = config::load().base;
    if !base.exists() {
        return Ok(vec![]);
    }

    let mut projects = Vec::new();

    for cat in Category::all() {
        let cat_dir = base.join(cat.dir_name());
        if !cat_dir.exists() {
            continue;
        }

        let mut entries: Vec<_> = std::fs::read_dir(&cat_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.'))
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in &entries {
            if is_group_folder(&entry.path()) {
                let mut children: Vec<_> = std::fs::read_dir(entry.path())?
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.')
                    })
                    .collect();
                children.sort_by_key(|e| e.file_name());
                for child in children {
                    projects.push(Project {
                        name: child.file_name().to_string_lossy().to_string(),
                        path: child.path(),
                        category: cat.clone(),
                    });
                }
            } else {
                projects.push(Project {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry.path(),
                    category: cat.clone(),
                });
            }
        }
    }

    Ok(projects)
}

fn is_group_folder(path: &Path) -> bool {
    let parent_name = match path.file_name() {
        Some(n) => n.to_string_lossy().to_lowercase(),
        None => return false,
    };
    std::fs::read_dir(path)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .any(|e| {
                    let child = e.file_name().to_string_lossy().to_lowercase();
                    child.starts_with(&format!("{}-", parent_name))
                        || child.starts_with(&format!("{}_", parent_name))
                })
        })
        .unwrap_or(false)
}

// ── Interactive picker ─────────────────────────────────────────────────────────

fn pick_project(initial_text: Option<&str>) -> Result<Project> {
    let projects = scan_projects()?;
    if projects.is_empty() {
        anyhow::bail!(
            "No projects found under {}.\n\
             Run `projm organize <dir>` or `projm run <path>` to run a project outside the base.",
            config::load().base.display()
        );
    }

    let max_name_len = projects
        .iter()
        .map(|p| p.name.len())
        .max()
        .unwrap_or(20)
        .max(20);

    let labels: Vec<String> = projects
        .iter()
        .map(|p| {
            format!(
                "  {}  {:<width$}",
                p.category.label(),
                p.name,
                width = max_name_len
            )
        })
        .collect();

    let term = Term::stderr();
    let theme = ColorfulTheme::default();
    let mut select = FuzzySelect::with_theme(&theme)
        .with_prompt("run project")
        .items(&labels);
    if let Some(text) = initial_text {
        select = select.with_initial_text(text);
    }
    let idx = select.interact_on(&term)?;

    Ok(projects[idx].clone())
}

fn pick_script(scripts: &[String]) -> Result<String> {
    let term = Term::stderr();
    let theme = ColorfulTheme::default();
    let idx = FuzzySelect::with_theme(&theme)
        .with_prompt("run script")
        .items(scripts)
        .default(0)
        .interact_on(&term)?;
    Ok(scripts[idx].clone())
}

fn pick_workspace_target(tool: &MonorepoTool, packages: &[String]) -> Result<Option<String>> {
    let term = Term::stderr();

    let mut items: Vec<String> = Vec::with_capacity(packages.len() + 1);
    items.push(format!("{}  {}", "▶".green().bold(), "Run All"));

    let tool_name = match tool {
        MonorepoTool::Turbo => "turbo",
        MonorepoTool::PnpmWorkspace => "pnpm",
        MonorepoTool::Nx => "nx",
        MonorepoTool::BunWorkspace => "bun",
    };

    for pkg in packages {
        items.push(format!(
            "  {}  {}",
            "◻".dimmed(),
            pkg
        ));
    }

    let theme = ColorfulTheme::default();
    let idx = Select::with_theme(&theme)
        .with_prompt(format!("select target ({})", tool_name))
        .items(&items)
        .default(0)
        .interact_on(&term)?;

    if idx == 0 {
        Ok(None) // Run All
    } else {
        Ok(Some(packages[idx - 1].clone()))
    }
}

// ── Stack detection ────────────────────────────────────────────────────────────

fn detect_stack(path: &Path) -> ProjectStack {
    let has = |f: &str| path.join(f).exists();

    // Tauri (Cargo.toml + package.json + src-tauri/)
    if has("src-tauri") && has("Cargo.toml") && has("package.json") {
        return ProjectStack::Tauri {
            pm: detect_package_manager(path),
        };
    }

    // Pure Rust
    if has("Cargo.toml") && !has("package.json") {
        return ProjectStack::Rust;
    }

    // JS/TS (package.json)
    if has("package.json") {
        return ProjectStack::Js {
            pm: detect_package_manager(path),
        };
    }

    // Flutter/Dart
    if has("pubspec.yaml") {
        return ProjectStack::Flutter;
    }

    // Go
    if has("go.mod") {
        return ProjectStack::Go;
    }

    // Python
    if has("uv.lock") || (has("pyproject.toml") && !has("package.json")) {
        return ProjectStack::PythonUv;
    }
    if has("requirements.txt") || has("setup.py") || has("setup.cfg") {
        return ProjectStack::Python;
    }

    // Ruby on Rails
    if has("Gemfile") && path.join("config/routes.rb").exists() {
        return ProjectStack::Rails;
    }

    // Elixir
    if has("mix.exs") {
        return ProjectStack::Elixir;
    }

    // Gradle (Kotlin/Java)
    if has("build.gradle") || has("build.gradle.kts") {
        return ProjectStack::Gradle;
    }

    // Maven
    if has("pom.xml") {
        return ProjectStack::Maven;
    }

    // Laravel/PHP
    if has("composer.json") && has("artisan") {
        return ProjectStack::Laravel;
    }

    // C/C++
    if has("CMakeLists.txt") {
        return ProjectStack::Cpp;
    }

    // C# / .NET
    if has_ext(path, "csproj") || has_ext(path, "sln") {
        return ProjectStack::Dotnet;
    }

    ProjectStack::Unknown
}

fn detect_package_manager(path: &Path) -> PackageManager {
    if path.join("bun.lock").exists() || path.join("bun.lockb").exists() {
        PackageManager::Bun
    } else if path.join("pnpm-lock.yaml").exists() {
        PackageManager::Pnpm
    } else if path.join("yarn.lock").exists() {
        PackageManager::Yarn
    } else {
        PackageManager::Npm
    }
}

fn has_ext(dir: &Path, ext: &str) -> bool {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .any(|e| e.path().extension().map_or(false, |x| x == ext))
        })
        .unwrap_or(false)
}

// ── Dev command resolution ─────────────────────────────────────────────────────

fn resolve_dev_command(path: &Path) -> Result<String> {
    let stack = detect_stack(path);

    match stack {
        ProjectStack::Rust => Ok("cargo run".to_string()),

        ProjectStack::Js { pm } => {
            let script = find_package_script(path, &["dev", "start"]);
            match pm {
                PackageManager::Bun => {
                    let script_name = script.as_deref().unwrap_or("dev");
                    if script.is_some() {
                        Ok(format!("bun run {}", script_name))
                    } else {
                        // Bun can run package.json scripts directly
                        Ok(format!("bun run {}", script_name))
                    }
                }
                PackageManager::Pnpm => {
                    let script_name = script.as_deref().unwrap_or("dev");
                    Ok(format!("pnpm {}", script_name))
                }
                PackageManager::Yarn => {
                    let script_name = script.as_deref().unwrap_or("dev");
                    Ok(format!("yarn {}", script_name))
                }
                PackageManager::Npm => {
                    let script_name = script.as_deref().unwrap_or("dev");
                    Ok(format!("npm run {}", script_name))
                }
            }
        }

        ProjectStack::Tauri { pm } => {
            let manager_cmd = match pm {
                PackageManager::Bun => "bun",
                PackageManager::Pnpm => "pnpm",
                PackageManager::Yarn => "yarn",
                PackageManager::Npm => "npm",
            };
            Ok(format!("{} tauri dev", manager_cmd))
        }

        ProjectStack::Flutter => Ok("flutter run".to_string()),

        ProjectStack::Go => Ok("go run .".to_string()),

        ProjectStack::PythonUv => {
            let entry = find_python_entry(path);
            Ok(format!("uv run python {}", entry))
        }

        ProjectStack::Python => {
            let entry = find_python_entry(path);
            let python = if Path::new("python3").exists() {
                "python3"
            } else {
                "python"
            };
            Ok(format!("{} {}", python, entry))
        }

        ProjectStack::Rails => Ok("bin/rails server".to_string()),

        ProjectStack::Elixir => Ok("mix run --no-halt".to_string()),

        ProjectStack::Gradle => Ok("./gradlew run".to_string()),

        ProjectStack::Maven => Ok("mvn compile exec:java".to_string()),

        ProjectStack::Laravel => Ok("php artisan serve".to_string()),

        ProjectStack::Cpp => Ok("cmake --build build".to_string()),

        ProjectStack::Dotnet => Ok("dotnet run".to_string()),

        ProjectStack::Unknown => {
            anyhow::bail!(
                "Could not auto-detect the project stack for {}.\n\
                 Add a .projm.toml file to the project root with a [run] command:\n\
                 \n\
                 [run]\n\
                 command = \"your dev command here\"\n",
                path.display()
            );
        }
    }
}

/// Read package.json and find the first existing script from candidates
fn find_package_script(path: &Path, candidates: &[&str]) -> Option<String> {
    let pkg_path = path.join("package.json");
    let content = std::fs::read_to_string(pkg_path).ok()?;

    // Simple scan for "scripts" block
    let scripts_start = content.find("\"scripts\"")?;
    let brace = content[scripts_start..].find('{')?;
    let start = scripts_start + brace + 1;

    // Find matching brace
    let mut depth = 1u32;
    let mut end = start;
    for (i, b) in content.as_bytes()[start..].iter().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i;
                    break;
                }
            }
            _ => {}
        }
    }

    let block = &content[start..end];

    for candidate in candidates {
        // Look for `"candidate": ` in the scripts block
        let search = &format!("\"{}\"", candidate);
        if let Some(pos) = block.find(search) {
            // Check it's followed by `:` (it's a key, not a value)
            let after = block[pos + search.len()..].trim_start();
            if after.starts_with(':') {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

/// Find Python entry point
fn find_python_entry(path: &Path) -> String {
    let candidates = ["main.py", "app.py", "cli.py", "__main__.py"];
    for c in &candidates {
        if path.join(c).exists() {
            return c.to_string();
        }
    }
    // Try pyproject.toml [project.scripts] or [tool.poetry.scripts]
    if let Ok(content) = std::fs::read_to_string(path.join("pyproject.toml")) {
        if let Some(cmd) = extract_first_script_entry(&content) {
            return cmd;
        }
    }
    "main.py".to_string()
}

/// Extract the first entry from [project.scripts] or [tool.poetry.scripts] in pyproject.toml
fn extract_first_script_entry(content: &str) -> Option<String> {
    for section in &["[project.scripts]", "[tool.poetry.scripts]"] {
        if let Some(pos) = content.find(section) {
            let rest = &content[pos + section.len()..];
            for line in rest.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('[') {
                    break;
                }
                // entry format: "name = module:function"
                if let Some(eq) = trimmed.find('=') {
                    let entry = trimmed[eq + 1..].trim();
                    // Extract just the module part
                    if let Some(colon) = entry.find(':') {
                        let module = entry[..colon].trim();
                        if !module.is_empty() {
                            return Some(format!("python -m {}", module));
                        }
                    }
                }
            }
        }
    }
    None
}

fn resolve_named_script(name: &str, path: &Path) -> String {
    let stack = detect_stack(path);
    match stack {
        ProjectStack::Js { pm } | ProjectStack::Tauri { pm } => match pm {
            PackageManager::Bun => format!("bun run {}", name),
            PackageManager::Pnpm => format!("pnpm {}", name),
            PackageManager::Yarn => format!("yarn {}", name),
            PackageManager::Npm => format!("npm run {}", name),
        },
        ProjectStack::Rust => format!("cargo {}", name),
        _ => name.to_string(),
    }
}

// ── Monorepo support ───────────────────────────────────────────────────────────

fn find_monorepo_root(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path.to_path_buf());
    while let Some(dir) = current {
        if detect_monorepo(&dir).is_some() {
            return Some(dir);
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }
    None
}

fn detect_monorepo(path: &Path) -> Option<MonorepoTool> {
    if path.join("turbo.json").exists() {
        return Some(MonorepoTool::Turbo);
    }
    if path.join("pnpm-workspace.yaml").exists() {
        return Some(MonorepoTool::PnpmWorkspace);
    }
    if path.join("nx.json").exists() {
        return Some(MonorepoTool::Nx);
    }
    // Bun workspace: bun.lock + package.json workspaces
    if (path.join("bun.lock").exists() || path.join("bun.lockb").exists())
        && has_workspaces(path)
    {
        return Some(MonorepoTool::BunWorkspace);
    }
    // pnpm/yarn workspace with workspaces field but no marker file
    // (covered by PnpmWorkspace above which checks pnpm-workspace.yaml)
    None
}

fn has_workspaces(path: &Path) -> bool {
    let pkg = path.join("package.json");
    let content = match std::fs::read_to_string(pkg) {
        Ok(c) => c,
        Err(_) => return false,
    };
    content.contains("\"workspaces\"")
}

fn resolve_workspace_packages(path: &Path, tool: &MonorepoTool) -> Vec<String> {
    match tool {
        MonorepoTool::PnpmWorkspace => resolve_pnpm_workspace_packages(path),
        MonorepoTool::BunWorkspace => resolve_bun_workspace_packages(path),
        // Turbo and Nx: return empty (use .projm.toml for packages)
        MonorepoTool::Turbo | MonorepoTool::Nx => Vec::new(),
    }
}

fn resolve_pnpm_workspace_packages(root: &Path) -> Vec<String> {
    let yaml_path = root.join("pnpm-workspace.yaml");
    let content = match std::fs::read_to_string(yaml_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    // Extract packages globs from `packages:` list
    let mut globs: Vec<String> = Vec::new();
    let mut in_packages = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("packages:") {
            in_packages = true;
            continue;
        }
        if in_packages {
            if trimmed.starts_with('-') {
                let glob = trimmed.trim_start_matches('-').trim().trim_matches('\'')
                    .trim_matches('"')
                    .to_string();
                if !glob.is_empty() {
                    globs.push(glob);
                }
            } else if trimmed.starts_with('#') {
                continue;
            } else if !trimmed.starts_with('-') && !trimmed.is_empty() {
                // End of packages section (another key)
                break;
            }
        }
    }

    // Resolve globs to actual directories
    let mut packages = Vec::new();
    for glob in &globs {
        resolve_glob(root, glob, &mut packages);
    }
    packages.sort();
    packages.dedup();
    packages
}

fn resolve_bun_workspace_packages(root: &Path) -> Vec<String> {
    let pkg = root.join("package.json");
    let content = match std::fs::read_to_string(pkg) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    // Find "workspaces" array
    let ws_start = match content.find("\"workspaces\"") {
        Some(i) => i,
        None => return Vec::new(),
    };
    let rest = &content[ws_start + 12..]; // skip past "workspaces"
    let brace = match rest.find('[') {
        Some(i) => i,
        None => return Vec::new(),
    };
    let list_start = brace + 1;
    let mut depth = 1u32;
    let mut list_end = list_start;
    for (i, b) in rest.as_bytes()[list_start..].iter().enumerate() {
        match b {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    list_end = list_start + i;
                    break;
                }
            }
            _ => {}
        }
    }

    let list_str = &rest[list_start..list_end];
    let mut packages = Vec::new();
    for entry in list_str.split(',') {
        let trimmed = entry.trim().trim_matches('"').trim_matches('\'');
        if !trimmed.is_empty() {
            resolve_glob(root, trimmed, &mut packages);
        }
    }
    packages.sort();
    packages.dedup();
    packages
}

/// Resolve a glob like "apps/*" or "packages/*" to actual subdirectory names
fn resolve_glob(root: &Path, glob: &str, out: &mut Vec<String>) {
    // Simple glob: only handle "parent/*" and "parent/**" patterns
    if let Some(star_pos) = glob.find('*') {
        let prefix = &glob[..star_pos];
        // If no trailing slash before *, the glob is like "apps*" - not supported simply
        if prefix.is_empty() || !prefix.ends_with('/') {
            return;
        }
        let prefix = prefix.trim_end_matches('/');
        let dir = root.join(prefix);
        if !dir.is_dir() {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !name.starts_with('.') {
                        out.push(format!("{}/{}", prefix, name));
                    }
                }
            }
        }
    } else {
        // Literal path
        let dir = root.join(glob);
        if dir.is_dir() {
            if let Some(name) = dir.file_name() {
                out.push(name.to_string_lossy().to_string());
            }
        }
    }
}

fn run_monorepo(root: &Path, cfg: &Option<RunConfig>) -> Result<()> {
    let tool = detect_monorepo(root).expect("called run_monorepo without monorepo tool");
    let tool_name = match tool {
        MonorepoTool::Turbo => "turbo",
        MonorepoTool::PnpmWorkspace => "pnpm",
        MonorepoTool::Nx => "nx",
        MonorepoTool::BunWorkspace => "bun",
    };

    // Get workspace packages from config or auto-detect
    let config_packages = cfg
        .as_ref()
        .and_then(|c| c.run.as_ref())
        .and_then(|r| r.workspace_packages.as_ref())
        .cloned()
        .unwrap_or_default();

    let auto_packages = resolve_workspace_packages(root, &tool);

    // Merge: config overrides auto-detected
    let mut package_names: Vec<String> = if !config_packages.is_empty() {
        config_packages.keys().cloned().collect()
    } else {
        auto_packages
            .iter()
            .map(|p| {
                // Use last component as display name
                p.split('/').last().unwrap_or(p).to_string()
            })
            .collect()
    };
    package_names.sort();

    if package_names.is_empty() {
        // No packages - just run the dev command at root
        eprintln!("  {} {} workspaces detected — running root dev command", "→".dimmed(), tool_name);
        let root_cmd = resolve_monorepo_root_command(root, &tool, None)?;
        return spawn_and_wait(&root_cmd, root);
    }

    // Show picker
    let selected = pick_workspace_target(&tool, &package_names)?;

    match selected {
        None => {
            // Run All
            let cmd = resolve_monorepo_root_command(root, &tool, None)?;
            eprintln!("  {} running all packages: {}", "▶".green(), cmd.dimmed());
            spawn_and_wait(&cmd, root)
        }
        Some(pkg) => {
            // Find the path for this package
            let pkg_path = config_packages
                .get(&pkg)
                .map(|p| root.join(p))
                .or_else(|| {
                    // Try auto-detected: find first match
                    let matches: Vec<&String> =
                        auto_packages.iter().filter(|p| p.ends_with(&pkg)).collect();
                    matches.first().map(|p| root.join(p))
                });

            let cmd = resolve_monorepo_root_command(root, &tool, Some(&pkg))?;
            match &pkg_path {
                Some(path) if path.exists() => {
                    eprintln!("  {} {}: {}", "▶".green(), pkg, cmd.dimmed());
                    spawn_and_wait(&cmd, root)
                }
                _ => {
                    eprintln!("  {} {}: {}", "▶".green(), pkg, cmd.dimmed());
                    spawn_and_wait(&cmd, root)
                }
            }
        }
    }
}

fn resolve_monorepo_root_command(
    root: &Path,
    tool: &MonorepoTool,
    package: Option<&str>,
) -> Result<String> {
    // Check for .projm.toml command override first
    if let Some(cfg) = load_run_config(root) {
        if let Some(section) = &cfg.run {
            if let Some(cmd) = &section.command {
                return Ok(cmd.clone());
            }
        }
    }

    // Check for a script override for the named script
    if let Some(cfg) = load_run_config(root) {
        if let Some(section) = &cfg.run {
            if let Some(scripts) = &section.scripts {
                if scripts.len() == 1 {
                    let cmd = resolve_named_script(&scripts[0], root);
                    return Ok(if let Some(pkg) = package {
                        format!("{} --filter={}", cmd, pkg)
                    } else {
                        cmd
                    });
                }
            }
        }
    }

    let cmd = match tool {
        MonorepoTool::Turbo => {
            if let Some(pkg) = package {
                format!("turbo run dev --filter={}", pkg)
            } else {
                "turbo run dev".to_string()
            }
        }
        MonorepoTool::PnpmWorkspace => {
            if let Some(pkg) = package {
                format!("pnpm --filter {} dev", pkg)
            } else {
                "pnpm -r dev".to_string()
            }
        }
        MonorepoTool::Nx => {
            if let Some(pkg) = package {
                format!("nx run {}:dev", pkg)
            } else {
                "nx run-many --target=dev".to_string()
            }
        }
        MonorepoTool::BunWorkspace => {
            if let Some(pkg) = package {
                format!("bun run --filter {} dev", pkg)
            } else {
                "bun run dev".to_string()
            }
        }
    };

    // If the tool binary isn't on PATH, prefix with the package manager
    // so e.g. `turbo run dev` becomes `pnpm turbo run dev`.
    let tool_binary = match tool {
        MonorepoTool::Turbo => "turbo",
        MonorepoTool::Nx => "nx",
        // pnpm and bun ARE the package manager — no prefix needed
        MonorepoTool::PnpmWorkspace | MonorepoTool::BunWorkspace => return Ok(cmd),
    };

    Ok(wrap_tool_command(root, tool_binary, &cmd))
}

/// If `tool_binary` isn't directly on PATH, prefix with the project's package
/// manager so e.g. `turbo run dev` → `pnpm turbo run dev` or `npx turbo run dev`.
fn wrap_tool_command(root: &Path, tool_binary: &str, cmd: &str) -> String {
    if has_binary(tool_binary) {
        return cmd.to_string();
    }
    let pm = detect_package_manager(root);
    match pm {
        PackageManager::Bun => format!("bun x {}", cmd),
        PackageManager::Pnpm => format!("pnpm {}", cmd),
        PackageManager::Yarn => format!("yarn {}", cmd),
        PackageManager::Npm => format!("npx {}", cmd),
    }
}

fn has_binary(name: &str) -> bool {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .any(|p| p.join(name).exists())
}

// ── .projm.toml loading ────────────────────────────────────────────────────────

fn load_run_config(path: &Path) -> Option<RunConfig> {
    let config_path = path.join(".projm.toml");
    if !config_path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(config_path).ok()?;
    toml::from_str(&content).ok()
}

// ── Process spawning ───────────────────────────────────────────────────────────

fn spawn_and_wait(command: &str, cwd: &Path) -> Result<()> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("Failed to run: {}", command))?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        std::process::exit(code);
    }

    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_proj(files: &[&str]) -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        for f in files {
            let path = dir.path().join(f);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&path, "").unwrap();
        }
        dir
    }

    #[test]
    fn detect_rust_stack() {
        let d = tmp_proj(&["Cargo.toml"]);
        assert_eq!(detect_stack(d.path()), ProjectStack::Rust);
    }

    #[test]
    fn detect_bun_stack() {
        let d = tmp_proj(&["package.json", "bun.lock"]);
        assert_eq!(
            detect_stack(d.path()),
            ProjectStack::Js {
                pm: PackageManager::Bun
            }
        );
    }

    #[test]
    fn detect_pnpm_stack() {
        let d = tmp_proj(&["package.json", "pnpm-lock.yaml"]);
        assert_eq!(
            detect_stack(d.path()),
            ProjectStack::Js {
                pm: PackageManager::Pnpm
            }
        );
    }

    #[test]
    fn detect_yarn_stack() {
        let d = tmp_proj(&["package.json", "yarn.lock"]);
        assert_eq!(
            detect_stack(d.path()),
            ProjectStack::Js {
                pm: PackageManager::Yarn
            }
        );
    }

    #[test]
    fn detect_npm_stack() {
        let d = tmp_proj(&["package.json", "package-lock.json"]);
        assert_eq!(
            detect_stack(d.path()),
            ProjectStack::Js {
                pm: PackageManager::Npm
            }
        );
    }

    #[test]
    fn detect_tauri_stack() {
        let d = tmp_proj(&["Cargo.toml", "package.json", "src-tauri/main.rs"]);
        assert_eq!(
            detect_stack(d.path()),
            ProjectStack::Tauri {
                pm: PackageManager::Npm
            }
        );
    }

    #[test]
    fn detect_go_stack() {
        let d = tmp_proj(&["go.mod"]);
        assert_eq!(detect_stack(d.path()), ProjectStack::Go);
    }

    #[test]
    fn detect_flutter_stack() {
        let d = tmp_proj(&["pubspec.yaml"]);
        assert_eq!(detect_stack(d.path()), ProjectStack::Flutter);
    }

    #[test]
    fn detect_python_uv_stack() {
        let d = tmp_proj(&["uv.lock", "pyproject.toml"]);
        assert_eq!(detect_stack(d.path()), ProjectStack::PythonUv);
    }

    #[test]
    fn detect_rails_stack() {
        let d = tmp_proj(&["Gemfile", "config/routes.rb"]);
        assert_eq!(detect_stack(d.path()), ProjectStack::Rails);
    }

    #[test]
    fn detect_elixir_stack() {
        let d = tmp_proj(&["mix.exs"]);
        assert_eq!(detect_stack(d.path()), ProjectStack::Elixir);
    }

    #[test]
    fn detect_gradle_stack() {
        let d = tmp_proj(&["build.gradle"]);
        assert_eq!(detect_stack(d.path()), ProjectStack::Gradle);
    }

    #[test]
    fn detect_maven_stack() {
        let d = tmp_proj(&["pom.xml"]);
        assert_eq!(detect_stack(d.path()), ProjectStack::Maven);
    }

    #[test]
    fn detect_laravel_stack() {
        let d = tmp_proj(&["composer.json", "artisan"]);
        assert_eq!(detect_stack(d.path()), ProjectStack::Laravel);
    }

    #[test]
    fn detect_cpp_stack() {
        let d = tmp_proj(&["CMakeLists.txt"]);
        assert_eq!(detect_stack(d.path()), ProjectStack::Cpp);
    }

    #[test]
    fn detect_dotnet_stack() {
        let d = tmp_proj(&["MyApp.csproj"]);
        assert_eq!(detect_stack(d.path()), ProjectStack::Dotnet);
    }

    #[test]
    fn detect_unknown_stack() {
        let d = tmp_proj(&["README.md", ".gitignore"]);
        assert_eq!(detect_stack(d.path()), ProjectStack::Unknown);
    }

    #[test]
    fn rust_dev_command() {
        let d = tmp_proj(&["Cargo.toml"]);
        assert_eq!(resolve_dev_command(d.path()).unwrap(), "cargo run");
    }

    #[test]
    fn bun_dev_command() {
        let d = tmp_proj(&["package.json", "bun.lock"]);
        let cmd = resolve_dev_command(d.path()).unwrap();
        assert!(cmd.starts_with("bun run"));
        assert!(cmd.contains("dev"));
    }

    #[test]
    fn pnpm_dev_command() {
        let d = tmp_proj(&["package.json", "pnpm-lock.yaml"]);
        let cmd = resolve_dev_command(d.path()).unwrap();
        assert_eq!(cmd, "pnpm dev");
    }

    #[test]
    fn go_dev_command() {
        let d = tmp_proj(&["go.mod"]);
        assert_eq!(resolve_dev_command(d.path()).unwrap(), "go run .");
    }

    #[test]
    fn flutter_dev_command() {
        let d = tmp_proj(&["pubspec.yaml"]);
        assert_eq!(resolve_dev_command(d.path()).unwrap(), "flutter run");
    }

    #[test]
    fn python_uv_dev_command() {
        let d = tmp_proj(&["uv.lock", "pyproject.toml", "main.py"]);
        let cmd = resolve_dev_command(d.path()).unwrap();
        assert_eq!(cmd, "uv run python main.py");
    }

    #[test]
    fn find_package_script_dev() {
        let d = tmp_proj(&["package.json"]);
        std::fs::write(
            d.path().join("package.json"),
            r#"{"scripts": {"dev": "vite", "build": "vite build"}}"#,
        )
            .unwrap();
        assert_eq!(find_package_script(d.path(), &["dev", "start"]).as_deref(), Some("dev"));
    }

    #[test]
    fn find_package_script_start_fallback() {
        let d = tmp_proj(&["package.json"]);
        std::fs::write(
            d.path().join("package.json"),
            r#"{"scripts": {"start": "node server.js"}}"#,
        )
            .unwrap();
        assert_eq!(find_package_script(d.path(), &["dev", "start"]).as_deref(), Some("start"));
    }

    #[test]
    fn find_python_entry_main() {
        let d = tmp_proj(&["main.py", "app.py"]);
        assert_eq!(find_python_entry(d.path()), "main.py");
    }

    #[test]
    fn find_python_entry_app() {
        let d = tmp_proj(&["app.py"]);
        assert_eq!(find_python_entry(d.path()), "app.py");
    }

    #[test]
    fn find_python_entry_fallback() {
        let d = tmp_proj(&["README.md"]);
        assert_eq!(find_python_entry(d.path()), "main.py");
    }

    #[test]
    fn detect_monorepo_turbo() {
        let d = tmp_proj(&["turbo.json", "package.json"]);
        assert_eq!(detect_monorepo(d.path()), Some(MonorepoTool::Turbo));
    }

    #[test]
    fn detect_monorepo_pnpm() {
        let d = tmp_proj(&["pnpm-workspace.yaml"]);
        assert_eq!(detect_monorepo(d.path()), Some(MonorepoTool::PnpmWorkspace));
    }

    #[test]
    fn detect_monorepo_nx() {
        let d = tmp_proj(&["nx.json", "package.json"]);
        assert_eq!(detect_monorepo(d.path()), Some(MonorepoTool::Nx));
    }

    #[test]
    fn detect_monorepo_bun() {
        let d = tmp_proj(&["bun.lock", "package.json"]);
        std::fs::write(
            d.path().join("package.json"),
            r#"{"workspaces": ["apps/*", "packages/*"]}"#,
        )
            .unwrap();
        assert_eq!(detect_monorepo(d.path()), Some(MonorepoTool::BunWorkspace));
    }

    #[test]
    fn no_monorepo_detection_for_plain_project() {
        let d = tmp_proj(&["Cargo.toml"]);
        assert_eq!(detect_monorepo(d.path()), None);
    }

    #[test]
    fn resolve_pnpm_workspace_globs() {
        let d = tmp_proj(&["pnpm-workspace.yaml", "apps/web/package.json", "apps/api/package.json"]);
        std::fs::write(
            d.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'apps/*'\n  - 'packages/*'\n",
        )
            .unwrap();
        let packages = resolve_pnpm_workspace_packages(d.path());
        assert!(packages.contains(&"apps/web".to_string()));
        assert!(packages.contains(&"apps/api".to_string()));
    }

    #[test]
    fn resolve_bun_workspace_globs() {
        let d = tmp_proj(&["package.json", "packages/shared/package.json", "packages/ui/package.json"]);
        std::fs::write(
            d.path().join("package.json"),
            r#"{"workspaces": ["packages/*"]}"#,
        )
            .unwrap();
        let packages = resolve_bun_workspace_packages(d.path());
        assert!(packages.contains(&"packages/shared".to_string()));
        assert!(packages.contains(&"packages/ui".to_string()));
    }

    #[test]
    fn load_run_config_with_command() {
        let d = tmp_proj(&[".projm.toml"]);
        std::fs::write(
            d.path().join(".projm.toml"),
            "[run]\ncommand = \"cargo run --release\"\n",
        )
            .unwrap();
        let cfg = load_run_config(d.path()).unwrap();
        assert_eq!(
            cfg.run.unwrap().command.unwrap(),
            "cargo run --release"
        );
    }

    #[test]
    fn load_run_config_with_scripts() {
        let d = tmp_proj(&[".projm.toml"]);
        std::fs::write(
            d.path().join(".projm.toml"),
            "[run]\nscripts = [\"dev\", \"build\", \"test\"]\n",
        )
            .unwrap();
        let cfg = load_run_config(d.path()).unwrap();
        let scripts = cfg.run.unwrap().scripts.unwrap();
        assert_eq!(scripts, vec!["dev", "build", "test"]);
    }

    #[test]
    fn load_run_config_with_workspace_packages() {
        let d = tmp_proj(&[".projm.toml"]);
        std::fs::write(
            d.path().join(".projm.toml"),
            "[run.workspace_packages]\napi = \"apps/api\"\nweb = \"apps/web\"\n",
        )
            .unwrap();
        let cfg = load_run_config(d.path()).unwrap();
        let wp = cfg.run.unwrap().workspace_packages.unwrap();
        assert_eq!(wp.get("api").unwrap(), "apps/api");
        assert_eq!(wp.get("web").unwrap(), "apps/web");
    }

    #[test]
    fn is_project_dir_with_cargo() {
        let d = tmp_proj(&["Cargo.toml"]);
        assert!(is_project_dir(d.path()));
    }

    #[test]
    fn is_project_dir_with_package_json() {
        let d = tmp_proj(&["package.json"]);
        assert!(is_project_dir(d.path()));
    }

    #[test]
    fn is_project_dir_empty() {
        let d = tmp_proj(&[]);
        assert!(!is_project_dir(d.path()));
    }
}
