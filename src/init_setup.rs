use anyhow::{Context, Result};
use colored::Colorize;
use std::{fs, path::PathBuf, process::Command};

use crate::completions;

const PROJM_BLOCK_START: &str = "# >>> projm >>>";
const PROJM_BLOCK_END: &str = "# <<< projm <<<";
const ZOXIDE_INIT_ZSH: &str = "eval \"$(zoxide init zsh)\"";
const ZOXIDE_INIT_POWERSHELL: &str = "Invoke-Expression (& { (zoxide init powershell | Out-String) })";

#[derive(Clone, Copy)]
enum InitTarget {
    Zsh,
    PowerShell,
}

pub fn run() -> Result<()> {
    let target = detect_target();

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
    let profile = update_shell_profile(target)?;
    eprintln!("\n  {} updated {}", "done.".green().bold(), profile.display());

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

fn update_shell_profile(target: InitTarget) -> Result<PathBuf> {
    let profile = shell_profile_path(target);

    if let Some(parent) = profile.parent() {
        fs::create_dir_all(parent)?;
    }

    let old = fs::read_to_string(&profile).unwrap_or_default();
    let updated = match target {
        InitTarget::Zsh => {
            let with_projm = ensure_projm_block_zsh(&old);
            ensure_line(&with_projm, ZOXIDE_INIT_ZSH)
        }
        InitTarget::PowerShell => {
            let with_projm = ensure_projm_block_powershell(&old);
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
        InitTarget::Zsh => home.join(".zshrc"),
        InitTarget::PowerShell => home.join("Documents/PowerShell/Microsoft.PowerShell_profile.ps1"),
    }
}

fn ensure_projm_block_zsh(content: &str) -> String {
    if content.contains(PROJM_BLOCK_START) {
        return content.to_owned();
    }

    let block = format!(
        "{start}\npg() {{\n    local cmd\n    cmd=$(projm g 2>/dev/tty) || return\n    [ -n \"$cmd\" ] && eval \"$cmd\"\n}}\nfpath=(\"$HOME/.config/zsh/completions\" $fpath)\nautoload -Uz compinit && compinit\n{end}\n",
        start = PROJM_BLOCK_START,
        end = PROJM_BLOCK_END
    );

    if content.trim().is_empty() {
        block
    } else {
        format!("{}\n\n{}", content.trim_end(), block)
    }
}

fn ensure_projm_block_powershell(content: &str) -> String {
    if content.contains(PROJM_BLOCK_START) {
        return content.to_owned();
    }

    let block = format!(
        "{start}\nfunction pg {{\n  $cmd = projm g 2>$null\n  if ($cmd) {{ Invoke-Expression $cmd }}\n}}\n. \"$HOME/.config/powershell/completions/projm.ps1\"\n{end}\n",
        start = PROJM_BLOCK_START,
        end = PROJM_BLOCK_END
    );

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
        let a = ensure_projm_block_zsh("");
        let b = ensure_projm_block_zsh(&a);
        assert_eq!(a, b);
    }

    #[test]
    fn ensure_powershell_block_is_idempotent() {
        let a = ensure_projm_block_powershell("");
        let b = ensure_projm_block_powershell(&a);
        assert_eq!(a, b);
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

