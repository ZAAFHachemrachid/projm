# Better Terminal: Embedded Polish + External Handoff + Tabs

## Context

projm's GUI already ships an embedded xterm.js terminal (`app/components/ui/terminal.tsx` + PTY bridge in `src-tauri/src/lib.rs` via `portable-pty 0.8`), but it has a headline bug: the PTY is hardcoded at 24×80 and there is no resize command — FitAddon resizes the frontend only, so TUI apps (vim, htop, Claude Code) render garbled/wrapped. There is also no way to hand off to the user's real terminal.

Decision (user): **hybrid** approach — make the embedded terminal genuinely good for project workflows (runner, agents, quick commands), and add "Open in external terminal" so normal shell work happens in the real terminal. Ship single-session fixes first, then multiple tabs.

Verified during design:
- `portable-pty 0.8.1` `MasterPty::resize(&self, PtySize)` takes `&self` — just rename the `_master` field to `master`.
- `cmd_kill_terminal` is called from `app/app/page.tsx:489` (project tab close), not terminal.tsx unmount.
- xterm 6.x-compatible addons: `@xterm/addon-search 0.16.0`, `@xterm/addon-web-links 0.12.0`, `@xterm/addon-clipboard 0.2.0`, `@xterm/addon-unicode11 0.9.0`.
- `@tauri-apps/plugin-shell` JS package is missing from `app/package.json` even though the `shell:allow-open` capability is granted.
- `Prefs` (`crates/projm-core/src/prefs.rs`) uses `#[serde(default)]` per field — adding a field is backward compatible.
- Dev loop: `cargo tauri dev` from `src-tauri/` (beforeDevCommand runs the Next app); JS deps via `bun install` in `app/`.

---

## Phase A — Embedded terminal fundamentals (single session, keyed by cwd)

### Rust — `src-tauri/src/lib.rs`

1. Rename `TermSession._master` → `master` (struct ~line 22 + insert site ~line 159).
2. `ensure_terminal` accepts `cols: u16, rows: u16`; use them in `openpty(PtySize { rows, cols, .. })`. `cmd_spawn_terminal` takes `cols: Option<u16>, rows: Option<u16>` (default 80×24). `cmd_launch_agent` (~line 484) passes defaults.
3. New command, registered in `generate_handler!`:

```rust
#[tauri::command]
fn cmd_resize_terminal(cwd: String, cols: u16, rows: u16, state: tauri::State<'_, TerminalState>) -> Result<(), String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    if let Some(s) = sessions.get(&cwd) {
        s.master.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 }).map_err(|e| e.to_string())?;
    }
    Ok(()) // missing session ok: resize can race spawn on mount
}
```

### Frontend — `app/components/ui/terminal.tsx`

1. Replace `window.addEventListener("resize")` (~lines 83–89) with a **ResizeObserver** on `containerRef`, debounced via `requestAnimationFrame`, calling `fitAddon.fit()` (catches sidebar/panel layout changes).
2. `term.onResize(({cols, rows}) => invoke("cmd_resize_terminal", {cwd, cols, rows}))` — `fit()` only fires onResize on actual change, so it self-dedupes.
3. Reorder init: `term.open()` → `fit()` → `invoke("cmd_spawn_terminal", {cwd, cols: term.cols, rows: term.rows})`. Keep the 200ms settle refit.
4. Terminal options: add `scrollback: 10_000`, `allowProposedApi: true`.
5. Addons (dynamic import, matching existing pattern; add to `app/package.json`):
   - `@xterm/addon-search` — Ctrl+Shift+F toggles a small overlay input; findNext/findPrevious on Enter/Shift+Enter, Esc closes.
   - `@xterm/addon-web-links` — explicit handler calling `open(uri)` from `@tauri-apps/plugin-shell` (add `@tauri-apps/plugin-shell ^2` to package.json; capability already granted). Don't rely on `window.open` in the webview.
   - `@xterm/addon-clipboard` — OSC 52 support (matters for agents/tmux).
   - `@xterm/addon-unicode11` — correct wide/emoji widths in TUIs.
   - Clipboard keys via `attachCustomKeyEventHandler`: Ctrl+Shift+C copy selection, Ctrl+Shift+V paste. Plain Ctrl+C stays SIGINT.
6. **Skip WebGL addon** — webkit2gtk WebGL is flaky/software-rendered on Linux; DOM renderer is fine for this panel. Revisit only if profiling shows jank.
7. While in there: fix the listener-leak window — use a `disposed` flag guarding the async `listen()` resolution (same pattern as `runner-panel.tsx:285–318`).

### Verify A
`cargo build --manifest-path src-tauri/Cargo.toml`; `bun install` in `app/`; `cargo tauri dev`. Smoke: `tput cols; tput lines` matches panel; run `htop`/`vim`, resize window + collapse sidebar → clean redraw, no 80-col ghosting; launch Claude Code via AgentLauncher → full-panel TUI; Ctrl+Shift+F searches scrollback; printed URL opens browser; Ctrl+Shift+C/V round-trips; Ctrl+C still interrupts.

---

## Phase B — External terminal handoff ("normal terminal for normal stuff")

### `crates/projm-core/src/prefs.rs`
Add `#[serde(default)] pub terminal: Option<String>` to `Prefs`.

