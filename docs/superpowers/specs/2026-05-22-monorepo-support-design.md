# Monorepo Support and Folder Exclusion Design

Design and specification for adding automatic monorepo detection and standard directory exclusions to `projm`.

## Overview

Modern web and systems projects often use monorepos (e.g. via Turborepo, pnpm workspaces, Lerna, Nx) containing multiple internal apps and libraries. Currently, `projm` may misclassify these as pure frontend `ui` projects or scan internal directories like `node_modules` as individual standalone projects if `projm organize` is inadvertently run inside or on a monorepo folder.

This specification addresses these pain points by:
1. Identifying monorepos and classifying them under `Category::Apps`.
2. Filtering out standard non-project subfolders (`node_modules`, `target`, `dist`, `build`) during the directory organization scans.

---

## Proposed Changes

### 1. Classification (`src/classify.rs`)

We introduce an early-stage monorepo check after custom rules and `doc-lab.md` overrides.

#### Helper `is_monorepo`
Checks for the presence of:
- `turbo.json`
- `pnpm-workspace.yaml`
- `lerna.json`
- `nx.json`
- A `"workspaces"` field inside `package.json`

```rust
fn is_monorepo(path: &Path) -> bool {
    if path.join("turbo.json").exists()
        || path.join("pnpm-workspace.yaml").exists()
        || path.join("lerna.json").exists()
        || path.join("nx.json").exists()
    {
        return true;
    }

    let pkg_json_path = path.join("package.json");
    if pkg_json_path.exists() {
        if let Ok(content) = std::fs::read_to_string(pkg_json_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if val.get("workspaces").is_some() {
                    return true;
                }
            }
        }
    }

    false
}
```

#### Integration inside `classify`
```rust
    // ── doc-lab.md is the explicit labs marker — highest priority ──────────
    if has("doc-lab.md") {
        return Category::Labs;
    }

    // ── Monorepos (Turborepo, pnpm workspaces, etc.) ──────────────────────────
    if is_monorepo(path) {
        return Category::Apps;
    }
```

---

### 2. Scanner Exclusions (`src/organize.rs`)

We modify the immediate subdirectory scanner filter block to exclude directories that are never distinct projects.

```rust
    // ── Collect immediate subdirectories ──────────────────────────────────────
    let mut raw: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            if !path.is_dir() {
                return false;
            }
            let name = e.file_name().to_string_lossy();
            if name.starts_with('.') {
                return false;
            }
            // Ignore common non-project subdirectories
            if name == "node_modules" || name == "target" || name == "dist" || name == "build" {
                return false;
            }
            true
        })
        .collect();
```

---

## Verification Plan

### Automated Tests
1. **`test_is_monorepo_classification`** inside `tests/organize_tests.rs`: Create mock folders with `turbo.json` and a `package.json` with a `"workspaces"` field to verify they are classified as `Category::Apps`.
2. **`test_organize_excludes_node_modules`** inside `tests/organize_tests.rs`: Create subfolders named `node_modules`, `target`, `dist`, `build` alongside a real project. Verify only the real project is processed when scanning.

### Manual Verification
1. Run `cargo test` to ensure all tests pass.
2. Confirm the directory `projm/test_base/ui/medlink` can be safely classified and organized.
