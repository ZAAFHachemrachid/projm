use anyhow::{Context, Result};
use colored::Colorize;
use std::{fs, path::Path, path::PathBuf, process::Command};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select, FuzzySelect};
use clap::CommandFactory;

use crate::completions;
use projm_core::{config, editors};

const PROJM_BLOCK_START: &str = "# >>> projm >>>";
const PROJM_BLOCK_END: &str = "# <<< projm <<<";

pub fn run(
    alias: &str,
    non_interactive: bool,
    shell_override: Option<completions::CompletionShell>,
    profile_override: Option<PathBuf>,
) -> Result<()> {
    // Create default rules.toml if not already present
    projm_core::rules::init_default_rules()?;

    let is_interactive = !non_interactive && console::user_attended();
    let detected_shell = shell_override.unwrap_or_else(detect_shell);

    if is_interactive {
        run_wizard(alias, detected_shell, profile_override)?;
    } else {
        run_non_interactive(alias, detected_shell, profile_override)?;
    }

    Ok(())
}

fn run_non_interactive(
    alias: &str,
    shell: completions::CompletionShell,
    profile_override: Option<PathBuf>,
) -> Result<()> {
    eprintln!("[1/3] checking zoxide...");

    if has_zoxide() {
        eprintln!("      {} already installed", "✓".green().bold());
    } else {
        eprintln!("      not found");
        install_zoxide()?;
    }

    eprintln!("[2/3] writing completions...");
    let completion_file = completion_path(shell);
    write_completions(shell, &completion_file)?;
    eprintln!(
        "      {} {}",
        "✓".green().bold(),
        completion_file.display().to_string().dimmed()
    );

    eprintln!("[3/3] updating shell profile...");
    let profile = profile_override.unwrap_or_else(|| default_profile_path(shell));
    update_shell_profile(shell, &profile, alias, &completion_file)?;
    eprintln!("\n  {} updated {}", "done.".green().bold(), profile.display());

    Ok(())
}

