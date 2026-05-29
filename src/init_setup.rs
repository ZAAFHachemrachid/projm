use anyhow::{Context, Result};
use colored::Colorize;
use std::{fs, path::PathBuf, process::Command};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select, FuzzySelect};

use crate::{completions, config, editors};

const PROJM_BLOCK_START: &str = "# >>> projm >>>";
const PROJM_BLOCK_END: &str = "# <<< projm <<<";
const ZOXIDE_INIT_ZSH: &str = "eval \"$(zoxide init zsh)\"";
const ZOXIDE_INIT_POWERSHELL: &str = "Invoke-Expression (& { (zoxide init powershell | Out-String) })";

#[derive(Clone, Copy)]
enum InitTarget {
    Zsh,
    PowerShell,
}

pub fn run(alias: &str, non_interactive: bool) -> Result<()> {
    let target = detect_target();

    // Create default rules.toml if not already present
    crate::rules::init_default_rules()?;

    let is_interactive = !non_interactive && console::user_attended();

    if is_interactive {
        run_wizard(alias, target)?;
    } else {
        run_non_interactive(alias, target)?;
    }

    Ok(())
}

fn run_non_interactive(alias: &str, target: InitTarget) -> Result<()> {
    eprintln!("[1/3] checking zoxide...");

    if has_zoxide() {
        eprintln!("      {} already installed", "✓".green().bold());
    } else {
        eprintln!("      not found");
        install_zoxide()?;
    }

    eprintln!("[2/3] writing completions...");
    let completion_file = completion_path(target);
    write_completions(target, &completion_file)?;
    eprintln!(
        "      {} {}",
        "✓".green().bold(),
        completion_file.display().to_string().dimmed()
    );

    eprintln!("[3/3] updating shell profile...");
    let profile = update_shell_profile(target, alias)?;
    eprintln!("\n  {} updated {}", "done.".green().bold(), profile.display());

    Ok(())
}

fn run_wizard(alias: &str, target: InitTarget) -> Result<()> {
    println!();
    println!("{}", "  ┌────────────────────────────────────────────────────────┐".cyan());
    println!("{}", "  │  🚀 Welcome to projm                                   │".cyan().bold());
    println!("{}", "  │  The developer-first project organizer & navigator.    │".cyan());
    println!("{}", "  └────────────────────────────────────────────────────────┘".cyan());
    println!();
    println!("  Let's configure your development environment. This wizard will guide you through:");
    println!("    ⚙️  Setting your base directory");
    println!("    📝 Picking your preferred editor");
    println!("    🐚 Setting up shell completions & the fuzzy-jump alias");
    println!("    🎮 Running an interactive 1-minute sandbox showcase");
    println!();

    let theme = ColorfulTheme::default();

    // 1. Configure Base Directory
    let current_base = config::load().base;
    let default_base_str = current_base.to_string_lossy().to_string();
    let base_input: String = Input::with_theme(&theme)
        .with_prompt("Where would you like to store your organized projects?")
        .default(default_base_str)
        .interact_text()?;

    let base_path = PathBuf::from(&base_input);
    if base_path != current_base {
        config::set_base(&base_path)?;
    }
    println!();

    // 2. Select Preferred Editor
    let installed = editors::detect_installed();
    let mut selected_editor = String::new();

    if installed.is_empty() {
        println!("  {}", "No supported editors detected on your $PATH.".yellow());
        let manual_entry: String = Input::with_theme(&theme)
            .with_prompt("Please enter your editor command (e.g. nvim, code, helix) or press Enter to skip:")
            .default("".to_string())
            .interact_text()?;
        if !manual_entry.is_empty() {
            selected_editor = manual_entry;
            println!("  Default editor set to: {}", selected_editor.bold());
        }
    } else {
        println!("  Detected the following editors installed on your machine:");
        let labels: Vec<String> = installed.iter().map(|e| format!("  {} ({})", e.name, e.binary)).collect();
        let chosen = Select::with_theme(&theme)
            .with_prompt("Choose your preferred editor")
            .items(&labels)
            .default(0)
            .interact()?;
        selected_editor = installed[chosen].binary.to_owned();
        println!("  {} Preferred editor set to: {}", "✓".green(), selected_editor.bold());
    }
    println!();

    // 3. Configure Alias
    let chosen_alias: String = Input::with_theme(&theme)
        .with_prompt("What alias would you like to use for fuzzy-navigation?")
        .default(alias.to_string())
        .interact_text()?;
    println!();

    // 4. Check & Install Zoxide
    println!("{}", "[1/3] checking zoxide...".bold());
    if has_zoxide() {
        println!("      {} already installed", "✓".green().bold());
    } else {
        println!("      zoxide not found. It is highly recommended for project navigation.");
        let install_z = Confirm::with_theme(&theme)
            .with_prompt("Would you like to try installing zoxide automatically now?")
            .default(true)
            .interact()?;
        if install_z {
            if let Err(e) = install_zoxide() {
                println!("      {} {}", "✗".red().bold(), e);
            }
        }
    }
    println!();

    // 5. Setup completions & shell profile
    println!("{}", "[2/3] writing completions...".bold());
    let completion_file = completion_path(target);
    write_completions(target, &completion_file)?;
    println!(
        "      {} {}",
        "✓".green().bold(),
        completion_file.display().to_string().dimmed()
    );
    println!();

    println!("{}", "[3/3] updating shell profile...".bold());
    let profile = update_shell_profile(target, &chosen_alias)?;
    println!("      {} updated {}", "✓".green().bold(), profile.display());
    println!();

    println!("{}", "🎉 Configuration successfully complete!".green().bold());
    println!();

    // 6. Interactive sandbox demo!
    let run_demo = Confirm::with_theme(&theme)
        .with_prompt("Would you like to run a quick 1-minute sandbox demo of projm?")
        .default(true)
        .interact()?;

    if run_demo {
        run_sandbox_demo(&selected_editor)?;
    }

    println!();
    println!("  To start using projm, restart your shell or run:");
    println!("  {}", format!("source {}", profile.display()).cyan());
    println!();

    Ok(())
}

