# PROJM-CORE

Shared library for `projm` — project classification, organization, navigation, and dev command detection.

## STRUCTURE

```
src/
├── lib.rs         # pub mod declarations (10 lines)
├── classify.rs    # Category enum, classify(), split_suffix(), Category::all()
├── organize.rs    # Organizer::run_with_base(), plan-based moves, prefix grouping
├── go.rs          # Fuzzy jump picker, git info, zoxide wrapper
├── run.rs         # Dev command resolution (1376 lines — largest)
├── check.rs       # PATH tool scanning, 20+ tools, 5 categories
├── rules.rs       # CustomRule parsing, rules.toml eval, first-match-wins
├── clone.rs       # Git URL parsing, extract_repo_name(), clone + auto-organize
├── config.rs      # Config struct, JSON persistence (~/.config/projm/config.json)
├── editors.rs     # KNOWN_EDITORS, detect_installed() via which
└── prefs.rs       # Prefs struct, per-project editor memory (HashMap)
tests/             # 6 integration tests (no #[cfg(test)] in src/)
```

## WHERE TO LOOK

| Module | Key Symbol | Role |
|--------|-----------|------|
| `classify` | `Category` (8 variants + Custom) | Stack-marker → category |
| `classify` | `classify()` | Sequential match, returns (Category, suffix) |
| `classify` | `split_suffix()` | Dash/underscore prefix/suffix extraction |
| `organize` | `run_with_base()` | Scan → Move plan → execute | 
| `organize` | `Move { src, dest, cat, group }` | Single move operation |
| `go` | `run()` | Fuzzy select → cd + open editor |
| `go` | `Project { name, path, category, git_info }` | Listed project |
| `run` | `ProjectStack` (14 variants) | Auto-detected stack |
| `run` | `PackageManager` (4 variants) | Lockfile-based |
| `run` | `MonorepoTool` (4 variants) | Turbo/Pnpm/Nx/Bun |
| `check` | `Tool { name, binary, category, version_args }` | Diagnosable tool |
| `rules` | `CustomRule { name, marker, name_contains, suffix, has_dep }` | User rule |
| `clone` | `extract_repo_name()` | URL → name |
| `config` | `Config { base, categories }` | Persistent settings |
| `editors` | `InstalledEditor { binary, name, path }` | PATH-detected editor |
| `prefs` | `Prefs { last_editor, last_project }` | Editor memory |

## CONVENTIONS

- Section headers: `// ── Section name ──`
- Errors: `anyhow::Result` / `bail!()` / `.context()`
- UI: `colored` for styling, `dialoguer::FuzzySelect`/`Select` with `ColorfulTheme`
- No `#[cfg(test)]` in source — tests in `tests/` only
- `run.rs` is 1376 lines — the complexity hotspot
- Integration tests use real temp dirs via `tempfile`

## ANTI-PATTERNS

- No `unwrap()` outside tests — use `?` + `Context`
- No raw ANSI escape codes — use `colored` crate
- No hardcoded paths — use `dirs::config_dir()`, `dirs::home_dir()`
- `run.rs` should be split when adding new stacks