fn run_wizard(
    alias: &str,
    detected_shell: completions::CompletionShell,
    profile_override: Option<PathBuf>,
) -> Result<()> {
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

    // 3. Select Target Shell
    let shells = [
        ("Zsh", completions::CompletionShell::Zsh),
        ("Bash", completions::CompletionShell::Bash),
        ("Fish", completions::CompletionShell::Fish),
        ("PowerShell", completions::CompletionShell::Powershell),
        ("Nushell", completions::CompletionShell::Nushell),
    ];
    let shell_labels: Vec<String> = shells.iter().map(|(n, _)| n.to_string()).collect();
    let default_shell_idx = shells.iter().position(|(_, s)| *s == detected_shell).unwrap_or(0);

    let chosen_shell_idx = Select::with_theme(&theme)
        .with_prompt("Which shell would you like to configure?")
        .items(&shell_labels)
        .default(default_shell_idx)
        .interact()?;

    let chosen_shell = shells[chosen_shell_idx].1;
    println!("  {} Target shell set to: {}", "✓".green(), shells[chosen_shell_idx].0.bold());
    println!();

    // 4. Configure Alias
    let chosen_alias: String = Input::with_theme(&theme)
        .with_prompt("What alias would you like to use for fuzzy-navigation?")
        .default(alias.to_string())
        .interact_text()?;
    println!();

    // 5. Configure Profile Path
    let default_profile = profile_override.clone().unwrap_or_else(|| default_profile_path(chosen_shell));
    let use_default_profile = Confirm::with_theme(&theme)
        .with_prompt(format!("Use default profile path: {}?", default_profile.display().to_string().cyan()))
        .default(true)
        .interact()?;

    let final_profile = if use_default_profile {
        default_profile
    } else {
        let custom_profile_input: String = Input::with_theme(&theme)
            .with_prompt("Enter custom profile path")
            .default(default_profile.display().to_string())
            .interact_text()?;
        PathBuf::from(custom_profile_input)
    };
    println!();

    // 6. Check & Install Zoxide
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

    // 7. Setup completions & shell profile
    println!("{}", "[2/3] writing completions...".bold());
    let completion_file = completion_path(chosen_shell);
    write_completions(chosen_shell, &completion_file)?;
    println!(
        "      {} {}",
        "✓".green().bold(),
        completion_file.display().to_string().dimmed()
    );
    println!();

    println!("{}", "[3/3] updating shell profile...".bold());
    update_shell_profile(chosen_shell, &final_profile, &chosen_alias, &completion_file)?;
    println!("      {} updated {}", "✓".green().bold(), final_profile.display());
    println!();

    println!("{}", "🎉 Configuration successfully complete!".green().bold());
    println!();

    // 8. Interactive sandbox demo!
    let run_demo = Confirm::with_theme(&theme)
        .with_prompt("Would you like to run a quick 1-minute sandbox demo of projm?")
        .default(true)
        .interact()?;

    if run_demo {
        run_sandbox_demo(&selected_editor)?;
    }

    println!();
    println!("  To start using projm, restart your shell or run:");
    match chosen_shell {
        completions::CompletionShell::Zsh | completions::CompletionShell::Bash | completions::CompletionShell::Fish | completions::CompletionShell::Nushell => {
            println!("  {}", format!("source {}", final_profile.display()).cyan());
        }
        completions::CompletionShell::Powershell => {
            println!("  {}", format!(". \"{}\"", final_profile.display()).cyan());
        }
    }
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
    projm_core::organize::run_with_base(&source_dump, &demo_base, false)?;

    println!();
    println!("  ⚡ {}", "Step 2: Fuzzy Navigation Showcase".bold());
    println!("  Now let's test how you will navigate between them.");
    println!("  We will open a mini-version of our fuzzy navigator.");
    println!("  Use your Arrow Keys to select, or start typing to search:");
    println!();

    // Display Fuzzy Picker loaded with mock projects
    let demo_projects = [
        ("services", "rust-telemetry-server", demo_base.join("services/rust-telemetry-server")),
        ("ui", "react-dashboard-ui", demo_base.join("ui/react-dashboard-ui")),
        ("ml", "python-ml-model", demo_base.join("ml/python-ml-model")),
    ];

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

fn detect_shell() -> completions::CompletionShell {
    if let Ok(shell_env) = std::env::var("SHELL") {
        let shell_lower = shell_env.to_lowercase();
        if shell_lower.contains("zsh") {
            return completions::CompletionShell::Zsh;
        } else if shell_lower.contains("fish") {
            return completions::CompletionShell::Fish;
        } else if shell_lower.contains("bash") {
            return completions::CompletionShell::Bash;
        } else if shell_lower.contains("nu") {
            return completions::CompletionShell::Nushell;
        }
    }

    if std::env::var("PSModulePath").is_ok() || std::env::var("POWERSHELL_DISTRIBUTION_CHANNEL").is_ok() {
        return completions::CompletionShell::Powershell;
    }

    if std::env::consts::OS == "windows" {
        completions::CompletionShell::Powershell
    } else {
        completions::CompletionShell::Zsh
    }
}

fn default_profile_path(shell: completions::CompletionShell) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    match shell {
        completions::CompletionShell::Zsh => {
            if let Ok(zdotdir) = std::env::var("ZDOTDIR") {
                let path = PathBuf::from(zdotdir).join(".zshrc");
                if path.parent().is_some_and(|p| p.exists()) {
                    return path;
                }
            }
            home.join(".zshrc")
        }
        completions::CompletionShell::Bash => {
            if std::env::consts::OS == "macos" {
                home.join(".bash_profile")
            } else {
                home.join(".bashrc")
            }
        }
        completions::CompletionShell::Fish => {
            home.join(".config/fish/config.fish")
        }
        completions::CompletionShell::Powershell => {
            if let Some(path) = query_shell_profile("pwsh", &["-NoProfile", "-Command", "$PROFILE"]) {
                return path;
            }
            if let Some(path) = query_shell_profile("powershell", &["-NoProfile", "-Command", "$PROFILE"]) {
                return path;
            }
            if cfg!(target_os = "windows") {
                home.join("Documents/PowerShell/Microsoft.PowerShell_profile.ps1")
            } else {
                home.join(".config/powershell/Microsoft.PowerShell_profile.ps1")
            }
        }
        completions::CompletionShell::Nushell => {
            if let Some(path) = query_shell_profile("nu", &["-c", "$nu.config-path"]) {
                return path;
            }
            if cfg!(target_os = "windows") {
                dirs::config_dir()
                    .map(|p| p.join("nushell/config.nu"))
                    .unwrap_or_else(|| home.join("AppData/Roaming/nushell/config.nu"))
            } else {
                home.join(".config/nushell/config.nu")
            }
        }
    }
}

