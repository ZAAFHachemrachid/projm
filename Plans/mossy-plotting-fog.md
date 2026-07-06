# Plan — Dynamic polyglot tabbed-log dev runner in `projm run`

## Context

Today `projm run` (`crates/projm-core/src/run.rs`) resolves **one** project's dev command
and hands it to `sh -c`, inheriting stdio. It already has excellent polyglot brains —
`detect_stack()` + `resolve_dev_command()` cover 11 ecosystems (Rust, Python/uv, JS/TS,
Go, C/C++, Flutter, .NET, …) plus monorepo detection (turbo/pnpm/nx/bun) — but there is
**no TUI, no multi-app orchestration, and no process-group cleanup**.

Separately, a hand-built Bun tool `dt.ts` (copied, hardcoded, into `drive-track` and
`medlink`) gives a nice Turborepo-style TUI: detached-spawn + process-group kill, per-app
ring-buffer logs, `--list`/`--help`. Its weakness is the **hardcoded per-repo catalog** and
JS-monorepo-only assumptions.

**Goal:** fold the good parts of `dt.ts` into `projm` natively, as a **dynamic** (auto-
discovered) **polyglot** runner that manages **multiple apps inside the current repo** at
once — extended to Cargo workspaces, Python, C/C++ (CMake/Make), and Arduino/ESP32
(PlatformIO). This retires the copy-pasted `dt.ts` in favor of one binary.

**Locked decisions (from the user):**
- **Home:** Rust, inside `projm run`. No Bun/TS deliverable.
- **Layout:** **Tabbed logs** (top tab bar of apps + one full-width live log pane), *not* a sidebar.
- **Scope:** discover & run the apps **inside the current repo** (workspace members / monorepo apps).

## Approach

Add a new `runner/` submodule to `projm-core`. Leave the existing single-project inline
`run()` path **byte-for-byte unchanged** — the TUI is an additive multi-app path chosen by
mode selection. Reuse `run.rs`'s detection helpers rather than re-implementing them.

### Mode selection (`projm run`)
| Invocation | Behavior |
|---|---|
| `projm run <name>` | inline single project — **existing `run()`, unchanged** |
| `projm run --list` | discover apps, print catalog table, exit (works in non-TTY) |
| `projm run --selftest` | prove process-group kill reaps a nested child tree, exit 0/1 |
| `projm run --tui` | force the TUI (error if not a TTY) |
| `projm run --all` | TUI, auto-start every app on launch |
| `projm run` (no arg) | if repo has ≥2 runnable apps **or** is a monorepo root, and stdout is a TTY → TUI; else fall through to existing inline `run()` |

