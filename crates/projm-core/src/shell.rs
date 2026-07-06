//! Resolve which shell the embedded (in-app) terminal spawns.
//!
//! Resolution order: the user's Settings choice (`shell` in
//! `~/.config/projm/prefs.json`) → `$SHELL` → a probe list of common shells.
//! An explicit choice may be a bare binary name (resolved on `$PATH`) or an
//! absolute path (trusted as-is). `"auto"`/empty means auto-detect. This
//! mirrors `external_term`'s emulator resolution so both are testable.

use crate::agents;

/// Shells offered in the GUI shell picker, per platform. The value is the name
/// spawned via `CommandBuilder` (resolved on `$PATH`) or shown for detection.
#[cfg(not(target_os = "windows"))]
pub const KNOWN_SHELLS: &[&str] = &["zsh", "bash", "fish", "sh", "nu", "pwsh"];
/// Shells offered in the GUI shell picker on Windows.
#[cfg(target_os = "windows")]
pub const KNOWN_SHELLS: &[&str] = &["pwsh", "powershell", "cmd", "nu", "bash"];

/// Resolve the shell command the embedded terminal should spawn. `pref` is the
/// user's Settings value (`None`/`""`/`"auto"` → auto-detect). Returns a bare
/// binary name or absolute path that `CommandBuilder` can spawn.
pub fn resolve_shell(pref: Option<&str>) -> String {
    let on_path = |name: &str| agents::detect_path(name).is_some();
    explicit_shell(pref, &on_path).unwrap_or_else(auto_shell)
}

/// Return the explicit shell command when `pref` names a usable one — an
/// absolute path (trusted verbatim) or a bare name found by `available`.
/// `None` signals "fall back to auto-detect": pref was empty/`"auto"`, or the
/// named shell is not installed (so the terminal still opens with a fallback).
fn explicit_shell(pref: Option<&str>, available: &dyn Fn(&str) -> bool) -> Option<String> {
    let p = pref?.trim();
    if p.is_empty() || p.eq_ignore_ascii_case("auto") {
        return None;
    }
    if p.contains('/') || p.contains('\\') {
        return Some(p.to_string());
    }
    available(p).then(|| p.to_string())
}

#[cfg(not(target_os = "windows"))]
fn auto_shell() -> String {
    if let Ok(sh) = std::env::var("SHELL") {
        let sh = sh.trim();
        if !sh.is_empty() && std::path::Path::new(sh).exists() {
            return sh.to_string();
        }
    }
    for candidate in ["/bin/zsh", "/bin/bash", "/bin/sh"] {
        if std::path::Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }
    "/bin/sh".to_string()
}

#[cfg(target_os = "windows")]
fn auto_shell() -> String {
    std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_pref_falls_through_to_detect() {
        let available = |_: &str| true;
        assert_eq!(explicit_shell(Some("auto"), &available), None);
        assert_eq!(explicit_shell(Some("AUTO"), &available), None);
        assert_eq!(explicit_shell(Some("  "), &available), None);
        assert_eq!(explicit_shell(None, &available), None);
    }

    #[test]
    fn named_shell_on_path_wins() {
        let available = |name: &str| name == "fish";
        assert_eq!(
            explicit_shell(Some("fish"), &available).as_deref(),
            Some("fish")
        );
    }

    #[test]
    fn named_shell_absent_falls_through() {
        let available = |_: &str| false;
        assert_eq!(explicit_shell(Some("fish"), &available), None);
    }

    #[test]
    fn absolute_path_is_trusted_without_path_lookup() {
        let available = |_: &str| false;
        assert_eq!(
            explicit_shell(Some("/usr/bin/zsh"), &available).as_deref(),
            Some("/usr/bin/zsh")
        );
        assert_eq!(
            explicit_shell(Some(r"C:\Program Files\PowerShell\pwsh.exe"), &available).as_deref(),
            Some(r"C:\Program Files\PowerShell\pwsh.exe")
        );
    }
}
