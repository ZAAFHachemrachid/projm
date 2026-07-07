# Rules Engine v2 — More Matching Power + New Ways to Set Rules

## Context

projm's rules engine (`crates/projm-core/src/rules.rs`) currently supports 5 AND-combined matchers (`name`, `name_contains`, `suffix`, `marker`, `has_dep`) in `~/.config/projm/rules.toml`, first match wins, evaluated before all built-in heuristics (`classify.rs:180`). Rules can only be set by hand-editing the TOML or via the desktop app settings panel (which destroys comments on save from visual mode).

This plan enhances the engine (globs/regex, OR/negation, path/stack/version matchers, enable/priority/description) and adds new setting surfaces (CLI `projm rules` subcommands, per-project `.projm.toml` pin file, interactive assignment in the app, import/export). All changes are backward compatible — existing rules.toml files keep working unchanged.

## Phase 1 — Engine: new matchers & schema (projm-core)

**Crates**: add `globset = "0.4"`, `regex = "1"`, `semver = "1"`, `toml_edit = "0.25"` to `crates/projm-core/Cargo.toml` (regex/semver/toml_edit already in Cargo.lock transitively).

**New TOML schema** (all new fields optional; old fields keep exact names/semantics):

```toml
[[rule]]
description = "API microservices except legacy"
enabled     = true                  # default true
priority    = 10                    # optional; lower runs first; unprioritized keep file order
# name matchers (AND with everything else)
name = "..." | name_contains = "..." | name_glob = "*-api" | name_regex = "^svc-\\d+$" | suffix = "fw"
# path matchers
parent_dir = "clients"
path_glob  = "**/experiments/**"
# markers
marker = "rocket.toml" | markers = ["Dockerfile", "fly.toml"] | any_marker = ["justfile", "Makefile"]
# dependencies
has_dep = "burn" | has_deps = ["react", "vite"] | dep_version = { name = "react", req = ">=18" }
# stack (uses existing detect_stack)
stack = "rust"   # rust|js|tauri|flutter|go|python|rails|elixir|gradle|maven|laravel|cpp|dotnet
# negation: rule fails if ANY of these matcher tables fully matches
none_of = [ { name_contains = "legacy" } ]
# OR groups: at least ONE table must fully match (each table is AND internally)
any_of  = [ { name_glob = "*-api" }, { marker = "rocket.toml", has_dep = "rocket" } ]
category = "services"
```

Semantics: `top-level AND (any_of has ≥1 match) AND (none_of has 0 matches)`. Empty tables inside `any_of`/`none_of` are warned + rule skipped.

**Types** (`rules.rs`):
- `RawMatcher` (serde struct with all 14 matcher fields) + `CustomRule { #[serde(flatten)] matcher, any_of, none_of, description, enabled (default true), priority, category }`. Write a flatten round-trip parse test FIRST — if `toml` misbehaves with flatten, fall back to duplicating fields on `CustomRule` with a `top_matcher()` helper.
- `CompiledMatcher` — globs/regex/semver compiled once at load; `marker`+`markers` merged, `has_dep`+`has_deps` merged.
- `ValidatedRule { index, description, matcher: CompiledMatcher, any_of, none_of, category }` (index = original file position, for explain). Add `Default` on `CompiledMatcher` + a test builder to keep tests terse.
- `MatchContext` — per-path `OnceCell` caches for stack detection and parsed deps, shared across rules.
- Refactor `check_dep` → `read_all_deps(path) -> Vec<(String, Option<String>)>` (name + raw version per manifest: Cargo.toml, package.json, requirements.txt, pyproject.toml). `dep_version` uses `extract_base_version(raw) -> Option<semver::Version>` (strips `^ ~ >=` etc.; `workspace:*`/git URLs → no match — documented best-effort).
- `parse_and_validate(contents) -> Vec<ValidatedRule>` extracted from `load_rules` (skips `enabled=false`, warns+skips invalid glob/regex/semver/stack/category, stable-sorts by `(priority.unwrap_or(i64::MAX), file_index)`).
- `evaluate_rules(path, rules) -> Option<&ValidatedRule>` and `evaluate_rules_verbose(...) -> Vec<RuleEvaluation>` (per-rule matched flags, for test/dry-run UX).

**Stack exposure** (`crates/projm-core/src/run.rs`): make `ProjectStack` + `detect_stack` `pub`; add `ProjectStack::id() -> &'static str` and a `KNOWN_STACK_IDS` const for validation. No classify reordering — rules call `detect_stack` lazily via `MatchContext`.

