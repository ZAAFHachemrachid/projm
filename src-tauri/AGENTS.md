# SRC-TAURI

Tauri v2 desktop backend for `projm` — PTY terminal, project listing, scan commands.

## STRUCTURE

```
src/
├── lib.rs  (311 lines) — TerminalState, Tauri commands, project listing logic
└── main.rs (186 bytes)  — tauri::Builder launch
capabilities/           — Tauri permission capabilities
gen/schemas/            — Generated JSON schemas
icons/                  — App icons (.png, .svg, .icns, .ico)
```

Frontend lives in `../app/` (Next.js 16 webview, separate AGENTS.md).

## WHERE TO LOOK

| Symbol | Type | Role |
|--------|------|------|
| `TerminalState` | struct | PTY state: writer (tokio::Mutex), parser for terminal I/O via `portable-pty` |
| `ProjectItem` | struct | Serialized project: name, path, category, git_branch, git_dirty |
| `list_projects()` | Tauri command | Scan base dir → collect projects with git info |
| `scan_directory()` | Tauri command | Run organize scan + emit events |
| `get_git_info()` | fn helper | git branch --show-current + dirty-check via git status --porcelain |
| `is_group_folder()` | fn helper | Detect group folders by child naming pattern |

## CONVENTIONS

- `#[tauri::command]` with `serde::Serialize` response types
- PTY terminal via `portable-pty` crate (native pty fork)
- State managed via `tauri::State<TerminalState>` with `Arc<Mutex<>>`
- Git info fetched by spawning `git` subprocesses
- Frontend-backend IPC via Tauri events (`Emitter` trait)

## ANTI-PATTERNS

- Do NOT add UI rendering here — frontend is in `../app/`
- Do NOT use `portable_pty` for non-terminal operations
- Do NOT add blocking I/O on the Tauri command thread — use async or spawn