fn run_sandbox_demo(preferred_editor: &str) -> Result<()> {
    println!();
    println!("{}", "🎮 Starting Sandbox Demo...".cyan().bold());
    println!("  We will scaffold a temporary workspace to show how projm classifies and navigates projects.");

    // Create temp directory
    let temp_dir = tempfile::tempdir()?;
    let sandbox_path = temp_dir.path();
    let source_dump = sandbox_path.join("source-dump");
    let demo_base = sandbox_path.join("demo-base");

    fs::create_dir_all(&source_dump)?;
    fs::create_dir_all(&demo_base)?;

    // 1. Scaffold mock projects
    let rust_proj = source_dump.join("rust-telemetry-server");
    let react_proj = source_dump.join("react-dashboard-ui");
    let python_proj = source_dump.join("python-ml-model");

    fs::create_dir_all(&rust_proj)?;
    fs::create_dir_all(&react_proj)?;
    fs::create_dir_all(&python_proj)?;

    fs::write(rust_proj.join("Cargo.toml"), r#"[package]
name = "rust-telemetry-server"
version = "0.1.0"
[dependencies]
tokio = "1"
"#)?;

    fs::write(react_proj.join("package.json"), r#"{
  "name": "react-dashboard-ui",
  "dependencies": {
    "react": "^18.0.0",
    "vite": "^4.0.0"
  }
}"#)?;

    fs::write(python_proj.join("pyproject.toml"), r#"[project]
name = "python-ml-model"
dependencies = [
    "torch>=2.0"
]
"#)?;

    println!();
    println!("  ⚡ {}", "Step 1: Automatic Classification & Organization".bold());
    println!("  Scaffolded 3 unorganized mock projects in a temporary dump folder:");
    println!("    📁 rust-telemetry-server/ (contains Cargo.toml)");
    println!("    📁 react-dashboard-ui/    (contains package.json)");
    println!("    📁 python-ml-model/        (contains pyproject.toml)");
    println!();
    println!("  Running 'projm organize' to scan, classify, and group them into 'demo-base/'...");
    println!();

    // Run our custom run_with_base on the sandbox
    crate::organize::run_with_base(&source_dump, &demo_base, false)?;

    println!();
    println!("  ⚡ {}", "Step 2: Fuzzy Navigation Showcase".bold());
    println!("  Now let's test how you will navigate between them.");
    println!("  We will open a mini-version of our fuzzy navigator.");
    println!("  Use your Arrow Keys to select, or start typing to search:");
    println!();

    // Display Fuzzy Picker loaded with mock projects
    let demo_projects = vec!(
        ("services", "rust-telemetry-server", demo_base.join("services/rust-telemetry-server")),
        ("ui", "react-dashboard-ui", demo_base.join("ui/react-dashboard-ui")),
        ("ml", "python-ml-model", demo_base.join("ml/python-ml-model")),
    );

    let labels: Vec<String> = demo_projects
        .iter()
        .map(|(cat, name, _)| format!("  {:<10}  {}", format!("[{}]", cat).cyan(), name.bold()))
        .collect();

    let chosen = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("jump to")
        .items(&labels)
        .default(0)
        .interact()?;

    let (chosen_cat, chosen_name, chosen_path) = &demo_projects[chosen];

    println!();
    println!("  🎉 {}", "Awesome choice!".green().bold());
    println!("  You selected: {} {}", format!("[{}]", chosen_cat).cyan(), chosen_name.bold());
    println!("  Path: {}", chosen_path.display().to_string().dimmed());
    println!();
    if !preferred_editor.is_empty() {
        println!(
            "  In a real terminal shell, running 'pg' would instantly jump to this folder\n  and run: {} .",
            preferred_editor.bold()
        );
    } else {
        println!(
            "  In a real terminal shell, running 'pg' would instantly jump to this folder\n  and open your default editor."
        );
    }
    println!();

    println!("{}", "  Onboarding complete! You are ready to organize and navigate your codebases like a pro. 🚀".green());
    println!();

    Ok(())
}

fn detect_target() -> InitTarget {
    if std::env::consts::OS == "windows" {
        InitTarget::PowerShell
    } else {
        InitTarget::Zsh
    }
}

fn has_zoxide() -> bool {
    Command::new("zoxide")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_or(false, |s| s.success())
}

fn install_zoxide() -> Result<()> {
    let plan = installer_plan(std::env::consts::OS, os_release().as_deref());
    for cmd in &plan {
        eprintln!("      trying: {}", cmd.dimmed());
        if run_install_command(cmd) {
            eprintln!("      {} installed zoxide", "✓".green().bold());
            return Ok(());
        }
    }

    anyhow::bail!(
        "failed to install zoxide automatically. install it manually, then run `projm init` again"
    )
}

fn run_install_command(cmd: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        return Command::new("cmd")
            .args(["/C", cmd])
            .status()
            .map_or(false, |s| s.success());
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("sh")
            .args(["-c", cmd])
            .status()
            .map_or(false, |s| s.success())
    }
}

fn completion_path(target: InitTarget) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    match target {
        InitTarget::Zsh => home.join(".config/zsh/completions/_projm"),
        InitTarget::PowerShell => home.join(".config/powershell/completions/projm.ps1"),
    }
}