## Phase 2 — Explain support (classify.rs)

```rust
pub enum ClassificationSource {
    ProjectMarker,                          // .projm.toml (Phase 3)
    Rule { index: usize, description: Option<String> },  // 1-based
    Heuristic(&'static str),                // "doc-lab.md", "monorepo", "suffix: fw", "stack: tauri", "default: labs"
}
pub struct Classification { pub category: Category, pub source: ClassificationSource }

pub fn classify_explained(path, rules) -> Classification;
pub fn classify(path, rules) -> Category { classify_explained(...).category }  // signature unchanged
```

Rename current `classify` body to `classify_explained`; swap the rules loop (classify.rs:181-186) for `evaluate_rules`; wrap each `return Category::X` with its heuristic label. Existing call sites (`organize.rs:78,281`, `src-tauri/src/lib.rs:495`) need no changes.

## Phase 3 — Per-project marker file (`.projm.toml`)

New module `crates/projm-core/src/marker.rs`:
- `MARKER_FILE = ".projm.toml"`; `ProjectMarker { category: Option<String>, group: Option<String>, hidden: Option<bool> }`.
- `read_marker(dir) -> Option<ProjectMarker>` (warn+None on parse error), `write_marker(dir, &marker)` (toml_edit round-trip if exists).
- **Precedence: highest** — checked in `classify_explained` before custom rules. New order: marker → custom rules → doc-lab.md (kept as legacy labs marker) → monorepo → suffix → stack → name heuristics → labs default.
- Security (file arrives with cloned repos): only 3 whitelisted keys read; reject `category`/`group` values containing path separators or `..`; run category through existing coercion.
- `organize.rs`: marker `group` overrides prefix-derived group in `run_with_base` and `organize_single`.

## Phase 4 — Rule file mutation helpers (comment-preserving)

New module `crates/projm-core/src/rules_edit.rs` using `toml_edit::DocumentMut` so user comments survive. Each helper gets an `_at(path, ...)` variant for hermetic tests; save path reuses `save_rules_raw` validation.

```rust
pub enum RuleSelector { Index(usize) /* 1-based */, Name(String) }
pub enum ImportMode { Merge, Replace }
pub struct ImportReport { added, skipped_duplicates, replaced }

pub fn list_rules() -> Result<Vec<CustomRule>, String>;
pub fn append_rule(rule: &CustomRule, at: Option<usize>) -> Result<usize, String>;
pub fn remove_rule(selector: &RuleSelector) -> Result<CustomRule, String>;  // errors listing candidates on 0/ambiguous name
pub fn export_rules(dest: Option<&Path>) -> Result<String, String>;         // raw copy → comments preserved
pub fn import_rules(src: &Path, mode: ImportMode) -> Result<ImportReport, String>;
```

Import Merge: skip rules `==` an existing one (`CustomRule: PartialEq`), append rest with `# imported from <file> <date>` comment. Replace: validate then copy wholesale. **File-only in v1 — no URL import** (supply-chain risk; revisit with confirm-diff flow later).

## Phase 5 — CLI `projm rules` subcommand family

`crates/projm-cli/src/main_cli.rs`: add `Rules { #[command(subcommand)] sub: RulesSubcommands }` following the existing `BlueprintSubcommands` precedent (`blueprints.rs`). New `crates/projm-cli/src/rules_cmd.rs` + dispatch arm in `main.rs`:

- `list [--json]` — priority-ordered table.
- `add --category <cat> [--name|--name-contains|--name-glob|--suffix|--marker|--has-dep|--stack ...] [--at N]` — at least one matcher required.
- `remove <index|name>` (aliases `rm`, `delete`) — echoes removed rule.
- `test <path> [--json]` — uses `classify_explained`: `myproj-api → services  matched rule #2 (name_contains = "api")` / `trainer → ml  pinned by .projm.toml` / `foo-fw → embedded  heuristic: suffix "fw"`, plus a hint line suggesting `projm rules pin` when a heuristic decided.
- `edit` — `$VISUAL`/`$EDITOR`, re-validate on save; on failure offer re-edit/keep/revert (backup first).
- `export [file]` / `import <file> [--replace]`.
- `pin [path] --category <cat> [--group G] [--hidden]` — writes `.projm.toml` (default path: cwd).
- `assign [query]` — `dialoguer::FuzzySelect` over organized projects (reuse `go.rs` listing approach) → category select → choose "Pin with .projm.toml" vs "Add global exact-name rule (inserted at #1)" → optional `Confirm` "Move folder now?" → `organize::organize_single`.

