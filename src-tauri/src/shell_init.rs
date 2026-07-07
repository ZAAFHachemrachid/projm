//! Build the PTY shell command for the embedded terminal.
//!
//! Ported from the Terax terminal architecture: the user's shell choice
//! (Settings pref → login shell → `$SHELL` → probe) is spawned with
//! per-shell integration that sources the user's real config and then emits
//! OSC 7 (cwd) + OSC 133 A/B/C/D (prompt/command boundaries) so the host can
//! track cwd and command state without parsing prompts.
//!
//! - zsh: synthetic `ZDOTDIR` (shim files source the user's own, guarded by
//!   `PROJM_USER_ZDOTDIR` against projm-in-projm), spawned as a login
//!   shell so `/etc/zprofile` (macOS `path_helper`) populates PATH.
//! - bash: `--rcfile` + `-i` (bash ignores --rcfile under `-l`; the rcfile
//!   emulates login init itself).
//! - fish: a guarded conf.d snippet (only activates when `PROJM_TERMINAL` set).
//! - anything else: spawned plain.
//!
//! Every spawn also gets a UTF-8 locale fallback, `TERM`/`COLORTERM`, and the
//! AppImage environment scrub from `projm_core::env_hygiene`.

use portable_pty::CommandBuilder;

pub fn build_command(shell_path: &str, cwd: &str) -> CommandBuilder {
    let mut cmd = CommandBuilder::new(shell_path);
    apply_common(&mut cmd, cwd);

    #[cfg(unix)]
    unix::apply_integration(&mut cmd, shell_path);

    cmd
}

fn apply_common(cmd: &mut CommandBuilder, cwd: &str) {
    cmd.cwd(cwd);
    // A GUI app launched from the desktop has no TERM in its environment, so a
    // shell spawned here inherits none ("TERM environment variable not set").
    // Without TERM, zsh's line editor (zle) and autosuggestions can't emit
    // cursor-control sequences. The frontend renders the PTY via xterm.js,
    // which advertises xterm-256color with truecolor.
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("PROJM_TERMINAL", "1");
    ensure_utf8_locale(cmd);

    // Never leak the AppImage runtime env into the shell — it breaks rustup
    // proxies (ARGV0) and webkit2gtk child processes (LD_LIBRARY_PATH & co).
    if let Some(fix) = projm_core::env_hygiene::appimage_env_fix() {
        for key in &fix.remove {
            cmd.env_remove(key);
        }
        for (key, value) in &fix.set {
            cmd.env(key, value);
        }
    }
}

fn ensure_utf8_locale(cmd: &mut CommandBuilder) {
    let is_utf8 = |v: &str| {
        let up = v.to_ascii_uppercase();
        up.contains("UTF-8") || up.contains("UTF8")
    };
    let already_utf8 = ["LC_ALL", "LC_CTYPE", "LANG"]
        .iter()
        .any(|k| std::env::var(k).ok().as_deref().is_some_and(is_utf8));
    if already_utf8 {
        return;
    }
    #[cfg(target_os = "macos")]
    let fallback = "en_US.UTF-8";
    #[cfg(all(unix, not(target_os = "macos")))]
    let fallback = "C.UTF-8";
    #[cfg(windows)]
    let fallback = "en_US.UTF-8";
    cmd.env("LANG", fallback);
}

#[cfg(unix)]
mod unix {
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};

    use portable_pty::CommandBuilder;

    const ZSHENV: &str = include_str!("scripts/zshenv.zsh");
    const ZPROFILE: &str = include_str!("scripts/zprofile.zsh");
    const ZLOGIN: &str = include_str!("scripts/zlogin.zsh");
    const ZSHRC: &str = include_str!("scripts/zshrc.zsh");
    const BASHRC: &str = include_str!("scripts/bashrc.bash");
    const FISH_INIT: &str = include_str!("scripts/init.fish");

    pub fn apply_integration(cmd: &mut CommandBuilder, shell_path: &str) {
        match shell_path.rsplit('/').next().unwrap_or(shell_path) {
            "zsh" => {
                match prepare_zdotdir() {
                    Ok(zdotdir) => {
                        // Guard against projm-in-projm: remember the user's own
                        // ZDOTDIR so the shims source the right config.
                        if let Ok(user_zd) = std::env::var("ZDOTDIR") {
                            if Path::new(&user_zd) != zdotdir.as_path() {
                                cmd.env("PROJM_USER_ZDOTDIR", user_zd);
                            }
                        }
                        cmd.env("ZDOTDIR", &zdotdir);
                    }
                    Err(e) => {
                        eprintln!("zsh shell integration disabled: {e}");
                    }
                }
                // Login shell so /etc/zprofile runs path_helper on macOS — without
                // this, GUI-launched apps get a minimal PATH missing Homebrew.
                cmd.arg("-l");
            }
            "bash" => {
                match prepare_bash_rcfile() {
                    Ok(rc) => {
                        cmd.arg("--rcfile");
                        cmd.arg(rc);
                    }
                    Err(e) => {
                        eprintln!("bash shell integration disabled: {e}");
                    }
                }
                // bash ignores --rcfile under -l, so we use -i and source
                // /etc/profile from inside our rcfile to emulate login init.
                cmd.arg("-i");
            }
            "fish" => {
                if let Err(e) = prepare_fish_conf_d() {
                    eprintln!("fish shell integration disabled: {e}");
                }
                cmd.arg("-i");
            }
            _ => {}
        }
    }

    fn integration_root() -> Result<PathBuf, String> {
        let home = dirs::home_dir().ok_or_else(|| "could not resolve home dir".to_string())?;
        let root = home.join(".cache").join("projm").join("shell-integration");
        fs::create_dir_all(&root).map_err(|e| format!("create {}: {e}", root.display()))?;
        Ok(root)
    }

    fn prepare_zdotdir() -> Result<PathBuf, String> {
        let dir = integration_root()?.join("zsh");
        fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        write_if_changed(&dir.join(".zshenv"), ZSHENV)?;
        write_if_changed(&dir.join(".zprofile"), ZPROFILE)?;
        write_if_changed(&dir.join(".zshrc"), ZSHRC)?;
        write_if_changed(&dir.join(".zlogin"), ZLOGIN)?;
        Ok(dir)
    }

    fn prepare_bash_rcfile() -> Result<PathBuf, String> {
        let dir = integration_root()?.join("bash");
        fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        let rc = dir.join("bashrc");
        write_if_changed(&rc, BASHRC)?;
        Ok(rc)
    }

    fn prepare_fish_conf_d() -> Result<(), String> {
        let home = dirs::home_dir().ok_or_else(|| "could not resolve home dir".to_string())?;
        let dir = home.join(".config").join("fish").join("conf.d");
        fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        write_if_changed(&dir.join("projm.fish"), FISH_INIT)
    }

    fn write_if_changed(path: &Path, content: &str) -> Result<(), String> {
        if let Ok(existing) = fs::read_to_string(path) {
            if existing == content {
                return Ok(());
            }
        }
        // Atomic replace: a parallel shell startup must never source a half-written file.
        let mut tmp: OsString = path.as_os_str().to_owned();
        tmp.push(".__projm_tmp__");
        let tmp = PathBuf::from(tmp);
        fs::write(&tmp, content).map_err(|e| format!("write {}: {e}", tmp.display()))?;
        fs::rename(&tmp, path).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            format!("rename {} -> {}: {e}", tmp.display(), path.display())
        })
    }
}
