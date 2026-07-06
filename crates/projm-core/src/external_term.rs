//! Open the user's real terminal emulator at a project directory.
//!
//! Resolution order: `terminal` in `~/.config/projm/prefs.json` → `$TERMINAL`
//! env var → a probe list of common emulators. The terminal is spawned fully
//! detached (own process group, null stdio) so it outlives projm.

use anyhow::Result;
use std::path::Path;
use std::process::{Command, Stdio};

#[cfg(not(target_os = "macos"))]
use crate::agents;
use crate::prefs::Prefs;

/// Emulators probed on Linux/BSD when neither prefs nor $TERMINAL name one.
const PROBE_LIST: &[&str] = &[
    "kitty",
    "alacritty",
    "wezterm",
    "foot",
    "ghostty",
    "gnome-terminal",
    "konsole",
    "xfce4-terminal",
    "xterm",
];

/// Open the preferred terminal emulator with its working directory at `path`.
/// Returns the name of the terminal that was launched.
pub fn open_terminal_at(path: &Path) -> Result<String> {
    let prefs = Prefs::load().unwrap_or_default();
    let env_terminal = std::env::var("TERMINAL").ok();
    open_terminal_at_impl(path, prefs.terminal.as_deref(), env_terminal.as_deref())
}

/// Working-directory arguments for the known emulators. Unknown terminals get
/// no args — they inherit the cwd set on the Command instead.
fn args_for(terminal: &str, path: &Path) -> Vec<String> {
    let p = path.to_string_lossy().into_owned();
    match terminal {
        "kitty" => vec!["--directory".into(), p],
        "alacritty" => vec!["--working-directory".into(), p],
        "wezterm" => vec!["start".into(), "--cwd".into(), p],
        "foot" | "ghostty" | "gnome-terminal" | "xfce4-terminal" => {
            vec![format!("--working-directory={p}")]
        }
        "konsole" => vec!["--workdir".into(), p],
        _ => Vec::new(),
    }
}

/// Pick the terminal to launch: prefs value → $TERMINAL → probe list, each
/// gated on `available` (a $PATH check in production, injectable for tests).
fn resolve_terminal(
    pref: Option<&str>,
    env_terminal: Option<&str>,
    available: &dyn Fn(&str) -> bool,
) -> Option<String> {
    for candidate in pref.into_iter().chain(env_terminal) {
        let candidate = candidate.trim();
        if !candidate.is_empty() && available(candidate) {
            return Some(candidate.to_string());
        }
    }
    PROBE_LIST
        .iter()
        .find(|t| available(t))
        .map(|t| t.to_string())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn open_terminal_at_impl(
    path: &Path,
    pref: Option<&str>,
    env_terminal: Option<&str>,
) -> Result<String> {
    let on_path = |name: &str| agents::detect_path(name).is_some();
    let Some(terminal) = resolve_terminal(pref, env_terminal, &on_path) else {
        anyhow::bail!(
            "no terminal emulator found on $PATH — set \"terminal\" in \
             ~/.config/projm/prefs.json or the $TERMINAL environment variable"
        );
    };

    let mut cmd = Command::new(&terminal);
    cmd.args(args_for(&terminal, path));
    spawn_detached_unix(cmd, path)?;
    Ok(terminal)
}

#[cfg(target_os = "macos")]
fn open_terminal_at_impl(
    path: &Path,
    pref: Option<&str>,
    env_terminal: Option<&str>,
) -> Result<String> {
    // On macOS terminals are apps, not $PATH binaries — launch via `open -a`.
    let app = pref
        .or(env_terminal)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Terminal")
        .to_string();

    let mut cmd = Command::new("open");
    cmd.arg("-a").arg(&app).arg(path);
    spawn_detached_unix(cmd, path)?;
    Ok(app)
}

#[cfg(unix)]
fn spawn_detached_unix(mut cmd: Command, path: &Path) -> Result<()> {
    use std::os::unix::process::CommandExt;
    let mut child = cmd
        .current_dir(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    // Reap off-thread so the launcher never leaves a zombie behind.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_terminal_at_impl(
    path: &Path,
    pref: Option<&str>,
    env_terminal: Option<&str>,
) -> Result<String> {
    use std::os::windows::process::CommandExt;
    // CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW — the terminal opens its own
    // window; the launcher itself must not flash a console.
    const DETACH_FLAGS: u32 = 0x0200 | 0x0800_0000;

    let on_path = |name: &str| agents::detect_path(name).is_some();
    let preferred = resolve_terminal(pref, env_terminal, &on_path);

    let (name, mut cmd) = match preferred.as_deref() {
        Some("wt") => {
            let mut c = Command::new("wt");
            c.arg("-d").arg(path);
            ("wt".to_string(), c)
        }
        Some(other) => {
            let mut c = Command::new(other);
            c.args(args_for(other, path));
            (other.to_string(), c)
        }
        None if on_path("wt") => {
            let mut c = Command::new("wt");
            c.arg("-d").arg(path);
            ("wt".to_string(), c)
        }
        None => {
            let mut c = Command::new("cmd");
            c.arg("/c")
                .arg("start")
                .arg("")
                .arg("/D")
                .arg(path)
                .arg("cmd");
            ("cmd".to_string(), c)
        }
    };

    let mut child = cmd
        .current_dir(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(DETACH_FLAGS)
        .spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn pref_wins_over_env_and_probe() {
        let available = |_: &str| true;
        let got = resolve_terminal(Some("kitty"), Some("alacritty"), &available);
        assert_eq!(got.as_deref(), Some("kitty"));
    }

    #[test]
    fn env_wins_when_pref_missing_or_unavailable() {
        let available = |name: &str| name != "ghostty";
        let got = resolve_terminal(Some("ghostty"), Some("alacritty"), &available);
        assert_eq!(got.as_deref(), Some("alacritty"));
    }

    #[test]
    fn probe_order_is_respected() {
        let available = |name: &str| name == "foot" || name == "xterm";
        let got = resolve_terminal(None, None, &available);
        assert_eq!(got.as_deref(), Some("foot"));
    }

    #[test]
    fn nothing_available_yields_none() {
        let available = |_: &str| false;
        assert_eq!(resolve_terminal(None, Some("  "), &available), None);
    }

    #[test]
    fn args_tables_cover_known_terminals() {
        let p = PathBuf::from("/tmp/proj");
        assert_eq!(args_for("kitty", &p), vec!["--directory", "/tmp/proj"]);
        assert_eq!(
            args_for("wezterm", &p),
            vec!["start", "--cwd", "/tmp/proj"]
        );
        assert_eq!(
            args_for("gnome-terminal", &p),
            vec!["--working-directory=/tmp/proj"]
        );
        assert_eq!(args_for("konsole", &p), vec!["--workdir", "/tmp/proj"]);
        // Unknown terminals rely on Command::current_dir.
        assert!(args_for("mystery-term", &p).is_empty());
    }
}