Shell completions regenerate automatically via existing `completions::emit`.

## Phase 6 — Tauri commands (`src-tauri/src/lib.rs`)

Keep `cmd_get_rules_raw`/`cmd_save_rules_raw` (raw editor tab). Add + register:
`cmd_rules_list`, `cmd_rules_add(rule, at)`, `cmd_rules_remove(index)`, `cmd_rules_test(path)` (→ `{category, sourceKind, detail}`), `cmd_rules_export(dest)`, `cmd_rules_import(src, replace)`, `cmd_explain_classification(path)`, and `cmd_assign_category(path, category, mode: "marker"|"rule", group, move_project)` (writes marker or prepends exact-name rule; optionally calls `organize_single`, returns new path).

## Phase 7 — Desktop UI

`app/components/settings-panel.tsx` (rules section ~1716-1975):
- **Blocker fix**: `parseRulesToml`/`stringifyRulesToml` (~lines 89-143) whitelist only the 5 legacy fields and silently DROP new fields on visual-mode save. Migrate visual designer mutations to granular `cmd_rules_list/add/remove` (comment-preserving); extend the TS `CustomRule` interface with new scalar fields; render rules containing `any_of`/`none_of` as read-only cards ("edit in raw mode").
- Add new field inputs (glob, regex, stack dropdown, enabled toggle, description) to the visual designer.
- "Test a path" row → `cmd_rules_test`, shows category badge + reason.
- Export/Import buttons via `@tauri-apps/plugin-dialog` (already installed) → `cmd_rules_export/import`, ImportReport toast.

`app/app/page.tsx` (project cards ~1359 grid / ~1493 list): "Set category…" dropdown action → dialog with category select, radio marker-vs-rule (marker default), "Move folder now" checkbox → `cmd_assign_category` → refresh list.

## Phase 8 — Template & docs

- Update `init_default_rules()` template with commented examples of all new fields + precedence note (marker > rules > doc-lab.md > heuristics).
- README + `docs/superpowers/specs/`: document `.projm.toml` pinning and `projm rules --help`.

## Tests

- `crates/projm-core/tests/rules_tests.rs`: migrate 10 existing tests to a builder; add: backcompat (old template parses byte-for-byte), flatten round-trip, glob match/no-match/invalid-skipped, regex, parent_dir, path_glob, markers-all/any_marker-one, has_deps-all, any_of OR + combined with top-level AND, none_of negation (name + marker), empty-matcher-rejected, dep_version per manifest + unparseable-no-match, stack match/mismatch/unknown-id, enabled=false skipped, priority reorder, evaluate_rules identity + verbose flags.
- New `rules_edit_tests.rs`: append preserves header comments; append+remove round-trip keeps user comments; remove-by-name errors on missing/ambiguous; no-matcher add errors; `--at` position; import merge dedup/replace/invalid-untouched; export re-parses identical.
- New `marker_tests.rs`: parse category/group/hidden, ignore unknown keys, None on invalid TOML; precedence (marker beats rule beats doc-lab.md); path-separator category rejected; `classify_explained` reports correct rule index and suffix heuristic.
- New `crates/projm-cli/tests/rules_cli_tests.rs` (dev-deps `assert_cmd`, `predicates`; set `XDG_CONFIG_HOME` to tempdir): add→list --json→remove→test flow; add-with-no-matcher exits non-zero.

## Verification

1. `cargo test -p projm_core` and `cargo test -p projm_cli` — all new + migrated tests pass.
2. `cargo check` / `cargo build` across the workspace (including src-tauri).
3. Manual CLI: `projm rules add --name-glob "*-api" --category services`, `projm rules list`, `projm rules test <some-project>`, `projm rules pin <dir> --category ml`, `projm rules export /tmp/r.toml && projm rules import /tmp/r.toml` (expect all-duplicates skip).
4. Confirm an existing v1 rules.toml with comments still loads and that `projm rules add` preserves its comments.
5. Desktop app: `tsc --noEmit` in app/, then verify rules tab (visual add/remove, test-a-path, export/import) and card "Set category…" flow with Interceptor.

## Sequencing

Phase 1 → 2 → 3 (core, each independently testable) → 4 → 5 (CLI) → 6 (Tauri) → 7 (UI) → 8 (docs). Phases 1-4 land as projm-core work with tests before any surface work begins.
