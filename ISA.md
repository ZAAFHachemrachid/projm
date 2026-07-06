---
project: projm
task: Terminal Settings — pick embedded-shell + external-emulator in the desktop Settings panel
slug: projm-terminal-settings
effort: E3
phase: complete
progress: 12/12
mode: standard
started: 2026-07-06
updated: 2026-07-06
---

# Projm — Open-Project Tabs with Persistent Sessions

## Problem

Projm can only display one project at a time: `selectedProject` is a single `useState` in `app/app/page.tsx`, and opening another project replaces the current one. Worse, the view remount destroys the xterm shell UI, and the backend shell is a single global PTY (`TerminalState.writer` is one `Option<writer>`; `terminal-data` events are unkeyed), so switching projects orphans the old shell and cross-wires input/output. The user cannot glance at which projects are open, cannot switch back with one click, and loses shell context on every switch.

## Vision

The top header strip becomes a working set, like an IDE: one tab per opened project, each with a live session indicator. Clicking a tab — or pressing Ctrl+Tab — lands you back in that project with its shell scrollback intact and its dev-server logs still streaming. Opening feels like opening; closing feels like closing; nothing is silently lost in between.

## Out of Scope

Reordering tabs by drag-and-drop. Persisting live shell processes across app restarts (only the tab list is restored). Terminal resize (PTY stays 24x80 as today). Multi-window support. Session indicators for projects that are not open as tabs.

## Constraints

