# Design Document: Git Integration for `pg`

This specification describes the addition of Git branch and status markers in the `pg` (`projm g`) project selection prompt.

## Goal

Provide terminal users with instant visual cues about their projects' Git branch and clean/dirty states right inside the fuzzy-picker interface (`dialoguer`), as shown below:

```
  apps      drivetrack-api        main  ✓
  apps      drivetrack-web        feat/auth  *
```

Additionally, this change includes fixes for existing compiler visibility errors (`E0449`) and warnings in the codebase to guarantee a solid, warning-free baseline build.

---

## Technical Details

### 1. Build and Stability Fixes

*   **File**: `src/main_cli.rs`
    *   **Problem**: Variant fields under a public enum in Rust automatically inherit visibility. Specifying `pub` explicitly causes an `E0449` compilation error in modern compilers.
    *   **Solution**: Remove `pub` from the fields of `Commands::Organize` and `Commands::Completions`.
*   **File**: `tests/prefs_tests.rs`
    *   **Problem**: Dead helper function `setup()` causes unused code warnings.
    *   **Solution**: Clean up or prefix with `_` to suppress the warning.

### 2. Git Status and Branch Extraction

*   **Method**: Shelling out to standard `git` binary using `std::process::Command`. This keeps memory and compiled footprint extremely light and avoids native linking issues with `libgit2`.
*   **Logic**:
    *   If the project directory does not contain a `.git` sub-directory, skip Git checking entirely (gracefully return `None`).
    *   **Retrieve branch**: Run `git branch --show-current`. If it returns successfully, parse and trim stdout.
    *   **Retrieve status**: Run `git status --porcelain`. If stdout is non-empty, mark the repository as *dirty* (`is_dirty = true`). Otherwise, mark it as *clean*.

```rust
struct GitInfo {
    branch: String,
    is_dirty: bool,
}

fn get_git_info(path: &Path) -> Option<GitInfo> {
    if !path.join(".git").exists() {
        return None;
    }

    let branch_output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(path)
        .output()
        .ok()?;

    if !branch_output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&branch_output.stdout).trim().to_string();
    if branch.is_empty() {
        return None;
    }

    let status_output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()
        .ok()?;

    let is_dirty = status_output.status.success() && !status_output.stdout.is_empty();

    Some(GitInfo { branch, is_dirty })
}
```

### 3. Clean and Aligned Columns in the Fuzzy-Picker

To ensure columns align beautifully regardless of varying project name lengths, we dynamically determine the maximum project name length at runtime and apply aligned formatting:

```rust
let max_name_len = projects
    .iter()
    .map(|p| p.name.len())
    .max()
    .unwrap_or(20)
    .max(20);

let labels: Vec<String> = projects
    .iter()
    .map(|p| {
        let cat_lbl = p.category.label();
        
        let git_part = if let Some(git) = &p.git_info {
            let status_indicator = if git.is_dirty {
                "*".yellow().bold().to_string()
            } else {
                "✓".green().to_string()
            };
            format!("  {:<15}  {}", git.branch.dimmed(), status_indicator)
        } else {
            "".to_string()
        };

        format!("  {}  {:<width$}  {}", cat_lbl, p.name, git_part, width = max_name_len)
    })
    .collect();
```

---

## Verification Plan

### Automated Tests
*   Ensure full compilation: `cargo check` and `cargo build` pass with zero errors and zero warnings.
*   Ensure all existing tests pass: `cargo test`.
*   Write unit tests in a new or existing test file to verify the `get_git_info` logic under multiple repository configurations (non-git directory, clean repository, dirty repository).

### Manual Verification
*   Execute `projm g` (`pg`) inside a workspace.
*   Observe the terminal fuzzy selector interface.
*   Verify that projects display Git branches and correct status indicators.
*   Verify that non-git projects gracefully show no Git suffix information and alignment remains unbroken.
