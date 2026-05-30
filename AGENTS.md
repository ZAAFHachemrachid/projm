# PROJM KNOWLEDGE BASE

**Generated:** 2026-05-30 00:33 UTC
**Commit:** 52ffb7a
**Branch:** main

## OVERVIEW

Project organizer + navigator for developers. Scans directories, classifies projects by stack markers, groups related ones by name prefix, and provides fuzzy-jump navigation (`pg`) + dev command launching (`pn`). Rust CLI + Tauri desktop GUI + Next.js 16 frontend.

## STRUCTURE

```
./
├── crates/
│   ├── projm-core/     # Core library: classify, organize, go, run, check, rules, clone, config, editors, prefs
│   └── projm-cli/      # CLI binary: dispatch, init_setup, blueprints, completions
├── src-tauri/           # Tauri backend: PTY terminal, Tauri commands, project listing
├── app/                 # Next.js 16 app (Tauri webview): shadcn/ui, React 19
├── docs/superpowers/    # Design specs for shipped/future features
└── .agents/             # Project-level opencode skills
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Project classification logic | `crates/projm-core/src/classify.rs` | Sequential match against stack markers |
| File organization moves | `crates/projm-core/src/organize.rs` | Plan-based moves, grouping logic |
| Fuzzy jump (`pg`) | `crates/projm-core/src/go.rs` | zoxide-backed project picker |
| Dev command detection (`pn`) | `crates/projm-core/src/run.rs` | Stack detection → command resolution |
| Environment diagnostics | `crates/projm-core/src/check.rs` | `projm check` — PATH tool scanning |
| Custom rules | `crates/projm-core/src/rules.rs` | `rules.toml` parsing + evaluation |
| Git clone | `crates/projm-core/src/clone.rs` | URL parsing, git clone, auto-organize |
| Editor detection | `crates/projm-core/src/editors.rs` | KNOWN_EDITORS → which on PATH |
| Config / prefs | `crates/projm-core/src/config.rs`, `prefs.rs` | `~/.config/projm/config.json`, `prefs.json` |
| CLI dispatch | `crates/projm-cli/src/main.rs` + `main_cli.rs` | Clap-based subcommand routing |
| Shell init | `crates/projm-cli/src/init_setup.rs` | Shell integration, zoxide auto-install |
| Blueprints | `crates/projm-cli/src/blueprints.rs` | Interactive project scaffolding |
| Tauri commands | `src-tauri/src/lib.rs` | PTY terminal, project listing, scan invoke |
| Next.js pages | `app/app/*/page.tsx` | 5 pages: main, projects, scan, settings, diagnostics |
| UI components | `app/components/ui/` | shadcn/ui: sidebar, terminal, select, sheet, etc. |
| Design specs | `docs/superpowers/specs/` | 13 ADR-style specs for v0.3–v0.7 features |
| CONTEXT.md | `./CONTEXT.md` | Domain language (Project, Category, Group Folder, etc.) |

## CODE MAP

| Symbol | Type | Location | Role |
|--------|------|----------|------|
| `Category` | enum | `classify.rs` | 8 categories + Custom(String) |
| `classify()` | fn | `classify.rs` | Stack-marker → Category |
| `split_suffix()` | fn | `classify.rs` | Name prefix/suffix extraction |
| `Organizer::run()` | fn | `organize.rs` | Scan → plan → move |
| `go::run()` | fn | `go.rs` | Fuzzy jump picker |
| `run::run()` | fn | `run.rs` | Dev command detection + execution |
| `check::run()` | fn | `check.rs` | PATH tool health scan |
| `CustomRule` | struct | `rules.rs` | User-defined classification rule |
| `Config` | struct | `config.rs` | Base path + category list |
| `Prefs` | struct | `prefs.rs` | Per-project editor preference |
| `InstalledEditor` | struct | `editors.rs` | Editor binary + path |
| `extract_repo_name()` | fn | `clone.rs` | Git URL → repo name |
| `ProjectItem` | struct | `src-tauri/lib.rs` | Tauri project list item |
| `TerminalState` | struct | `src-tauri/lib.rs` | PTY terminal state |
| `PackageManager` | enum | `run.rs` | Bun/Pnpm/Yarn/Npm |
| `ProjectStack` | enum | `run.rs` | 14 supported stacks |
| `MonorepoTool` | enum | `run.rs` | Turbo/PnpmWorkspace/Nx/BunWorkspace |

## CONVENTIONS

- Rust workspace with `resolver = "2"` — 3 crate members
- All user-facing output uses `colored` for styling (no raw ANSI)
- Interactive prompts use `dialoguer` with `ColorfulTheme`
- Errors propagate via `anyhow::Result` / `bail!`
- Module files use `// ── Section header ──` separators
- `--dry-run` flag on `organize` — plan printed, no files moved
- Tests: integration tests in `crates/projm-core/tests/`, not in-module `#[cfg(test)]`
- Tauri commands: `#[tauri::command]` with `serde::Serialize` response types
- Next.js: app router, `dark` class on `<html>`, Geist font
- Config: JSON (`~/.config/projm/config.json`), Prefs: JSON (`~/.config/projm/prefs.json`), Rules: TOML (`~/.config/projm/rules.toml`)
- `projm init` generates shell functions + completions + installs zoxide
- GitHub Releases via `cargo-dist` CI in `.github/workflows/release.yml`

## ANTI-PATTERNS (THIS PROJECT)

- Do NOT use `#[cfg(test)]` in source files — tests go in `tests/` only
- Do NOT hardcode paths — use `dirs::config_dir()`, `dirs::home_dir()`
- Do NOT add new categories without updating `Category::all()` and `classify.rs`
- Do NOT use `unwrap()` outside of tests — prefer `?` + `anyhow::Context`
- Do NOT add new dependencies without checking workspace `Cargo.toml` first
- Do NOT break the `pg`/`pn` shell contract (eval-based stdout, stderr for UI)

## COMMANDS

```bash
cargo build              # Build all workspace crates
cargo run -- <args>      # Run CLI with args
cargo test               # Run all tests in workspace
cargo clippy             # Lint all crates
npm run dev              # Next.js dev (in app/)
cargo tauri dev          # Tauri desktop dev mode
```

## NOTES

- `projm-core` is the shared library — `projm-cli` and `src-tauri` both depend on it
- `orgranize` creates a Move plan first, then executes — never moves in-place
- `run.rs` is the largest file (1376 lines) — consider splitting
- `app/page.tsx` is 44KB — the main Tauri GUI surface
- Config path precedence: `~/.config/projm/config.json` → defaults
- Rules evaluated top-to-bottom, first match wins — before built-in logic
