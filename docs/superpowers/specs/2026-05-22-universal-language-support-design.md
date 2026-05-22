# Universal Language Support Design Specification

Extend classification in `projm` to cover every major stack, ensuring projects are organized into correct Category directories based on their files and configurations.

## Overview

In `projm` v0.6, we extend the project classification system to support Flutter, Dart, Kotlin/Android, Spring Boot, Java/Maven, Swift/iOS/macOS, Go, Ruby on Rails, PHP/Laravel, Elixir/Phoenix, C#/.NET, C/C++ embedded, and C/C++ native.

## Precedence and Ordering Flow

Classification runs sequentially in `classify()` inside `src/classify.rs`:

1. **Custom Rules**: Checked first via `rules.toml` definitions.
2. **Explicit Labs Override**: Checked next via `doc-lab.md`.
3. **Monorepos**: `turbo.json`, `pnpm-workspace.yaml`, `lerna.json`, `nx.json`, or `"workspaces"` in `package.json` $\rightarrow$ `apps`.
4. **Suffix-based Overrides**: Standard suffixes like `-fw` $\rightarrow$ `embedded`, `-web` $\rightarrow$ `ui`, etc.
5. **Embedded Stacks**:
   * Rust Embedded: `memory.x`, `openocd.cfg`, `.probe-rs`, or embedded target in `.cargo/config.toml`.
   * C/C++ Embedded: `CMakeLists.txt` AND (linker script `*.ld` or `openocd.cfg` or `openocd` folder).
6. **Stack-specific Classifiers**:
   * **Tauri / Fullstack Rust**: `src-tauri` or (`Cargo.toml` + `package.json`) $\rightarrow$ `apps`.
   * **Flutter / Dart**:
     * `pubspec.yaml` + (`android/` or `ios/` folder) $\rightarrow$ `apps`.
     * `pubspec.yaml` only $\rightarrow$ `ui`.
   * **Kotlin / Android**: `build.gradle` (or `build.gradle.kts`) + `AndroidManifest.xml` (in common locations or via depth-limited search) $\rightarrow$ `apps`.
   * **Spring Boot / JVM Backend**: `build.gradle` (or `build.gradle.kts`) only (without `AndroidManifest.xml`) $\rightarrow$ `services`.
   * **Java / Maven**: `pom.xml` $\rightarrow$ `services`.
   * **Swift / iOS / macOS**: `*.xcodeproj` or `Package.swift` $\rightarrow$ `apps`.
   * **Go**: `go.mod` $\rightarrow$ `tools` (if name contains `"cli"`, `"tool"`, `"util"`) or `services` (default).
   * **Ruby on Rails**: `Gemfile` + `config/routes.rb` $\rightarrow$ `services`.
   * **Laravel / PHP**: `composer.json` + `artisan` $\rightarrow$ `services`.
   * **Elixir / Phoenix**: `mix.exs` $\rightarrow$ `services`.
   * **C# / .NET**: `*.csproj` or `*.sln` $\rightarrow$ `apps` (if name contains `"app"`, `"ui"`, `"desktop"`, `"mobile"`) or `services` (default).
   * **C / C++ Native (Non-embedded)**: `CMakeLists.txt` only $\rightarrow$ `tools`.
   * **Python (Existing)**: `requirements.txt`/`pyproject.toml`/`setup.py` $\rightarrow$ `ml` (if ML markers present) or `tools` (default).
   * **Node/JS (Existing)**: `package.json` $\rightarrow$ `ui`/`services`/`apps` based on package kinds.
   * **Rust (Existing)**: `Cargo.toml` $\rightarrow$ `tools` (if name contains `"cli"`, `"tool"`, `"util"`) or `services` (default).
7. **Fallback**: `labs`.

## Technical Helpers

* **`has_android_manifest(path: &Path) -> bool`**
  Checks common paths first (e.g., `src/main/AndroidManifest.xml`) and performs a depth-limited search up to 4 levels.
* **`is_c_cpp_embedded(path: &Path) -> bool`**
  Checks if `CMakeLists.txt` is present and if either a `.ld` file or `openocd` folder/config exists.
