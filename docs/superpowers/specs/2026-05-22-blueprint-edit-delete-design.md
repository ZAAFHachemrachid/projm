# Blueprint Edit and Delete (v0.3.3) Design Specification

This specification outlines the design and implementation for modifying (`edit`/`update`) and removing (`delete`/`rm`) project blueprints in `projm`. It enables developers to manage their interactive project creation templates directly from the CLI.

---

## 1. Goal Description

Currently, `projm` allows creating new project blueprints via `projm blueprint add` and listing them via `projm blueprint list`. However, there are no subcommands to edit or delete existing blueprints. If a user makes a typo in a command template or wants to remove a blueprint, they have to manually locate and edit `~/.config/projm/blueprints.json`.

This feature adds two new subcommands:
- `projm blueprint edit [name]` (with alias `update`)
- `projm blueprint delete [name]` (with aliases `rm`, `remove`)

These subcommands will support both direct argument targeting and an interactive selection prompt fallback.

---

## 2. CLI Design

We will extend `BlueprintSubcommands` in `src/main_cli.rs`:

```rust
#[derive(Subcommand, Debug, Clone)]
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
    /// Edit an existing blueprint
    #[command(alias = "update")]
    Edit {
        /// Optional name of the blueprint to edit
        name: Option<String>,
    },
    /// Delete an existing blueprint
    #[command(alias = "rm", alias = "remove")]
    Delete {
        /// Optional name of the blueprint to delete
        name: Option<String>,
    },
}
```

---

## 3. Execution Logic

We will implement the execution flows in `src/blueprints.rs`.

### Edit Flow (`edit`)
1. **Target Resolution**:
   - If `name` is provided: Search for the blueprint matching this name. If not found, print a clear error and exit.
   - If `name` is `None`: Present a dialoguer `Select` picker listing all saved blueprints. If the store is empty, exit with an informative message.
2. **Form Pre-Filling**:
   - Prompt the user to update the **Blueprint Name** using `dialoguer::Input` with `.with_initial_value(existing_name)`.
   - Prompt the user to update the **Command Template** using `dialoguer::Input` with `.with_initial_value(existing_template)`.
3. **Validation**:
   - The new name must be alphanumeric with dashes or underscores, not empty.
   - The new name must not conflict with another existing blueprint (excluding itself).
   - The command template must not be empty and must contain the `{name}` placeholder.
4. **Execution**:
   - Mutate the blueprint in the store, save it to `blueprints.json`.
   - Output a clean green success checkmark to the terminal.

### Delete Flow (`delete`)
1. **Target Resolution**:
   - If `name` is provided: Search for the blueprint matching this name. If not found, print a clear error and exit.
   - If `name` is `None`: Present a dialoguer `Select` picker listing all saved blueprints. If the store is empty, exit with an informative message.
2. **Confirmation**:
   - Prompt for confirmation using `dialoguer::Confirm` with: `"Are you sure you want to delete blueprint '{name}'?"` (defaulting to `false`).
3. **Execution**:
   - Remove the blueprint from the `store.blueprints` vector.
   - Save the store to `blueprints.json`.
   - Output a clean green success checkmark to the terminal.

---

## 4. Verification Plan

### Automated/Unit Tests
- Verification will be conducted manually because these subcommands rely entirely on interactive terminal prompts.

### Manual Verification
1. Run `cargo build`.
2. Add a new blueprint: `cargo run -- blueprint add`.
3. Delete the blueprint interactively: `cargo run -- blueprint delete` (choose the blueprint, confirm deletion, verify it's gone from `cargo run -- blueprint list`).
4. Delete the blueprint directly: `cargo run -- blueprint delete my-blueprint`.
5. Edit the blueprint interactively: `cargo run -- blueprint edit` (update name, update command, verify changes using `list`).
6. Edit the blueprint directly: `cargo run -- blueprint edit my-blueprint`.