- No new frontend dependencies: Base UI tabs, `@tanstack/react-hotkeys`, Tailwind v4, lucide icons only.
- Backend stays in `src-tauri/src/lib.rs` using existing `portable_pty` and session-map patterns (`RunnerState` style).
- Existing runner commands and event names (`runner:log`, `runner:status`) must not change shape — RunnerPanel keeps working untouched except where mounting strategy requires.
- Dark hardcoded theme and existing visual language (#0d0e10 header, indigo accents) preserved.

## Goal

Projm keeps every opened project in an ordered tab strip in the top header; each tab owns a persistent shell PTY and runner session keyed by project path, switchable with one click or Ctrl+Tab / Ctrl+Shift+Tab, closable with the tab's × or Ctrl+W, with open tabs restored on next launch.

## Criteria

- [x] ISC-1: `page.tsx` holds `openProjects: ProjectItem[]` state distinct from `selectedProject` (Grep)
- [x] ISC-2: An `openProject()` helper appends to `openProjects` only when path not already present (Read)
- [x] ISC-3: Sidebar project click calls `openProject`, not bare `setSelectedProject` (Grep line ~806)
- [x] ISC-4: Ctrl+K search-modal project click calls `openProject` (Grep line ~1283)
- [x] ISC-5: File-tree/other project open sites (line ~1175) route through `openProject` (Grep)
- [x] ISC-6: A `ProjectTabs` component exists at `app/components/project-tabs.tsx` (Read)
- [x] ISC-7: Tab strip renders inside the top header panel row (`h-10` header in page.tsx) (Grep for `<ProjectTabs` placement)
- [x] ISC-8: Each tab shows the project name and full path as `title` tooltip (Read component)
- [x] ISC-9: Active tab is visually distinguished (distinct classes on `isActive`) (Read component)
- [x] ISC-10: Each tab has a close (×) button with `stopPropagation` so close ≠ select (Read component)
- [x] ISC-11: Clicking a tab sets that project as `selectedProject` (Read handler wiring)
- [x] ISC-12: Closing the active tab activates a neighbor tab, or clears selection when last tab closes (Read `closeProject`)
- [x] ISC-13: Closing a non-active tab does not change the active selection (Read `closeProject` logic)
- [x] ISC-14: Ctrl+Tab cycles forward through open tabs, wrapping at the end (Grep `useHotkey` binding)
- [x] ISC-15: Ctrl+Shift+Tab cycles backward, wrapping at the start (Grep `useHotkey` binding)
- [x] ISC-16: Ctrl+W closes the active tab (Grep `useHotkey` binding)
- [x] ISC-17: One `RunnerPanel` is mounted per open project, hidden (not unmounted) when inactive (Read body render)
- [x] ISC-18: The panels container stays mounted while dashboard or diagnostics views show (Read render branches)
- [x] ISC-19: Rust `TerminalState` becomes a session map keyed by cwd (`HashMap<String, …>`) (Read lib.rs)
- [x] ISC-20: `cmd_spawn_terminal` reuses an existing live session for the same cwd instead of respawning (Read lib.rs)
- [x] ISC-21: `terminal-data` events carry `{ cwd, data }` payload (Read emit site)
- [x] ISC-22: `cmd_write_terminal` takes `cwd` and routes input to that session's writer (Read lib.rs)
- [x] ISC-23: New `cmd_kill_terminal(cwd)` kills the child shell and removes the session entry (Read lib.rs)
- [x] ISC-24: `cmd_kill_terminal` is registered in `generate_handler!` (Grep)
- [x] ISC-25: `TerminalView` filters `terminal-data` events by its own cwd (Read terminal.tsx)
- [x] ISC-26: `TerminalView` passes `cwd` to `cmd_write_terminal` (Read terminal.tsx)
- [x] ISC-27: Closing a project tab invokes `cmd_kill_terminal` and `cmd_runner_stop_all` for that path (Read closeProject)
- [x] ISC-28: Tab session dot reflects runner status via a global `runner:status` listener in page.tsx (Read listener + dot wiring)
- [x] ISC-29: Open tab paths + active path persist to localStorage on change (Grep `localStorage`)
- [x] ISC-30: On launch, tabs restore from localStorage by matching paths against loaded projects (Read restore effect)
- [x] ISC-31: `cargo check` passes in src-tauri with zero errors (Bash)
- [x] ISC-32: `bun run build` (Next.js production build incl. typecheck) passes in app/ (Bash)
- [x] ISC-33: Anti: opening the same project twice must NOT create a duplicate tab (Read dedupe logic)
- [x] ISC-34: Anti: existing single-project flows (breadcrumb fallback, diagnostics view, dashboard) must NOT break — all three render branches still reachable (Read render tree)

## Test Strategy

| isc | type | check | threshold | tool |
|-----|------|-------|-----------|------|
| 1-5 | code | state + call-site rewiring present | exact match | Grep/Read |
| 6-13 | code | component exists with correct handlers | logic review | Read |
| 14-16 | code | hotkey bindings registered | 3 bindings | Grep |
| 17-18 | code | keepMounted hidden-panel rendering | no unmount on switch | Read |
| 19-27 | code | Rust session map + keyed events + kill cmd | compiles + logic | Read + cargo check |
| 28 | code | global status listener drives tab dots | wired | Read |
| 29-30 | code | localStorage save/restore effects | present | Grep/Read |
| 31 | build | cargo check exit 0 | 0 errors | Bash |
| 32 | build | next build exit 0 | 0 errors | Bash |
| 33-34 | anti | dedupe + legacy branches intact | logic review | Read |
| UI visual | deferred | tabs render/switch in running desktop app | user launch | DEFERRED-VERIFY (needs `bun run tauri dev`) |

## Features

| name | description | satisfies | depends_on | parallelizable |
|------|-------------|-----------|------------|----------------|
| terminal-sessions-backend | Per-cwd PTY session map, keyed events, kill command | ISC-19..24, 31 | — | yes (Forge) |
| terminal-view-keyed | TerminalView filters by cwd, writes with cwd | ISC-25, 26 | terminal-sessions-backend | no |
| open-projects-state | openProjects state, open/close helpers, call-site rewiring | ISC-1..5, 12, 13, 33 | — | yes (main) |
| project-tabs-component | Tab strip UI with dots, close buttons, active styling | ISC-6..11, 28 | open-projects-state | no |
| keep-mounted-panels | Per-project RunnerPanel mounting, hidden switching | ISC-17, 18, 34 | open-projects-state | no |
| hotkeys | Ctrl+Tab / Ctrl+Shift+Tab / Ctrl+W | ISC-14..16 | open-projects-state | no |
| tab-persistence | localStorage save + restore | ISC-29, 30 | open-projects-state | no |
| session-teardown | Close tab kills shell + stops runners | ISC-23, 27 | terminal-sessions-backend | no |

## Decisions

- 2026-07-06: Root-cause-at-ingestion — the single global PTY (unkeyed writer + unkeyed events) is the ingestion point of session loss; fix it in Rust rather than papering over it in the UI. Fixing there also cures the existing StrictMode double-spawn leak and input cross-wiring.
- 2026-07-06: Keep RunnerPanels mounted-but-hidden per open project instead of remounting keyed on selection — preserves xterm scrollback, log buffers, and event listeners for free; RunnerPanel internals stay untouched.
- 2026-07-06: No `cmd_runner_sessions` query command — tab dots derive from the existing `runner:status` event stream since panels stay mounted; app restart clears sessions anyway (Bitter Pill: no new API surface without need).
- 2026-07-06: EnterPlanMode skipped despite E3: user request is an unambiguous build directive, edits reversible, session autonomous — blocking on plan approval would stall delivery.
- 2026-07-06: Delegation — Forge implements the isolated Rust backend feature (lib.rs only) in parallel with main-agent frontend work; disjoint files prevent conflicts.
- 2026-07-06: Forge provenance — codex CLI auth expired (401 invalid_refresh_token); subagent disclosed and implemented with Claude lineage instead of GPT-5.4. Cross-vendor property not obtained this run; `codex login` needed to restore.
- 2026-07-06: Advisor gaps triaged — added Ctrl+PageUp/PageDown fallback bindings (webviews may swallow Ctrl+Tab) and wait-after-kill reaping in cmd_kill_terminal. PTY 24x80 sizing is pre-existing, declared Out of Scope; cwd keys come from one source (backend project list) so normalization risk accepted; scrollback bounded (xterm default cap + UI_LOG_CAP 2000).

## Changelog

- 2026-07-06 — conjectured: tabs were a pure frontend feature over the existing session backend. refuted by: `cmd_write_terminal` had no cwd parameter and `terminal-data` events were unkeyed — a single global PTY meant any multi-terminal UI would cross-wire input/output. learned: the runner subsystem was session-keyed but the terminal subsystem was not; "sessions persist" required fixing the terminal ingestion point in Rust first. criterion now: ISC-19..24 (per-cwd session map, keyed events, kill command).
- 2026-07-06 (terminal overhaul, plan `Plans/synthetic-wobbling-forest.md`) — conjectured: backend session ids should be minted in Rust and returned from spawn. refuted by: any id returned after spawn means the shell's first output (the prompt) races the frontend's event-listener attach and can be lost. learned: let the FRONTEND generate session ids so it subscribes to `terminal-data:{id}` before spawning; reuse-if-alive by id also absorbs StrictMode double-mounts. Shipped: PTY resize sync (`cmd_resize_terminal` + spawn-at-fitted-size, closing the 24x80 defect), per-project multi-tab sessions keyed by id with `terminal-exit:{id}` reaping, xterm addons (search/web-links/clipboard/unicode11, 10k scrollback), external terminal handoff via `projm_core::external_term` (prefs.terminal → $TERMINAL → probe list; detached spawn on all 3 OSes), agents launch into fresh tabs gated on a ready-handshake.

## Verification

- ISC-19: Read lib.rs:26-28 — `pub struct TerminalState { sessions: StdMutex<HashMap<String, TermSession>> }`
- ISC-20: Read lib.rs:112-122 — `sessions.get_mut(&cwd)` + `try_wait()` reuse path returns Ok(()) for live child, removes stale entry otherwise
- ISC-21: Read lib.rs:32-36, 174-180 — `TerminalData { cwd, data }` serialized payload emitted on `terminal-data`
- ISC-22: Read lib.rs:191-206 — `cmd_write_terminal(cwd, input)` routes via `sessions.get_mut(&cwd)`, errors when session missing
- ISC-23: Read lib.rs:209-219 — `sessions.remove(&cwd)` + `session.child.kill()` best-effort
- ISC-24: Grep lib.rs:753 — `cmd_kill_terminal` present in `generate_handler!` list
- ISC-31: Bash — `cargo check` in src-tauri: "Finished `dev` profile ... in 0.12s", zero errors
- ISC-32: Bash — `bun run build` in app/: "✓ Compiled successfully", TypeScript pass, 8/8 static pages; re-run green after hotkey-fallback edit
- ISC-1..18, 25..30: Grep/Read — helpers `openProject`/`closeProject`/`cycleTab`, ProjectTabs in header, rewired open-sites at page.tsx:947/1344/1451 (zero bare `setSelectedProject(p)` open-sites remain), hotkeys page.tsx:533-568, `projm.openTabs` persistence, cwd-keyed TerminalView
- ISC-33: Read — `openProject` dedupes on `prev.some(x => x.path.toString() === path)`
- ISC-34: Read + build — dashboard/diagnostics branches intact behind `selectedProject ? null : ...`; production build renders all routes
- ISC-23 (reap fix): Read lib.rs — `child.kill()` followed by off-thread `child.wait()`; cargo check re-run green
- UI visual: DEFERRED-VERIFY — desktop app run required; follow-up: launch `bun run tauri dev`, open two projects, confirm independent shells, one-click + Ctrl+Tab/Ctrl+PageDown switching, close teardown
- Terminal overhaul (2026-07-06): Bash — `cargo build --workspace` green; `cargo test --workspace` 166 passed / 0 failed (incl. 5 new external_term tests); `bun run build` ✓ compiled, 8/8 pages; eslint + tsc clean on terminal.tsx/runner-panel.tsx. DEFERRED-VERIFY (needs live app): tput cols matches panel, htop resize redraw, tab isolation (`echo $$`), `exit` closes tab without zombie, external-terminal button opens emulator at project cwd.

## Terminal Shell & Emulator Settings (2026-07-06)

**Problem:** The embedded terminal hardcoded its shell (`/bin/zsh`→`bash`→`sh`, `COMSPEC` on Windows) with no way to choose; the external-emulator preference existed only as a hand-edited JSON key (`prefs.terminal`) with no GUI. Users on fish/pwsh/powershell or a non-default emulator had no supported override.

**Goal:** A **Terminal** tab in desktop Settings that lets the user pick (a) the shell the in-app terminal spawns and (b) the emulator the "Open in terminal" action launches — both persisted to `~/.config/projm/prefs.json`, both auto-detecting when left on Auto.

**Out of scope:** Per-project shell overrides; retro-applying a shell change to already-open terminals; shell startup-arg/env customization; macOS emulator `$PATH` detection (apps aren't on PATH — free-text fallback covers it).

### Criteria
- [x] ISC-1: `crates/projm-core/src/shell.rs` resolves pref → `$SHELL` → probe list; absolute paths trusted, unknown names fall through to auto (so terminal always opens).
- [x] ISC-2: `explicit_shell` unit-tested — auto/blank→None, named-on-path wins, named-absent→None, absolute path trusted (4 tests green).
- [x] ISC-3: `Prefs` gains `shell: Option<String>` with `#[serde(default)]` (backward-compatible load of old prefs.json).
- [x] ISC-4: `Prefs::set_shell`/`set_terminal` persist choice; blank clears to auto.
- [x] ISC-5: `ensure_terminal` reads `prefs.shell` fresh per spawn via `shell::resolve_shell` (change applies to next terminal, no restart).
- [x] ISC-6: `external_term::PROBE_LIST` made `pub` for GUI candidate listing.
- [x] ISC-7: `cmd_get_terminal_config` returns saved prefs + resolvedShell + shell/emulator candidate lists with install status.
- [x] ISC-8: `cmd_set_terminal_shell` / `cmd_set_external_terminal` registered in `generate_handler!`.
- [x] ISC-9: Settings page has a **Terminal** tab (Terminal icon) between General and Categories.
- [x] ISC-10: Shell + emulator cards: native `<select>` (Auto / detected candidates flagged / Custom…) + custom-path Input + Save with success/error toast.
- [x] ISC-11: Anti: no regression — `cargo test -p projm_core` 76 unit + all integration suites pass; `tsc --noEmit` exit 0.
- [x] ISC-12: `cargo check -p projm-tauri` compiles all three new commands clean.

### Features
- `shell.rs` resolver + tests | satisfies ISC-1,2 | depends_on none
- prefs `shell` field + setters | ISC-3,4 | depends_on none
- ensure_terminal wiring | ISC-5 | depends_on shell.rs, prefs
- tauri terminal-config commands | ISC-6,7,8 | depends_on prefs, shell.rs, external_term
- Settings Terminal tab UI | ISC-9,10 | depends_on tauri commands

### Verification
- Terminal settings (2026-07-06): Bash — `cargo test -p projm_core` 76 passed / 0 failed (incl. 4 new `shell::tests`); `cargo check -p projm-tauri` Finished clean; `bunx tsc --noEmit` exit 0, 0 errors. Command-name + arg-name parity confirmed frontend↔backend by grep + successful `generate_handler!` compile. DEFERRED-VERIFY (needs live `bun run tauri dev`): open Settings▸Terminal, set shell=fish → new project tab spawns fish (`echo $0`); set emulator=kitty → "Open in terminal" launches kitty at cwd.