fn write_completions(target: InitTarget, path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let script = match target {
        InitTarget::Zsh => completions::zsh_script()?,
        InitTarget::PowerShell => completions::powershell_script()?,
    };
    fs::write(path, script)?;
    Ok(())
}

fn update_shell_profile(target: InitTarget, alias: &str) -> Result<PathBuf> {
    let profile = shell_profile_path(target);

    if let Some(parent) = profile.parent() {
        fs::create_dir_all(parent)?;
    }

    let old = fs::read_to_string(&profile).unwrap_or_default();
    let updated = match target {
        InitTarget::Zsh => {
            let with_projm = ensure_projm_block_zsh(&old, alias);
            ensure_line(&with_projm, ZOXIDE_INIT_ZSH)
        }
        InitTarget::PowerShell => {
            let with_projm = ensure_projm_block_powershell(&old, alias);
            ensure_line(&with_projm, ZOXIDE_INIT_POWERSHELL)
        }
    };

    if updated != old {
        fs::write(&profile, updated).with_context(|| format!("write {}", profile.display()))?;
    }

    Ok(profile)
}

fn shell_profile_path(target: InitTarget) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    match target {
        InitTarget::Zsh => home.join(".config/zsh/.zshrc"),
        InitTarget::PowerShell => home.join("Documents/PowerShell/Microsoft.PowerShell_profile.ps1"),
    }
}

fn ensure_projm_block_zsh(content: &str, alias: &str) -> String {
    let block = format!(
        "{start}\n{alias}() {{\n    local cmd\n    cmd=$(projm g \"$@\" 2>/dev/tty </dev/tty) || return\n    [ -n \"$cmd\" ] && eval \"$cmd\"\n}}\npn() {{\n    projm run \"$@\"\n}}\nfpath=(\"$HOME/.config/zsh/completions\" $fpath)\nautoload -Uz compinit && compinit\n{end}\n",
        alias = alias,
        start = PROJM_BLOCK_START,
        end = PROJM_BLOCK_END
    );

    if let (Some(start_idx), Some(end_idx)) = (content.find(PROJM_BLOCK_START), content.find(PROJM_BLOCK_END)) {
        if start_idx < end_idx {
            let existing_block = &content[start_idx..end_idx + PROJM_BLOCK_END.len()];
            if existing_block.trim() == block.trim() {
                return content.to_owned();
            }
            let mut new_content = content[..start_idx].to_owned();
            new_content.push_str(&block);
            new_content.push_str(content[end_idx + PROJM_BLOCK_END.len()..].trim_start_matches('\n'));
            return new_content;
        }
    }

    if content.trim().is_empty() {
        block
    } else {
        format!("{}\n\n{}", content.trim_end(), block)
    }
}

