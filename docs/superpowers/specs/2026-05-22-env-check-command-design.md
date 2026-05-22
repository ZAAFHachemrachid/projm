# Design Spec: `projm check` Environment Diagnostic Subcommand

This specification outlines the design and implementation of the `projm check` command, which scans and displays diagnostics for package managers, runtimes, container tools, and build utilities in the active user environment.

## User Review Required

> [!NOTE]
> The `check` command will execute external binaries (e.g., running `node --version`, `cargo --version`) with standard process execution. Since this runs locally under user permissions, it has no side-effects other than minor process invocation overhead.

> [!IMPORTANT]
> The check command will categorize tools cleanly into Toolchains and provide helpful warning flags if counterpart dependencies are out-of-sync (e.g., `npm` or `pnpm` is present but `node` is missing, or `cargo` is present but `rustc` is missing).

## Proposed Changes

### CLI Interface

We will add the `check` command as a top-level subcommand in `projm`:

```bash
projm check
```

This will run a diagnostic suite across several defined groups:

1. **Rust Toolchain**: `cargo`, `rustc`, `rustup`
2. **Python Toolchain**: `python` (or `python3`), `pip`, `uv`, `pipx`
3. **Node/JS Toolchain**: `node`, `npm`, `pnpm`, `yarn`, `bun`, `deno`
4. **Go Toolchain**: `go`
5. **Systems & Utilities**: `git`, `docker`, `docker-compose`, `curl`, `make`

---

### Component Design

We will implement this by introducing a new module `src/check.rs` and registering it in `src/main.rs` and `src/main_cli.rs`.

#### 1. `src/main_cli.rs` [MODIFY]
Add `Check` as a subcommand to the main CLI options:
```rust
pub enum Commands {
    // ... other commands
    /// Verify active development tools (uv, pip, bun, cargo, pnpm, npm, etc.)
    Check,
}
```

#### 2. `src/check.rs` [NEW]
Define the checker structure, categorization, and execution functions:

- **Data Models**:
```rust
use std::path::PathBuf;

pub enum ToolStatus {
    Ok,
    Warning(String),
    Missing,
}

pub struct ToolCheck {
    pub binary: &'static str,
    pub name: &'static str,
    pub version_args: &'static [&'static str],
}

pub struct ToolResult {
    pub name: &'static str,
    pub binary: &'static str,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub status: ToolStatus,
}

pub struct ToolGroup {
    pub group_name: &'static str,
    pub tools: Vec<ToolCheck>,
}
```

- **Helper Functions**:
  - `which(binary)`: Walks `$PATH` to locate the binary file.
  - `run_version(binary, args)`: Launches `<binary> <args...>`, captures stdout/stderr, strips whitespace, and returns a clean version string.
  - `run_diagnostics()`: Runs the diagnostics for each group sequentially.

- **Smart Cross-Checks (Doctor Mode)**:
  - If a group has package managers but lacks their primary runtime, flag a warning:
    - If `npm`/`pnpm`/`yarn` is installed but `node` is missing → `Warning("Node.js missing but JS package manager installed".to_string())`.
    - If `pip`/`uv`/`pipx` is installed but `python` is missing → `Warning("Python missing but Python tools installed".to_string())`.
    - If `cargo` is installed but `rustc` is missing → `Warning("Rust compiler (rustc) missing but Cargo installed".to_string())`.

- **Visual Output Design**:
Print a beautifully styled breakdown:
```
  Rust Toolchain
    ✓  Cargo            /usr/bin/cargo (cargo 1.76.0)
    ✓  Rustc            /usr/bin/rustc (rustc 1.76.0)
    ✓  Rustup           /usr/bin/rustup (rustup 1.26.0)

  Python Toolchain
    ✓  Python           /usr/bin/python3 (Python 3.10.12)
    ✓  Pip              /usr/bin/pip (pip 22.0.2)
    ✓  UV               /usr/bin/uv (uv 0.1.20)
    ✗  Pipx             missing

  Node/JS Toolchain
    ✓  Node             /usr/bin/node (v18.19.1)
    ✓  Npm              /usr/bin/npm (10.2.4)
    ✓  Pnpm             /usr/bin/pnpm (8.15.4)
    ✓  Bun              /usr/bin/bun (1.0.30)
    ✗  Yarn             missing
    ✗  Deno             missing

  Go Toolchain
    ✓  Go               /usr/bin/go (go version go1.21.6 linux/amd64)

  Systems & Utilities
    ✓  Git              /usr/bin/git (git version 2.43.0)
    ✓  Docker           /usr/bin/docker (Docker version 24.0.7)
    ⚠  Docker Compose   /usr/bin/docker-compose (docker-compose failed: permission denied)
    ✓  Curl             /usr/bin/curl (curl 8.5.0)
    ✓  Make             /usr/bin/make (GNU Make 4.3)
```

---

## Verification Plan

### Automated Tests
- We will add a integration test in `tests/` or unit tests in `src/check.rs` verifying that `which` locates system commands (like `git` or `cargo`) and `run_version` captures the output of `git --version` or `cargo --version`.

### Manual Verification
- Compile the binary using `cargo build`.
- Run `cargo run -- check` and inspect the formatted console output.
- Verify that present tools have their path and version shown with `✓`.
- Verify that absent tools are clearly marked as `missing` with `✗`.
