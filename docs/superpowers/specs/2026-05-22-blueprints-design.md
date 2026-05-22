# Design Document: Interactive Project Blueprints for `projm`

This specification describes the addition of interactive project templates (blueprints) in `projm`, enabling developers to save complex creation commands with placeholders and execute them interactively.

## Goal

Provide developers with a standard way to save, list, and run complex multi-step template project generators (e.g. `bun create better-t-stack@latest`, `django-admin startproject`, `cargo new`) from a local interactive wizard. Once the project is created, the system prompts the user to automatically run `projm organize` to file the new project into the correct base directory category (e.g. `~/projects/apps/`).

---

## Technical Details

### 1. CLI Commands & Subcommands

We add a new `Blueprint` subcommand to `Commands` in `src/main_cli.rs`:

```rust
#[derive(Subcommand)]
pub enum Commands {
    // Existing commands...

    /// Manage and run project creation blueprints
    Blueprint {
        #[command(subcommand)]
        sub: Option<BlueprintSubcommands>,
    },
}

#[derive(Subcommand)]
pub enum BlueprintSubcommands {
    /// Add a new blueprint interactively
    Add,
    /// List all saved blueprints
    List,
    /// Run a blueprint to create a new project
    Run {
        /// Optional name of the blueprint to run
        name: Option<String>,
    },
}
```

---

### 2. Storage & Serialization

Blueprints are stored as a JSON list in `~/.config/projm/blueprints.json`. We will implement a `BlueprintsStore` module in `src/blueprints.rs` to manage loading, saving, and querying blueprints.

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BlueprintsStore {
    pub blueprints: Vec<Blueprint>,
}

impl BlueprintsStore {
    pub fn load() -> anyhow::Result<Self> {
        let path = default_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let store = serde_json::from_str(&content).unwrap_or_default();
        Ok(store)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = default_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

fn default_path() -> anyhow::Result<PathBuf> {
    Ok(dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve XDG config dir"))?
        .join("projm/blueprints.json"))
}
```

---

### 3. Wizard & Subcommand Logic

#### `projm blueprint add`
Launches an interactive wizard to prompt the user:
- **Blueprint Name**: Prompts using `dialoguer::Input`. Must be unique and alphanumeric/dashes.
- **Creation Command**: Prompts using `dialoguer::Input`. Must contain the `{name}` placeholder.
  - *Validation*: If `{name}` is not found, the wizard warns the user and prompts again.

#### `projm blueprint list`
Prints all blueprints to stdout in a clean, readable layout.
```
  Blueprint             Command Template
  ──────────────────────────────────────────────────────────────────────────
  better-t-stack        bun create better-t-stack@latest {name} --frontend...
  cargo-lib             cargo new {name} --lib
```

#### `projm blueprint run [name]`
1. **Selection**: If `name` is not provided, display a `dialoguer::Select` prompt showing all saved blueprints. If no blueprints are saved, tell the user how to add one and exit.
2. **Project Name**: Prompt for the project name using `dialoguer::Input`. Ensure it is a valid directory name.
3. **Execution**:
   - Replace `{name}` with the chosen project name.
   - Print the final command to be executed.
   - Run the command under the Current Working Directory (CWD) through the active user's system shell (`sh -c "<command>"` or `bash -c "<command>"`).
   - Inherit standard I/O streams (`stdin`, `stdout`, `stderr`) to maintain fully interactive execution.
4. **Post-Creation Auto-Organization**:
   - Check if a directory matching the new project name exists in CWD.
   - If it exists, ask: `"Automatically organize '<project-name>' under ~/projects/ category?" [Y/n]`.
   - If yes, invoke `organize::run` pointing to the new project directory path to classify and file it.

---

## Verification Plan

### Automated Tests
- Build and compile verification: `cargo check` and `cargo build` pass with zero errors and warning-free.
- Unit tests: Create unit tests validating:
  - Blueprint parsing and placeholder replacement logic.
  - Serialization / deserialization of the JSON store.
  - Validation rules for template commands (checking `{name}`).

### Manual Verification
1. Add a new blueprint:
   `projm blueprint add`
   - Name: `test-cargo`
   - Command: `cargo new {name}`
2. Run the blueprint:
   `projm blueprint run`
   - Select `test-cargo` from the list.
   - Input project name: `my-awesome-test-lib`.
   - Verify that cargo runs and creates the directory.
   - Agree to the auto-organize prompt.
   - Verify the directory has been categorized and moved to `~/projects/tools/my-awesome-test-lib` (or appropriate category).
