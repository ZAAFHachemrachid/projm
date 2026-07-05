# projm

Project organizer and navigator for developers. Scans a directory, classifies projects by stack, groups related ones by name prefix, and lets you fuzzy-jump to any project and open it in your editor — all from the terminal.

![version](https://img.shields.io/badge/version-0.7.4-blue)

## Install

### 1. Quick Install Scripts (Recommended)

To install `projm` via pre-compiled binaries, you can run the interactive installation script.

**For macOS and Linux (Shell):**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ZAAFHachemrachid/projm/releases/latest/download/projm-installer.sh | sh
```

**For Windows (PowerShell):**
```powershell
irm https://github.com/ZAAFHachemrachid/projm/releases/latest/download/projm-installer.ps1 | iex
```

### 2. Pre-compiled Binaries & cargo-binstall

You can download the binaries directly from [GitHub Releases](https://github.com/ZAAFHachemrachid/projm/releases).

Alternatively, install via **`cargo-binstall`**:
```bash
cargo install cargo-binstall
cargo binstall projm
```

### 3. From Source

If you have the Rust toolchain installed, you can build and install `projm` directly from crates.io:

```bash
cargo install projm
```

## Setup

Add the shell functions to your zsh config:

```bash
projm init
source ~/.config/zsh/.zshrc
```

This automatically installs completions, sets up zoxide, and adds two shell functions:

- **`pg`** — fuzzy-jump to a project and open your editor
- **`pn`** — fuzzy-pick a project and run its dev command directly

> [!TIP]
> **Avoid Command Collisions:** If `pg` conflicts with another command on your system (e.g. PostgreSQL or `pgcli`), you can customize the generated shell function name using the `--alias` (or `-a`) flag:
> ```bash
> projm init --alias pj
> ```

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

# Generate shell completions
projm completions zsh
projm completions powershell

# Install shell integration + zoxide + completions
projm init

# Run the current project's dev command directly
projm run

# Fuzzy-pick a project and run its dev command
pn

# Run dev command for current directory
pn .

# Run dev command for a specific path
pn /path/to/project

# Verify active development tools and environment health
projm check
```

## How it works

### Project classification

Projects are classified by inspecting their contents:

| Marker                                                    | Category              |
| --------------------------------------------------------- | --------------------- |
| `doc-lab.md` present                                      | `labs`                |
| `memory.x` / `openocd.cfg` / embedded Cargo target        | `embedded`            |
| Monorepo markers (`turbo.json`, `pnpm-workspace.yaml`, `lerna.json`, `nx.json`, or `"workspaces"` in `package.json`) | `apps` |
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

### Adding support for new languages

Support for new language stacks is added directly in `src/classify.rs`. To add another stack:
1. **Identify the unique stack markers** (e.g. `mix.exs` for Elixir, `composer.json` for PHP).
2. **Add classification check** in the sequential `classify()` flow. If the stack can map to multiple categories, define helper functions for extra validation (e.g. `has_android_manifest()` or checking for subdirectories like `android/` and `ios/`).
3. **Verify with tests** by adding unit test cases to `tests/classify_tests.rs` matching your stack.

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

`pg` calls `projm g` internally (or your custom alias if configured via `projm init --alias <name>`). All interactive UI writes to stderr, only the final shell command goes to stdout for `eval`. Uses `z` (zoxide) if available, falls back to `cd`.

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
| `zeditor` | Zed      |
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

### Environment diagnostics

`projm check` scans your active `$PATH` and runs diagnostics to check the health and version of your compilers, runtimes, package managers, and development utilities:

- **Rust**: `cargo`, `rustc`, `rustup`
- **Python**: `python`/`python3`, `pip`, `uv`, `pipx`
- **Node/JS**: `node`, `npm`, `pnpm`, `yarn`, `bun`, `deno`
- **Go**: `go`
- **Systems/VCS**: `git`, `docker`, `docker-compose`, `curl`, `make`

It performs smart cross-dependency validation (e.g. warning you if a package manager like `npm` is present but its runtime `node` is missing). Run `projm check` to see your environment status.

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

### ~~v0.3 — Shell completions + zoxide setup~~ ✓ shipped

**Shell completions**

Generated via Clap for zsh, bash, fish, and PowerShell:

```bash
projm completions zsh >> ~/.config/zsh/completions/_projm
projm completions powershell > ~/.config/powershell/completions/projm.ps1
```

Completions cover all subcommands and flags. `projm init` installs platform-specific completions automatically.

**Auto-install & setup zoxide**

When `projm init` runs, it checks zoxide and handles install automatically if missing:

```
[1/3] checking zoxide...
[2/3] writing completions...
[3/3] updating ~/.zshrc...

  done. restart your shell or source ~/.zshrc
```

Package manager detection order (system-native preferred over cargo):

| OS            | Tried in order                      |
| ------------- | ----------------------------------- |
| Arch Linux    | `pacman` → `yay` → `paru` → `cargo` |
| Ubuntu/Debian | `apt` → `cargo`                     |
| macOS         | `brew` → `cargo`                    |
| Windows       | `winget` → `choco` → `scoop` → `cargo` |
| Fallback      | `cargo install zoxide`              |

On Linux/macOS, `init` updates `~/.zshrc` with zsh integration and `eval "$(zoxide init zsh)"` (idempotent). On Windows, `init` updates `~/Documents/PowerShell/Microsoft.PowerShell_profile.ps1` with PowerShell integration and `zoxide init powershell` (idempotent).

---

### ~~v0.3.2 — Interactive Project Blueprints~~ ✓ shipped

Save complex project creation commands with placeholders (e.g., `cargo new {name}`) and execute them interactively. After execution, `projm` prompts you to automatically run `organize` to move the new project into the correct category directory inside your **Base Directory**.

```bash
# Add a new blueprint interactively
projm blueprint add

# List all saved blueprints
projm blueprint list

# Run a blueprint to scaffold a new project
projm blueprint run
```

---

### ~~v0.4 — `projm check` (Environment Diagnostics)~~ ✓ shipped

A diagnostic subcommand that scans the local machine's `$PATH` to identify and verify the health, location, and versions of active compilers, runtimes, package managers, and development utilities:

- **Rust Toolchain**: `cargo`, `rustc`, `rustup`
- **Python Toolchain**: `python`/`python3`, `pip`, `uv`, `pipx`
- **Node/JS Toolchain**: `node`, `npm`, `pnpm`, `yarn`, `bun`, `deno`
- **Go Toolchain**: `go`
- **Systems & Utilities**: `git`, `docker`, `docker-compose`, `curl`, `make`

It performs smart cross-dependency validation (e.g. warning you if a package manager like `npm` is present but its runtime `node` is missing).

```bash
# Run environment diagnostics
projm check
```

---

### ~~v0.5 — Custom classification rules~~ ✓ shipped

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

### ~~v0.6 — Universal language support~~ ✓ shipped

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

### ~~v0.7 — `projm run` + `pn`~~ ✓ shipped

Auto-detect the project stack, predict the dev command, and run it directly — no need to leave the terminal.

**Two entry points:**

```bash
projm run           # detect + run dev command for current directory
projm run .         # same (explicit CWD)
projm run <path>    # detect + run dev command for a given path
projm run <name>    # match a project name or fuzzy-pick → run it

pn                  # fuzzy-pick a project from the organised base → run it
pn .                # run dev command for current directory
pn /path            # run dev command for a specific path
pn <query>          # pre-filtered picker or direct match → run
```

Instead of opening an editor (like `pg`), `pn` runs the project's dev command directly — streaming stdout/stderr to the terminal, with Ctrl+C support.

**Dev command resolution:**

| Detected Stack      | Command                              |
| ------------------- | ------------------------------------ |
| Rust (Cargo.toml)   | `cargo run`                          |
| Bun                 | `bun run dev`                        |
| pnpm                | `pnpm dev`                           |
| Yarn                | `yarn dev`                           |
| npm                 | `npm run dev`                        |
| Tauri               | `<pkg-manager> tauri dev`            |
| Flutter             | `flutter run`                        |
| Go                  | `go run .`                           |
| Python + uv.lock    | `uv run python main.py`              |
| Python (pip)        | `python main.py` / `python3 main.py` |
| Ruby on Rails       | `bin/rails server`                   |
| Elixir/Phoenix      | `mix run --no-halt`                  |
| Kotlin/Java Gradle  | `./gradlew run`                      |
| Java Maven          | `mvn compile exec:java`              |
| Laravel/PHP         | `php artisan serve`                  |
| C/C++ (CMake)       | `cmake --build build`                |
| C# / .NET           | `dotnet run`                         |

Package.json scripts are read to find `dev` → `start` → fallback.

**`.projm.toml` override:**

Place a config file in the project root to override auto-detection:

```toml
[run]
command = "cargo run --features embedded"     # full override

[run.scripts]
dev   = "cargo run"
build = "cargo build --release"
test  = "cargo test"

[run.workspace_packages]                       # label monorepo packages
api  = "packages/api"
web  = "apps/web"
```

**Monorepo support:**

Detects Turborepo, pnpm workspaces, Nx, and Bun workspaces. Shows an interactive picker:

```
  Select target (turbo):
  > ▶ Run All
    ◻ apps/web
    ◻ apps/api
    ◻ packages/shared
    ◻ packages/ui
```

Auto-resolves workspace packages from `pnpm-workspace.yaml`, bun workspaces, or package.json workspaces. Individual packages can be labelled via `.projm.toml`.

**Shell function:**

Generated automatically by `projm init` alongside `pg`. No `eval` needed — `pn` spawns the dev command directly:

```zsh
pn() {
    projm run "$@"
}
```

---

### Summary

| Version | Feature                                                          | Status    |
| ------- | ---------------------------------------------------------------- | --------- |
| v0.2    | Auto-detect installed editors + remember last choice             | ✓ shipped |
| v0.3    | Shell completions (zsh/bash/fish/powershell) + auto-install zoxide | ✓ shipped |
| v0.3.2  | Interactive blueprints (`projm blueprint`) + auto-organization   | ✓ shipped |
| v0.4    | `projm check` Environment diagnostics and doctor mode           | ✓ shipped |
| v0.5    | `rules.toml` custom classification                               | ✓ shipped |
| v0.6    | Universal language support (Flutter, Kotlin, Go, Swift, Java, …) | ✓ shipped |
| v0.7    | `projm run` + `pn` — detect and launch the project's dev command  | ✓ shipped |

---

## License

MIT