fn query_shell_profile(binary: &str, args: &[&str]) -> Option<PathBuf> {
    let output = Command::new(binary)
        .args(args)
        .output()
        .ok()?;
    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path_str.is_empty() {
            return Some(PathBuf::from(path_str));
        }
    }
    None
}

fn completion_path(shell: completions::CompletionShell) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    match shell {
        completions::CompletionShell::Zsh => home.join(".config/zsh/completions/_projm"),
        completions::CompletionShell::Bash => home.join(".config/projm/completions/projm.bash"),
        completions::CompletionShell::Fish => home.join(".config/fish/completions/projm.fish"),
        completions::CompletionShell::Powershell => home.join(".config/powershell/completions/projm.ps1"),
        completions::CompletionShell::Nushell => home.join(".config/projm/completions/projm.nu"),
    }
}

fn write_completions(shell: completions::CompletionShell, path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let script = match shell {
        completions::CompletionShell::Zsh => completions::zsh_script()?,
        completions::CompletionShell::Powershell => completions::powershell_script()?,
        completions::CompletionShell::Nushell => completions::nushell_script()?,
        completions::CompletionShell::Bash => {
            let mut cmd = crate::main_cli::Cli::command();
            let mut buf = Vec::new();
            clap_complete::generate(clap_complete::shells::Bash, &mut cmd, "projm", &mut buf);
            String::from_utf8(buf)?
        }
        completions::CompletionShell::Fish => {
            let mut cmd = crate::main_cli::Cli::command();
            let mut buf = Vec::new();
            clap_complete::generate(clap_complete::shells::Fish, &mut cmd, "projm", &mut buf);
            String::from_utf8(buf)?
        }
    };
    fs::write(path, script)?;
    Ok(())
}

fn update_shell_profile(
    shell: completions::CompletionShell,
    profile: &Path,
    alias: &str,
    completion_path: &Path,
) -> Result<()> {
    if let Some(parent) = profile.parent() {
        fs::create_dir_all(parent)?;
    }

    let old = fs::read_to_string(profile).unwrap_or_default();
    let updated = match shell {
        completions::CompletionShell::Zsh => {
            let comp_dir = completion_path.parent().unwrap_or(completion_path).to_string_lossy().to_string();
            let block = format!(
                "{start}\n{alias}() {{\n    local cmd\n    cmd=$(projm g \"$@\" 2>/dev/tty </dev/tty) || return\n    [ -n \"$cmd\" ] && eval \"$cmd\"\n}}\npn() {{\n    projm run \"$@\"\n}}\nfpath=(\"{comp_dir}\" $fpath)\nautoload -Uz compinit && compinit\n{end}\n",
                alias = alias,
                comp_dir = comp_dir,
                start = PROJM_BLOCK_START,
                end = PROJM_BLOCK_END
            );
            let with_projm = ensure_projm_block(&old, &block);
            ensure_line(&with_projm, "eval \"$(zoxide init zsh)\"")
        }
        completions::CompletionShell::Bash => {
            let comp_file = completion_path.to_string_lossy().to_string();
            let block = format!(
                "{start}\n{alias}() {{\n    local cmd\n    cmd=$(projm g \"$@\" 2>/dev/tty </dev/tty) || return\n    [ -n \"$cmd\" ] && eval \"$cmd\"\n}}\npn() {{\n    projm run \"$@\"\n}}\n. \"{comp_file}\"\n{end}\n",
                alias = alias,
                comp_file = comp_file,
                start = PROJM_BLOCK_START,
                end = PROJM_BLOCK_END
            );
            let with_projm = ensure_projm_block(&old, &block);
            ensure_line(&with_projm, "eval \"$(zoxide init bash)\"")
        }
        completions::CompletionShell::Fish => {
            let block = format!(
                "{start}\nfunction {alias}\n    set -l cmd (projm g $argv 2>/dev/tty </dev/tty)\n    if test -n \"$cmd\"\n        eval $cmd\n    end\nend\nfunction pn\n    projm run $argv\nend\n{end}\n",
                alias = alias,
                start = PROJM_BLOCK_START,
                end = PROJM_BLOCK_END
            );
            let with_projm = ensure_projm_block(&old, &block);
            ensure_line(&with_projm, "zoxide init fish | source")
        }
        completions::CompletionShell::Powershell => {
            let comp_file = completion_path.to_string_lossy().to_string();
            let block = format!(
                "{start}\nfunction {alias} {{\n  $cmd = projm g $args\n  if ($cmd) {{ Invoke-Expression $cmd }}\n}}\nfunction pn {{\n  projm run $args\n}}\n. \"{comp_file}\"\n{end}\n",
                alias = alias,
                comp_file = comp_file,
                start = PROJM_BLOCK_START,
                end = PROJM_BLOCK_END
            );
            let with_projm = ensure_projm_block(&old, &block);
            ensure_line(&with_projm, "Invoke-Expression (& { (zoxide init powershell | Out-String) })")
        }
        completions::CompletionShell::Nushell => {
            let comp_file = completion_path.to_string_lossy().to_string();
            let block = format!(
                "{start}\ndef --env {alias} [...args] {{\n    let cmd = (projm g ...$args | into string | str trim)\n    if ($cmd | is-empty) == false {{\n        let parts = ($cmd | split row \" && \")\n        if ($parts | length) >= 2 {{\n            let cd_part = ($parts | get 0)\n            let edit_part = ($parts | get 1)\n            let path = ($cd_part | str replace -r \"^(cd|z)\\s+'(.*)'$\" \"$2\")\n            cd $path\n            let editor = ($edit_part | str replace -r \"\\s+\\.$\" \"\")\n            run-external $editor \".\"\n        }}\n    }}\n}}\ndef --env pn [...args] {{\n    projm run ...$args\n}}\nsource \"{comp_file}\"\n{end}\n",
                alias = alias,
                comp_file = comp_file,
                start = PROJM_BLOCK_START,
                end = PROJM_BLOCK_END
            );
            let with_projm = ensure_projm_block(&old, &block);
            ensure_line(&with_projm, "zoxide init nushell | source")
        }
    };

    if updated != old {
        fs::write(profile, updated).with_context(|| format!("write {}", profile.display()))?;
    }

    Ok(())
}

