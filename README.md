# projm

Project organizer and navigator for developers. Scans a directory, classifies projects by stack, groups related ones by name prefix, and lets you fuzzy-jump to any project and open it in your editor — all from the terminal.

![version](https://img.shields.io/badge/version-0.2.0-orange)

## Install

```bash
cargo install projm
```

## Setup

Add the shell function to your zsh config:

```bash
projm init >> ~/.config/zsh/.zshrc
source ~/.config/zsh/.zshrc
```

This adds `pg` — a shell function that jumps you into a project and opens your editor.

## Usage

```bash
# Scan a directory and move projects into ~/projects/<category>/
projm organize ~/Downloads/dump

# Preview without moving anything
projm organize ~/Downloads/dump --dry-run

# Fuzzy-pick a project, choose an editor, jump there
pg

# Override the default base directory (~/projects)
projm set-base ~/code

# List detected editors on this machine
projm editors
```

## How it works

### Project classification

Projects are classified by inspecting their contents:

| Marker                                                    | Category              |
| --------------------------------------------------------- | --------------------- |
| `doc-lab.md` present                                      | `labs`                |
| `memory.x` / `openocd.cfg` / embedded Cargo target        | `embedded`            |
| `src-tauri/` or both `Cargo.toml` + `package.json`        | `apps`                |
| `pubspec.yaml` (Flutter/Dart)                             | `apps` or `ui`        |
| `build.gradle` / `build.gradle.kts` (Kotlin/Android)      | `apps`                |
| `*.xcodeproj` / `Package.swift` (Swift/iOS)               | `apps`                |
| `go.mod` (Go)                                             | `services` or `tools` |
| `pom.xml` / `build.gradle` (Java)                         | `services`            |
| `uv.lock` / `pyproject.toml` + ML markers                 | `ml`                  |
| `package.json` — reads deps to detect frontend vs backend | `ui` / `services`     |
| `Cargo.toml` only                                         | `services` or `tools` |
| Everything else                                           | `labs`                |

`package.json` projects are classified by reading actual dependencies — React/Vite/Svelte → `ui`, Hono/Express/Prisma → `services`, both → `apps`.

### Package manager detection

`projm` reads the lockfile present in the project root to identify the package manager — no guessing from `package.json` scripts:

| Lockfile                 | Package manager    |
| ------------------------ | ------------------ |
| `pnpm-lock.yaml`         | pnpm               |
| `bun.lockb` / `bun.lock` | bun                |
| `yarn.lock`              | yarn               |
| `package-lock.json`      | npm                |
| `uv.lock`                | uv (Python)        |
| `Pipfile.lock`           | pipenv             |
| `poetry.lock`            | poetry             |
| `Cargo.lock`             | cargo              |
| `go.sum`                 | go modules         |
| `pubspec.lock`           | pub (Dart/Flutter) |
| `Gemfile.lock`           | bundler (Ruby)     |
| `composer.lock`          | composer (PHP)     |

### Name-based grouping

Projects that share a prefix and a known suffix are grouped under a common folder:

```
~/projects/
└── apps/
    └── drivetrack/          ← group folder
        ├── drivetrack/      ← standalone root (no suffix)
        ├── drivetrack-api/
        ├── drivetrack-web/
        └── drivetrack-desk/
```

Recognised suffixes: `api`, `web`, `mob`, `desk`, `mono`, `backend`, `frontend`, `server`, `client`, `cli`, `bot`, `admin`, `dashboard`, `landing`, `docs`, and more.

### Labs marker

Drop a `doc-lab.md` file in any project to force it into `labs/` regardless of its stack:

```bash
touch my-experiment/doc-lab.md
projm organize ~/projects
```

### `pg` — fuzzy project jump

`pg` calls `projm g` internally. All interactive UI writes to stderr, only the final shell command goes to stdout for `eval`. Uses `z` (zoxide) if available, falls back to `cd`.

```
  apps      drivetrack-api        main  ✓
  apps      drivetrack-web        feat/auth  *
  ml        trashnet              main  ✓
  embedded  rocket-telemetry-fw   dev  *
  ui        pioneers-website      main  ✓
```

### Editor detection

projm scans `$PATH` at runtime — only editors that are actually installed appear in the picker. The list of editors it knows about:

| Binary   | Name     |
| -------- | -------- |
| `nvim`   | Neovim   |
| `zed`    | Zed      |
| `code`   | VS Code  |
| `kiro`   | Kiro     |
| `hx`     | Helix    |
| `idea`   | IntelliJ |
| `cursor` | Cursor   |
| `emacs`  | Emacs    |
| `vim`    | Vim      |

**Selection behaviour:**

- **0 found** → error with install hint
- **1 found** → opens directly, no picker shown
- **2+ found** → interactive picker, last choice pre-selected

Last choice is remembered per-project in `~/.config/projm/prefs.json`. Run `projm editors` to see what's detected on your machine.

## Directory structure

```
~/projects/
├── apps/        # Full-stack, Tauri, Flutter, Android, iOS, monorepos
├── services/    # Backend APIs — Rust, Hono, Go, Java, Express
├── ui/          # Frontend-only — React, Svelte, Vue
├── embedded/    # ESP32, LoRa, no_std Rust
├── ml/          # ML pipelines, notebooks
├── tools/       # CLI tools, scripts
└── labs/        # Experiments, anything with doc-lab.md
```

---

## Roadmap

### ~~v0.2 — Auto-detect editors~~ ✓ shipped

Editor detection, single-editor fast path, and per-project last-choice memory are all live. See [Editor detection](#editor-detection) above.

---

### v0.3 — Shell completions + zoxide setup

**Shell completions**

Generated via Clap at build time for zsh, bash, and fish:

```bash
projm completions zsh >> ~/.config/zsh/completions/_projm
```

Completions cover all subcommands and flags. `projm init` installs them automatically alongside the shell function.

**Auto-install & setup zoxide**

When `projm init` runs, detect and handle zoxide automatically:

```
[1/3] checking zoxide... not found
[2/3] installing zoxide via pacman -S zoxide
[3/3] writing eval to ~/.config/zsh/.zshrc

  done. restart your shell or: source ~/.config/zsh/.zshrc
```

Package manager detection order (system-native preferred over cargo):

| OS            | Tried in order                      |
| ------------- | ----------------------------------- |
| Arch Linux    | `pacman` → `yay` → `paru` → `cargo` |
| Ubuntu/Debian | `apt` → `cargo`                     |
| macOS         | `brew` → `cargo`                    |
| Fallback      | `cargo install zoxide`              |

Appends `eval "$(zoxide init zsh)"` to `.zshrc` if not already present. Fully idempotent.

---

### v0.4 — `projm new` + package manager detection

Scaffold a new project directly into the right place:

```bash
projm new drivetrack-mobile
# detects prefix "drivetrack" already in apps/drivetrack/
# → creates ~/projects/apps/drivetrack/drivetrack-mobile/
# → asks: stack? (rust / hono / react / tauri / flutter / uv)
# → runs the right init command with the right package manager
# → touches doc-lab.md if you pass --lab
```

Stack → init command mapping:

| Stack     | Command                             |
| --------- | ----------------------------------- |
| Rust      | `cargo init`                        |
| Hono/TS   | `pnpm create hono` (or detected pm) |
| React     | `pnpm create vite`                  |
| Tauri     | `pnpm create tauri-app`             |
| Flutter   | `flutter create`                    |
| Python/uv | `uv init`                           |
| Go        | `go mod init`                       |

**Package manager detection**

The right install command is determined from the lockfile in the project root — no guessing from `package.json` scripts:

| Lockfile                            | Package manager    |
| ----------------------------------- | ------------------ |
| `pnpm-lock.yaml`                    | pnpm               |
| `bun.lockb` / `bun.lock`            | bun                |
| `yarn.lock`                         | yarn               |
| `package-lock.json`                 | npm                |
| `uv.lock`                           | uv (Python)        |
| `poetry.lock`                       | poetry             |
| `Pipfile.lock`                      | pipenv             |
| `requirements.txt`                  | pip                |
| `Cargo.lock`                        | cargo              |
| `go.sum`                            | go modules         |
| `pubspec.lock`                      | pub (Dart/Flutter) |
| `build.gradle` / `build.gradle.kts` | gradle             |
| `pom.xml`                           | maven              |
| `Gemfile.lock`                      | bundler (Ruby)     |
| `composer.lock`                     | composer (PHP)     |

Also shown in `projm g` as project metadata.

---

### v0.5 — Custom classification rules

User-defined rules in `~/.config/projm/rules.toml`, evaluated before built-in logic:

```toml
# Rules are evaluated top to bottom, first match wins

[[rule]]
name     = "pioneers-website"  # exact name match
category = "ui"

[[rule]]
marker   = "rocket.toml"       # file presence (replaces the doc-lab.md hard override)
category = "services"

[[rule]]
name_contains = "adrar"        # substring match
category      = "labs"

[[rule]]
suffix   = "fw"                # override built-in suffix behaviour
category = "embedded"

[[rule]]
has_dep  = "burn"              # Cargo.toml dep → Rust ML
category = "ml"

[[rule]]
has_dep  = "tensorflow"        # package.json or requirements.txt dep
category = "ml"
```

Evaluation order:

```
1. rules.toml       ← your custom rules (highest priority)
2. built-in logic   ← everything else
```

> **Tip:** `doc-lab.md` is just a built-in rule. You can replicate its behaviour for any marker via `marker = "doc-lab.md"` with the highest position in your rules file.

---

### v0.6 — Universal language support

Extend classification to cover every major stack:

| Marker                                 | Language / Stack          | Category             |
| -------------------------------------- | ------------------------- | -------------------- |
| `pubspec.yaml` + `android/` or `ios/`  | Flutter                   | `apps`               |
| `pubspec.yaml` only                    | Dart package              | `ui`                 |
| `build.gradle` + `AndroidManifest.xml` | Kotlin / Android          | `apps`               |
| `build.gradle` without Android markers | Spring Boot / JVM backend | `services`           |
| `pom.xml`                              | Java / Maven              | `services`           |
| `*.xcodeproj` / `Package.swift`        | Swift / iOS / macOS       | `apps`               |
| `go.mod`                               | Go service or CLI         | `services` / `tools` |
| `Gemfile` + `config/routes.rb`         | Ruby on Rails             | `services`           |
| `composer.json` + `artisan`            | Laravel / PHP             | `services`           |
| `mix.exs`                              | Elixir / Phoenix          | `services`           |
| `*.csproj` / `*.sln`                   | C# / .NET                 | `services` / `apps`  |
| `CMakeLists.txt` + `.ld` / `openocd`   | C / C++ embedded          | `embedded`           |
| `CMakeLists.txt` only                  | C / C++ native app or lib | `tools`              |

All new stacks respect the same grouping rules — `myapp-android` and `myapp-ios` will group under `myapp/`.

---

### v0.7 — `projm run`

Detect and execute the project's dev command without leaving projm:

```bash
projm run
# reads lockfile → infers package manager
# reads package.json scripts / Cargo.toml / pyproject.toml
# → picks the right dev command and runs it
```

Dev command resolution per stack:

| Stack       | Command                     |
| ----------- | --------------------------- |
| Rust binary | `cargo run`                 |
| Hono/TS     | `bun dev` / `pnpm dev`      |
| React/Vite  | `bun dev` / `pnpm dev`      |
| Tauri       | `pnpm tauri dev`            |
| Flutter     | `flutter run`               |
| Python/uv   | `uv run` / `python main.py` |
| Go          | `go run .`                  |

Pairs naturally with `pg` — jump to a project and optionally start it in one step:

```bash
pg --run
```

---

### Summary

| Version | Feature                                                          | Status    |
| ------- | ---------------------------------------------------------------- | --------- |
| v0.2    | Auto-detect installed editors + remember last choice             | ✓ shipped |
| v0.3    | Shell completions (zsh/bash/fish) + auto-install zoxide          | planned   |
| v0.4    | `projm new` scaffold + package manager detection from lockfile   | planned   |
| v0.5    | `rules.toml` custom classification                               | planned   |
| v0.6    | Universal language support (Flutter, Kotlin, Go, Swift, Java, …) | planned   |
| v0.7    | `projm run` — detect and launch the project's dev command        | planned   |

---

## License

MIT