Non-TTY guard (medlink's lesson): if a TUI is requested/chosen but `!Term::stdout().is_term()`,
print a refusal pointing at `--list`, exit 1.

## Module layout

New `crates/projm-core/src/runner/`:
- `mod.rs` — entry `runner::run(...)`, mode-selection glue, non-TTY guard, `RunnableApp`, `--list` table, `self_test()`.
- `discover.rs` — `discover_apps(repo_root) -> Vec<RunnableApp>` + per-ecosystem enumerators + `extract_port`.
- `process.rs` — spawn/kill via `command-group`, reader + monitor threads, `AppRuntime`, ready-detection, `strip_ansi`.
- `tui.rs` — alt-screen/raw lifecycle, 100 ms dirty-flag render loop, frame builder, input thread, keymap.

Register with `pub mod runner;` in `crates/projm-core/src/lib.rs`.

**Reuse (visibility-only change):** promote these `run.rs` items to `pub(crate)` — no logic
change, existing tests stay green: `detect_stack`, `resolve_dev_command`, `find_package_script`,
`find_python_entry`, `detect_monorepo`, `find_monorepo_root`, `resolve_workspace_packages`,
`resolve_glob`, `load_run_config`, and the `MonorepoTool` / `ProjectStack` enums.

## Discovery (`discover.rs`)

```rust
pub struct RunnableApp {
    pub id: String, pub label: String, pub dir: PathBuf,
    pub command: String, pub port: Option<u16>, pub hint: String,
}
```
Each ecosystem enumerator runs independently and **swallows its own errors** (missing tool/
file ⇒ empty); results deduped by `(dir, command)` and sorted by `id`:
- **JS workspaces** — reuse `resolve_workspace_packages` for pnpm/bun; each pkg dir →
  `command = resolve_dev_command(dir)`, port from its `dev` script. For turbo/nx (empty
  resolver): glob `apps/*` + `packages/*` via `resolve_glob`, **and** surface root
  `package.json` `dev:*` keys as apps (`<pm> run dev:<name>`) — the case `dt.ts` handled for medlink.
- **Cargo workspace (net-new)** — parse root `Cargo.toml` `[workspace] members` (expand
  `crates/*` via `resolve_glob`); members with `[[bin]]` or `src/main.rs` → `cargo run -p <name>`.
- **Python** — subdirs with `pyproject.toml`/`requirements.txt` → `resolve_dev_command(dir)`;
  also surface `[project.scripts]` entries.
- **CMake (net-new)** — `add_executable(<name> …)` targets → `cmake --build build --target <name>`.
- **Make (net-new)** — run/dev/serve-like targets → `make <target>`.
- **PlatformIO (net-new)** — `[env:<name>]` sections → `pio run -e <name> -t upload` (hint: board).
- **Arduino (net-new)** — `*.ino` w/o `platformio.ini` → best-effort `arduino-cli compile/upload` (hint: FQBN).

`extract_port(cmd)` — pure token scan for `--port N` / `-p N` / `--port=N`; framework
defaults (vite→5173, next→3000, astro→4321) else `None`. **Best-effort; never gates startup.**

## TUI (`tui.rs`) — `console::Term` + manual ANSI, **no new TUI crate**

`console` 0.16 (already a dep) covers `size`, `move_cursor_to`, `clear_line`, `is_term`, and a
rich `read_key()` `Key` enum (`Tab`, `BackTab`, `PageUp/Down`, arrows, `Char`, `CtrlC`). Its two
gaps — no alt-screen API, `read_key` blocks with no poll — are covered by writing `\x1b[?1049h`/
`\x1b[?1049l` ourselves and running a **dedicated input thread**.

- **Render loop (main):** every 100 ms, if `AtomicBool` dirty set → lock state, build one frame
  (home `\x1b[H`, per-line `move_cursor_to` + content + `clear_line` — flicker-free like `dt.ts`),
  single `write!`, clear dirty. Compare `Term::size()` each tick for resize.
- **Layout:** row 0 inverse header (`repo ▸ dev · N/M running`); row 1 tab bar (one cell per app
  `[● label:port]`, focused inverse, dot colored by status); rows 2..h-1 full-width log pane for
  focused app; last row inverse footer hints.
- **Input thread:** `read_key()` loop → lock, mutate, set dirty. Keys: `Tab`/`BackTab` + digits
  switch tabs · `Enter`/`s` start · `x` stop · `r` restart · `a` all · `X` stop-all ·
  `PageUp`/`PageDown` scroll · `g`/`G` top/tail · `q`/`CtrlC` quit.
- **Scrub child output:** hand-rolled `strip_ansi` (CSI/OSC/single-char state machine) + drop
  control chars + `fit(line, width)` — ported from `dt.ts`, no regex crate.

## Process handling (`process.rs`) — cross-platform (projm CI runs linux/mac/windows)

**Add one dependency:** `command-group = "5"` to `crates/projm-core/Cargo.toml`. It abstracts
Unix process groups (setsid, group signals) **and** Windows Job Objects (atomic tree kill) —
strictly more correct than `taskkill /T` and avoids hand-rolled `unsafe pre_exec` + `#[cfg]`
splits. (`nix`/`windows-sys` arrive transitively; no direct crossterm/ratatui/regex/nix deps.)

- **Spawn:** each app via `command-group`, stdio piped, own group/job. `FORCE_COLOR=0` in env.
- **Stop:** group `SIGTERM` (Unix) / job terminate (Windows) → 4 s timer → if `try_wait()` still
  `None`, escalate to `SIGKILL`/`kill()`. Mirrors `dt.ts` TERM→KILL-after-4s.
- **Threads per running app:** 2 readers (`BufReader::read_line` → push ring, flip
  `starting→running` on ready-match, set dirty; lock never held across a blocking read) + 1
  monitor (`try_wait()` every ~150 ms → set stopped/errored on exit). `try_wait` (not `wait`)
  avoids wait-vs-kill ownership conflict since the child lives in the shared mutex.
- **Ready detection:** case-insensitive substring match (`ready`, `listening`, `localhost`,
  `compiled`, `started`, `running on`, `watching`) + a "quiet & alive after 4 s ⇒ running" fallback.

## Data model & concurrency

```rust
enum Status { Starting, Running, Stopped, Errored }
struct AppRuntime { status: Status, child: Option<GroupChild>, pid: Option<u32>,
                    logs: VecDeque<String> /*cap 4000*/, scroll: usize, follow: bool,
                    stopping: bool, kill_deadline: Option<Instant> }
struct TuiState { apps: Vec<RunnableApp>, runtimes: Vec<AppRuntime>, selected: usize, quitting: bool }
```
`Arc<Mutex<TuiState>>` guards mutable state; `Arc<AtomicBool>` for `dirty` so readers flag
repaints without the mutex. On quit: set `quitting`, group-kill every child, show cursor +
alt-screen off, `process::exit(0)` (reaps any `read_key`-parked input thread; OS restores terminal).

## Critical files

Modify:
- `crates/projm-core/src/run.rs` — `pub(crate)` visibility bump on reused helpers/enums; **inline path untouched**.
- `crates/projm-core/src/classify.rs` — add PlatformIO/`.ino` markers (feeds discovery + `check`).
- `crates/projm-core/src/lib.rs` — `pub mod runner;`.
- `crates/projm-core/Cargo.toml` — add `command-group = "5"`.
- `crates/projm-cli/src/main_cli.rs` — add `--tui/--list/--all/--selftest` flags to `Commands::Run`.
- `crates/projm-cli/src/main.rs` — dispatch flags into `runner::run`; keep `projm run <name>` → inline `run()`.

Create: `crates/projm-core/src/runner/{mod.rs, discover.rs, process.rs, tui.rs}`.

## Ordered steps
1. `pub(crate)` promotion in `run.rs` + `pub mod runner;` in `lib.rs`; `cargo test` green.
2. Add `command-group` to Cargo.toml.
3. `discover.rs`: `RunnableApp`, `extract_port`, JS + Cargo + Python enumerators, `discover_apps`, `--list` formatter + unit tests.
4. Add CMake/Make/PlatformIO/Arduino enumerators + tests.
5. `process.rs`: spawn/kill via `command-group`, reader+monitor threads, ready-match, `strip_ansi`, start/stop/restart/all, `self_test()`.
6. `tui.rs`: alt-screen lifecycle, render loop, frame builder, input thread, keymap.
7. `mod.rs`: mode-selection glue + non-TTY guard.
8. CLI flags in `main_cli.rs` + dispatch in `main.rs`.
9. Full `cargo test`; manual smoke; `--selftest` on all 3 OSes.

## Verification
- **Unit (tempfile, matching `run.rs` test style):** discovery per ecosystem — pnpm/bun
  workspaces, Cargo `crates/*` members, PlatformIO `[env:*]`, CMake `add_executable`, Make
  targets, root `dev:*`; plus `extract_port` cases (`--port N`, `-p N`, `--port=N`, defaults, `None`).
- **`--selftest`:** spawn a nested `sh -c "sleep 60 & sleep 60"` group, run the real stop path,
  assert `pgrep -g <pid>` shows 0 survivors → exit 0/1 (Windows: `cmd` tree + job-object kill).
- **`--list` non-TTY:** pipe stdout — must print the catalog and exit 0 without entering the TUI.
- **Regression:** existing `run.rs` test suite passes unchanged after the visibility bump.
- **Manual smoke:** run `projm run` at a real monorepo root; start/stop/restart across tabs;
  confirm quit leaves **no orphan dev servers** (`ps`/`lsof` check).

## Trickiest parts
1. Cross-platform group kill — `command-group` job objects (Win) + groups (Unix), TERM→KILL 4 s escalation, `try_wait` monitor.
2. `console` raw-mode limits — no alt-screen/poll; solved with manual `\x1b[?1049h/l` + dedicated input thread + 100 ms tick.
3. Port parsing — best-effort token scan; may be wrong, must never block startup.