### New module — `crates/projm-core/src/external_term.rs` (export from `lib.rs`)
Lives in core (Prefs is here; enables a future `projm term` CLI subcommand). API: `pub fn open_terminal_at(path: &Path) -> Result<String>` (returns terminal name used).

Resolution order (Linux): prefs `terminal` (if on PATH, reuse the PATH-probe approach from `agents::detect_path`) → `$TERMINAL` env → probe: `kitty, alacritty, wezterm, foot, ghostty, gnome-terminal, konsole, xfce4-terminal, xterm`.

Cwd args table (`fn args_for`): kitty `--directory`, alacritty `--working-directory`, wezterm `start --cwd`, foot/ghostty/gnome-terminal/xfce4-terminal `--working-directory=`, konsole `--workdir`, unknown/xterm → rely on `Command::current_dir` (set it always, belt-and-braces).

Detached spawn (Unix): null stdio + `.process_group(0)` + reaper thread (`child.wait()` in a spawned thread — no zombies).
macOS: `open -a <app|Terminal> <path>`. Windows: `wt -d <path>` if `wt.exe` on PATH, else `cmd /c start "" /D <path> cmd`; use `creation_flags(0x0200 | 0x08000000)`.

### Tauri + UI
- `src-tauri/src/lib.rs`: `cmd_open_external_terminal(path: String) -> Result<String, String>` wrapping core; register it.
- `app/components/ui/runner-panel.tsx`: button in the header actions cluster (~line 356, next to `<AgentLauncher/>`), lucide `SquareTerminal` icon, `title="Open in external terminal"`, invokes the command with `project.path`; surface errors like AgentLauncher does.
- Defer the settings-page pref field and the CLI subcommand — env var + probe order covers most users; both are small follow-ups.

### Verify B
`cargo build` workspace. Linux: unset `$TERMINAL` → probe finds emulator; `TERMINAL=alacritty` respected; `"terminal": "kitty"` in `~/.config/projm/prefs.json` wins over env. Terminal opens at project cwd; closing projm doesn't kill it; no `<defunct>` children.

---

## Phase C — Multiple terminal tabs per project

### Rust re-key — `src-tauri/src/lib.rs`
1. Session id: `static NEXT_TERM_ID: AtomicU64` → `format!("t{n}")` (no uuid dep).
2. `TermSession` gains `cwd: String`; `TerminalState.sessions` keyed by session id.
3. Commands: `cmd_spawn_terminal(cwd, cols, rows) -> String` (always spawns, returns id); `cmd_write_terminal(id, input)`; `cmd_resize_terminal(id, cols, rows)`; `cmd_kill_terminal(id)`; new `cmd_kill_project_terminals(cwd)` (kill+reap all matching cwd) — **update `app/app/page.tsx:489`** to call it; `cmd_launch_agent(cwd, name, sessionId: Option<String>)` — fresh session if none, returns id so UI opens a tab.
4. Per-session events: emit `terminal-data:{id}` and new `terminal-exit:{id}` (when reader loop ends: remove from map + `wait()` child via an `AppHandle` clone → fixes zombie on `exit`).

### Frontend
- `terminal.tsx`: props `{cwd, sessionId, onExit}`; listen on `terminal-data:${sessionId}` / `terminal-exit:${sessionId}`; spawning moves **up** to the runner panel; keep the `disposed`-guard pattern (StrictMode double-mount safety).
- `runner-panel.tsx`: replace the `SHELL` constant with `shellTabs: {id, title}[]` state; auto-spawn `Shell 1` on mount (preserves today's behavior); tab strip with close (×) and `+` triggers; `keepMounted` TabsContent per tab; AgentLauncher spawns a fresh tab per agent (long-running agent doesn't squat the user's shell).

### Verify C
3 tabs in one project + 1 in another: `echo $$` differs per tab; input isolated; resize-on-tab-switch correct. `exit` in a tab → tab closes, no defunct shell. Close project tab → all its sessions die. Agent launch → new streaming tab. StrictMode remount → no doubled output.

---

## Risks
- **Windows ConPTY**: resize storms repaint badly on old ConPTY — mitigated by spawning at real dims and rAF-debounced fits.
- **Listener leaks** with multiple sessions — mitigated by per-session event names + `disposed` guards.
- **Unknown terminal in prefs**: still works via `current_dir` inheritance for most emulators; name the limitation in the not-found error.

## Shipping order
A (small diff, fixes the headline 24×80 bug) → B (independent, pure addition) → C (largest, builds on A's resize plumbing; degrades gracefully to one auto-spawned tab).

## Critical files
- `src-tauri/src/lib.rs` — TermSession/TerminalState, all `cmd_*_terminal`, `ensure_terminal`, `generate_handler!`
- `app/components/ui/terminal.tsx` — xterm init, ResizeObserver, addons, session props
- `app/components/ui/runner-panel.tsx` — external-terminal button, tab strip, AgentLauncher wiring
- `crates/projm-core/src/prefs.rs` — `terminal` pref field
- `crates/projm-core/src/external_term.rs` — NEW: detection table + detached spawn
- `app/app/page.tsx` (~line 489) — project-close cleanup → `cmd_kill_project_terminals`
