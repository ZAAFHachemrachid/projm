# Embedded Terminal v2 — Shell Integration + AppImage Env Hygiene

Date: 2026-07-07
Status: Shipped
Inspired by: the Terax terminal architecture (terax-ai)

## Summary

Upgrades projm's embedded terminal from a bare PTY spawn to a Terax-style
integrated shell, and scrubs the AppImage runtime environment out of every
child process the app spawns.

## Problems solved

1. **AppImage env leak** — shells and dev-runner commands spawned from the
   AppImage build inherited `ARGV0` (breaks rustup's cargo shim: "unknown
   proxy name"), and mount-point `LD_LIBRARY_PATH`/`GDK_PIXBUF_MODULE_FILE`/
   `GST_PLUGIN_SYSTEM_PATH*` (breaks webkit2gtk child spawning for nested
   Tauri dev builds).
2. **No shell integration** — the host couldn't track cwd or command
   boundaries without parsing prompt text.
3. **Weak shell resolution** — `$SHELL` may be unset/misleading for
   GUI-launched apps.

## Components

- `crates/projm-core/src/env_hygiene.rs` — `appimage_env_fix()` detects
  `APPDIR` and computes repairs: remove launcher markers
  (`APPDIR`/`APPIMAGE`/`ARGV0`/`OWD`), filter bundle entries out of
  colon-list vars (`PATH`, `LD_LIBRARY_PATH`, `XDG_DATA_DIRS`, `GST_*`, …),
  drop scalar vars pointing into the bundle. `None` outside an AppImage.
  Applied in: PTY spawn (`shell_init`), dev runner
  (`runner/process.rs::build_command`), external terminal launch
  (`external_term.rs`).
- `crates/projm-core/src/shell.rs` — `login_shell()` reads the passwd entry
  (`getpwuid`); auto-detect order is now Settings pref → login shell →
  `$SHELL` → probe list.
- `src-tauri/src/shell_init.rs` + `src-tauri/src/scripts/` — per-shell
  integration, written atomically to `~/.cache/projm/shell-integration/`:
  - **zsh**: synthetic `ZDOTDIR` shims (`.zshenv`/`.zprofile`/`.zshrc`/
    `.zlogin`) that source the user's real config (via `PROJM_USER_ZDOTDIR`,
    guarding projm-in-projm), then install precmd/preexec hooks. Spawned with
    `-l` so macOS `path_helper` populates PATH. Trailing `:` in the shims
    keeps `$?` clean for the first prompt.
  - **bash**: `--rcfile <shim> -i`; the shim emulates login init
    (`/etc/profile`, profile files) because bash ignores `--rcfile` under
    `-l`; pre-exec marker via `PS0` (bash ≥ 4.4).
  - **fish**: guarded `~/.config/fish/conf.d/projm.fish` — no-ops unless
    `PROJM_TERMINAL` is set (improvement over Terax, which hooks every fish).
  - All emit **OSC 7** (cwd as `file://` URI) and **OSC 133 A/B/C/D**
    (prompt-start / prompt-end / pre-exec / command-done-with-exit-code).
  - Common env: `TERM=xterm-256color`, `COLORTERM=truecolor`,
    `PROJM_TERMINAL=1`, UTF-8 locale fallback (`C.UTF-8` on Linux).
- `app/components/ui/terminal.tsx` — loads the **WebGL renderer**
  (`@xterm/addon-webgl`) with DOM fallback on context loss; registers an
  **OSC 7 handler** exposing cwd changes via the new `onCwdChange` prop.

## Verification

- `zsh -n` / `bash -n` on all scripts; live test: `ZDOTDIR=<shims> zsh -il`
  emits `133;D;0`, `7;file://…` (correct URL-encoded cwd), `133;A` and still
  sources the user's zshrc.
- env_hygiene unit tests (6) cover markers, list filtering, scalar drops,
  double-slash bundle paths, and untouched vars.
- Full workspace suite green; dev app boots clean with the new spawn path.