fn ensure_projm_block_powershell(content: &str, alias: &str) -> String {
    let block = format!(
        "{start}\nfunction {alias} {{\n  $cmd = projm g $args 2>$null\n  if ($cmd) {{ Invoke-Expression $cmd }}\n}}\nfunction pn {{\n  projm run $args\n}}\n. \"$HOME/.config/powershell/completions/projm.ps1\"\n{end}\n",
        alias = alias,
        start = PROJM_BLOCK_START,
        end = PROJM_BLOCK_END
    );

    if let (Some(start_idx), Some(end_idx)) = (content.find(PROJM_BLOCK_START), content.find(PROJM_BLOCK_END)) {
        if start_idx < end_idx {
            let existing_block = &content[start_idx..end_idx + PROJM_BLOCK_END.len()];
            if existing_block.trim() == block.trim() {
                return content.to_owned();
            }
            let mut new_content = content[..start_idx].to_owned();
            new_content.push_str(&block);
            new_content.push_str(content[end_idx + PROJM_BLOCK_END.len()..].trim_start_matches('\n'));
            return new_content;
        }
    }

    if content.trim().is_empty() {
        block
    } else {
        format!("{}\n\n{}", content.trim_end(), block)
    }
}

fn ensure_line(content: &str, line: &str) -> String {
    if content.lines().any(|l| l.trim() == line.trim()) {
        return content.to_owned();
    }
    if content.trim().is_empty() {
        format!("{}\n", line)
    } else {
        format!("{}\n{}\n", content.trim_end(), line)
    }
}

fn os_release() -> Option<String> {
    let data = fs::read_to_string("/etc/os-release").ok()?;
    Some(data.to_lowercase())
}

pub fn installer_plan(os: &str, os_release: Option<&str>) -> Vec<String> {
    if os == "windows" {
        return vec![
            "winget install --id ajeetdsouza.zoxide -e --source winget".into(),
            "choco install zoxide -y".into(),
            "scoop install zoxide".into(),
            "cargo install zoxide".into(),
        ];
    }

    if os == "macos" {
        return vec!["brew install zoxide".into(), "cargo install zoxide".into()];
    }

    if os == "linux" {
        let release = os_release.unwrap_or_default();
        if release.contains("arch") || release.contains("manjaro") {
            return vec![
                "pacman -S --noconfirm zoxide".into(),
                "yay -S --noconfirm zoxide".into(),
                "paru -S --noconfirm zoxide".into(),
                "cargo install zoxide".into(),
            ];
        }
        if release.contains("ubuntu") || release.contains("debian") {
            return vec![
                "apt update && apt install -y zoxide".into(),
                "cargo install zoxide".into(),
            ];
        }

        return vec!["cargo install zoxide".into()];
    }

    vec!["cargo install zoxide".into()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_line_is_idempotent() {
        let a = ensure_line("hello", "world");
        let b = ensure_line(&a, "world");
        assert_eq!(a, b);
    }

    #[test]
    fn ensure_line_on_empty_content_has_no_leading_newline() {
        let line = ensure_line("", "world");
        assert_eq!(line, "world\n");
    }

    #[test]
    fn ensure_projm_block_is_idempotent() {
        let a = ensure_projm_block_zsh("", "pg");
        let b = ensure_projm_block_zsh(&a, "pg");
        assert_eq!(a, b);
    }

    #[test]
    fn ensure_powershell_block_is_idempotent() {
        let a = ensure_projm_block_powershell("", "pg");
        let b = ensure_projm_block_powershell(&a, "pg");
        assert_eq!(a, b);
    }

    #[test]
    fn ensure_projm_block_custom_alias() {
        let block_zsh = ensure_projm_block_zsh("", "pj");
        assert!(block_zsh.contains("pj() {"));
        assert!(!block_zsh.contains("pg() {"));
        assert!(block_zsh.contains("pn() {"));

        let block_ps = ensure_projm_block_powershell("", "pj");
        assert!(block_ps.contains("function pj {"));
        assert!(!block_ps.contains("function pg {"));
        assert!(block_ps.contains("function pn {"));
    }

    #[test]
    fn arch_plan_prefers_pacman_then_helpers_then_cargo() {
        let plan = installer_plan("linux", Some("id=arch"));
        assert_eq!(
            plan,
            vec![
                "pacman -S --noconfirm zoxide",
                "yay -S --noconfirm zoxide",
                "paru -S --noconfirm zoxide",
                "cargo install zoxide"
            ]
        );
    }

    #[test]
    fn debian_plan_prefers_apt_then_cargo() {
        let plan = installer_plan("linux", Some("id=ubuntu"));
        assert_eq!(
            plan,
            vec![
                "apt update && apt install -y zoxide",
                "cargo install zoxide"
            ]
        );
    }

    #[test]
    fn windows_plan_prefers_winget_then_choco_then_scoop_then_cargo() {
        let plan = installer_plan("windows", None);
        assert_eq!(
            plan,
            vec![
                "winget install --id ajeetdsouza.zoxide -e --source winget",
                "choco install zoxide -y",
                "scoop install zoxide",
                "cargo install zoxide"
            ]
        );
    }
}

