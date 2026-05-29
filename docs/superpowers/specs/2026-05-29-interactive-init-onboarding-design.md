# Design Spec: Interactive Onboarding Wizard & Showcase (`projm init`)

An engaging, premium, step-by-step interactive setup wizard and sandbox showcase built directly into the `projm init` command, running by default in interactive TTY environments while maintaining full backward compatibility in non-interactive shell settings.

---

## 1. Objectives

- **Enhance Developer Onboarding**: Make setting up `projm` interactive, colorful, and zero-friction.
- **Showcase Core Value Instantly**: Allow users to experience project organization and fuzzy-navigation safely within a temporary mock sandbox.
- **Maintain Compatibility**: Preserve existing non-interactive/scripted `projm init` behaviors for automated installer scripts.

---

## 2. CLI Interface Changes

We will modify the CLI command parameters in `src/main_cli.rs`:

- **Command**: `projm init`
- **New Option**: `--non-interactive` (boolean flag to completely skip the wizard, useful for automated scripts).

### CLI Arguments definition:
```rust
Init {
    /// Shell function/alias name
    #[arg(short = 'a', long, default_value = "pg")]
    alias: String,

    /// Run in non-interactive mode, bypassing the onboarding wizard
    #[arg(long)]
    non_interactive: bool,
}
```

---

## 3. Onboarding Wizard Architecture

### Interactive Check
When `projm init` is invoked:
1. Ensure the default configuration and rules are initialized.
2. Check if interaction is possible and desired:
   `let is_interactive = !non_interactive && console::user_attended();`
3. If `is_interactive`, trigger the wizard: `run_wizard(alias, target)`.
4. If not, fallback to existing automated behavior: `run_non_interactive(alias, target)`.

---

## 4. Setup Wizard Flow (Step-by-Step)

### Step 1: Welcome Screen
A beautiful, colored text logo and description:
```text
   ┌────────────────────────────────────────────────────────┐
   │  🚀 Welcome to projm                                   │
   │  The developer-first project organizer & navigator.    │
   └────────────────────────────────────────────────────────┘
```

### Step 2: Base Directory Prompt
Uses `dialoguer::Input` to read the target base projects directory (defaulting to `~/projects` or the current custom base path):
- If the user changes it, update it automatically using `config::set_base`.

### Step 3: Editor Picker
Scans installed editor binaries via `editors::detect_installed()`:
- Displays a `dialoguer::Select` menu of the detected editors.
- Saves the selected editor configuration.
- If none are found, prompts them to input a manual editor command or skip.

### Step 4: Shell Integration Alias
Prompts the user to specify their custom alias (defaulting to `pg`).

### Step 5: zoxide and Integration Checks
- Verifies if `zoxide` is on `$PATH`.
- If missing, displays installer options and prompts them for permission to automatically install.
- Performs shell completions and `.zshrc` / `.bashrc` profile integrations.

---

## 5. Guided Sandbox Showcase

Immediately following successful configuration, the wizard asks:
`? Would you like to run a quick 1-minute sandbox demo of projm? (Y/n)`

If approved:
1. **Scaffolding**:
   - Create a temporary directory using `tempfile::tempdir()`.
   - Setup `source-dump/` and `demo-base/` folders inside it.
   - Write files targeting three stacks:
     - `rust-service/Cargo.toml`
     - `node-app/package.json` (React & Vite markers)
     - `python-ml/pyproject.toml` (PyTorch markers)
2. **Organization Demonstration**:
   - Print a breakdown of what stack markers were found.
   - Run `organize::run` programmatically from `source-dump/` to `demo-base/`.
   - Print the structured layout of the organized base directory with folder emojis and colored paths.
3. **Fuzzy Search Navigation**:
   - Launch `dialoguer::FuzzySelect` representing the newly organized sandbox directories.
   - Let the user experience typing to search and pressing `Enter` to select.
   - Print a congratulatory final screen explaining how the chosen directory would be opened in their chosen editor in a real terminal shell.
4. **Cleanup**:
   - The temporary sandbox directories are dropped automatically, leaving the user's filesystem perfectly clean.

---

## 6. Verification & Test Plan

### Automated Tests
- Unit test in `src/init_setup.rs` to verify that `run` behaves correctly in non-interactive mode.
- Unit test to ensure `installer_plan` matches all expected target platforms.

### Manual Verification
- Execute `cargo run -- init` in an interactive shell to walk through each configuration step and sandbox demo.
- Execute `cargo run -- init --non-interactive` to verify it behaves exactly as the legacy non-interactive command, creating files and updating profiles without requiring inputs.
