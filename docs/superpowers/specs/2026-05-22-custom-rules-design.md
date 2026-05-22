# Custom Classification Rules (v0.5) Design Specification

This specification outlines the design and implementation for custom project classification rules in `projm`. It enables developers to define custom criteria in a `rules.toml` configuration file to override the built-in classification heuristics for specific project layouts.

---

## 1. Goal Description

Currently, `projm` uses hardcoded heuristics to classify scanned projects into one of seven categories (`apps`, `services`, `ui`, `embedded`, `ml`, `tools`, `labs`). The only override mechanism is creating an empty `doc-lab.md` file to force a project into the `labs` category.

This design introduces a declarative configuration file `~/.config/projm/rules.toml` (loaded first during classification) that evaluates user-defined rules from top to bottom. If a project matches all active criteria in a custom rule, it is classified into the designated category immediately, bypassing built-in logic.

---

## 2. Configuration Design

### Path & Location
The configuration file will be located at:
- **Unix/Linux**: `~/.config/projm/rules.toml`
- **macOS**: `~/Library/Application Support/projm/rules.toml`
- **Windows**: `%USERPROFILE%\AppData\Roaming\projm\rules.toml`

This matches the existing base directory for preferences and configuration stored via the `dirs` crate.

### Default Template
When `projm init` is run, if `rules.toml` does not exist in the configuration directory, `projm` will create it with a complete set of commented-out rules to serve as documentation:

```toml
# ==============================================================================
# Projm Custom Classification Rules Configuration (rules.toml)
# ==============================================================================
#
# Rules are evaluated from top to bottom. The first matching rule wins.
# Within a single [[rule]], all specified criteria must match (AND logic).
#
# Supported fields:
# - name          : Exact name match of the project directory (e.g. "pioneers-website")
# - name_contains : Substring match of the project directory name (e.g. "adrar")
# - marker        : File/directory presence marker in the project root (e.g. "rocket.toml")
# - suffix        : Override built-in suffix behaviour (e.g. "fw")
# - has_dep       : Check dependencies in Cargo.toml, package.json, or requirements.txt/pyproject.toml (e.g. "burn")
#
# Categories must be one of: "apps", "services", "ui", "embedded", "ml", "tools", "labs"
#
# Examples:
#
# [[rule]]
# name     = "pioneers-website"
# category = "ui"
#
# [[rule]]
# marker   = "rocket.toml"
# category = "services"
#
# [[rule]]
# name_contains = "adrar"
# category      = "labs"
#
# [[rule]]
# suffix   = "fw"
# category = "embedded"
#
# [[rule]]
# has_dep  = "burn"
# category = "ml"
#
# [[rule]]
# has_dep  = "tensorflow"
# category = "ml"
```

---

## 3. Data Structures

We will model the deserialized and validated structures as follows:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct CustomRule {
    pub name: Option<String>,
    pub marker: Option<String>,
    pub name_contains: Option<String>,
    pub suffix: Option<String>,
    pub has_dep: Option<String>,
    pub category: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct RulesConfig {
    #[serde(rename = "rule", default)]
    pub rules: Vec<CustomRule>,
}
```

To validate categories, we map valid strings directly to the `Category` enum:

```rust
#[derive(Debug, Clone)]
pub struct ValidatedRule {
    pub name: Option<String>,
    pub marker: Option<String>,
    pub name_contains: Option<String>,
    pub suffix: Option<String>,
    pub has_dep: Option<String>,
    pub category: Category,
}
```

---

## 4. Execution Logic & Semantics

### Loading & Fallbacks
1. `projm` reads `rules.toml` from the config directory.
2. If the file is missing, custom rule evaluation is silently skipped.
3. If the file is syntactically invalid, a clear warning is printed to `stderr` and custom rules are skipped:
   `  warning: Failed to parse rules.toml: <error details>`
4. If a rule specifies an invalid/unsupported category, a warning is printed to `stderr` and that specific rule is skipped:
   `  warning: Ignoring custom rule #<index> in rules.toml: unknown category '<value>'`

### Matching Algorithm (AND logic)
For a given project path `path`, a rule matches if and only if **all** of its non-empty fields match the project details.

```rust
impl ValidatedRule {
    pub fn matches(&self, path: &Path) -> bool {
        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => return false,
        };

        // 1. Exact name match
        if let Some(ref rule_name) = self.name {
            if name != *rule_name {
                return false;
            }
        }

        // 2. Name contains (substring match)
        if let Some(ref rule_contains) = self.name_contains {
            if !name.contains(rule_contains) {
                return false;
            }
        }

        // 3. Suffix match (case-insensitive, separated by dash or underscore)
        if let Some(ref rule_suffix) = self.suffix {
            if let Some((_, suf)) = split_suffix(&name) {
                if suf.to_lowercase() != rule_suffix.to_lowercase() {
                    return false;
                }
            } else {
                return false;
            }
        }

        // 4. Marker match (file/directory presence)
        if let Some(ref rule_marker) = self.marker {
            if !path.join(rule_marker).exists() {
                return false;
            }
        }

        // 5. Dependency match
        if let Some(ref rule_dep) = self.has_dep {
            if !self.check_dep(path, rule_dep) {
                return false;
            }
        }

        true
    }
}
```

### Dependency Scanning (`has_dep`)
The dependency detection scans the target project for manifest files and matches dependencies inside them:
- **Rust (`Cargo.toml`)**: Read and parse the manifest using `toml`. Search for the key in `dependencies`, `dev-dependencies`, and `build-dependencies`.
- **Node.js (`package.json`)**: Scan the dependencies and devDependencies using `extract_dep_keys` regex/line scanner.
- **Python (`requirements.txt`)**: Check package name before version specifiers (e.g. matching lines starting with `package==` or `package>=`).
- **Python (`pyproject.toml`)**: Parse as `toml` and look under PEP 621 `project.dependencies` and Poetry `tool.poetry.dependencies`.

---

## 5. Integration Plan

1. **Add Dependency**: Add `toml = "0.8"` to `Cargo.toml`.
2. **Create rules.rs**: Implement loading, parsing, verification, and matching logic. Expose `load_rules` and `init_default_rules`.
3. **Update classify.rs**:
   - Expose `extract_dep_keys_helper` or clean up dependency extraction for use in `rules.rs`.
   - Update `classify` signature to accept a slice of rules: `pub fn classify(path: &Path, rules: &[ValidatedRule]) -> Category`.
   - Check custom rules at the top of `classify`.
4. **Update organize.rs**:
   - Load rules once at command startup using `rules::load_rules()`.
   - Pass loaded rules into `classify`.
5. **Update init_setup.rs**:
   - Call `rules::init_default_rules()` to generate a default rules template on setup.
6. **Update Tests**:
   - Fix all existing compilation errors in test files by passing `&[]` to `classify` calls.
   - Implement extensive unit testing for custom rules, checking exact name, substring, marker, suffix, and dependency matching.