fn ensure_projm_block(content: &str, block: &str) -> String {
    if let (Some(start_idx), Some(end_idx)) = (content.find(PROJM_BLOCK_START), content.find(PROJM_BLOCK_END)) {
        if start_idx < end_idx {
            let mut new_content = content[..start_idx].to_owned();
            new_content.push_str(block);
            new_content.push_str(content[end_idx + PROJM_BLOCK_END.len()..].trim_start_matches('\n'));
            return new_content;
        }
    }

    if content.trim().is_empty() {
        block.to_owned()
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

fn has_zoxide() -> bool {
    Command::new("zoxide")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
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
            .is_ok_and(|s| s.success());
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("sh")
            .args(["-c", cmd])
            .status()
            .is_ok_and(|s| s.success())
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
        let block = format!("{}\nsome block\n{}", PROJM_BLOCK_START, PROJM_BLOCK_END);
        let a = ensure_projm_block("", &block);
        let b = ensure_projm_block(&a, &block);
        assert_eq!(a, b);
    }

    #[test]
    fn ensure_projm_block_custom_alias() {
        let test_comp = PathBuf::from("test_comp");
        let old = "";
        let alias = "pj";
        
        let comp_dir = test_comp.parent().unwrap_or(&test_comp).to_string_lossy().to_string();
        let block_content = format!(
            "{start}\n{alias}() {{\n    local cmd\n    cmd=$(projm g \"$@\" 2>/dev/tty </dev/tty) || return\n    [ -n \"$cmd\" ] && eval \"$cmd\"\n}}\npn() {{\n    projm run \"$@\"\n}}\nfpath=(\"{comp_dir}\" $fpath)\nautoload -Uz compinit && compinit\n{end}\n",
            alias = alias,
            comp_dir = comp_dir,
            start = PROJM_BLOCK_START,
            end = PROJM_BLOCK_END
        );
        let updated = ensure_projm_block(old, &block_content);
        assert!(updated.contains("pj() {"));
        assert!(!updated.contains("pg() {"));
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
