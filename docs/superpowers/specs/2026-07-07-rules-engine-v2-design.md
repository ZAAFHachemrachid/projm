# Rules Engine v2 — Richer Matching + New Ways to Set Rules

Date: 2026-07-07
Status: Shipped
Builds on: `2026-05-22-custom-rules-design.md`, `2026-05-23-content-category-design.md`

## Summary

Extends the declarative rules engine with glob/regex/path/stack/version matchers,
OR (`any_of`) and NOT (`none_of`) groups, and rule management fields
(`description`, `enabled`, `priority`). Adds four new surfaces for setting rules:
a `projm rules` CLI family, a per-project `.projm.toml` category pin, interactive
assignment in the desktop app, and rule-pack import/export. Fully backward
compatible — v1 rules files parse unchanged.

## Classification precedence (new)

```
1. .projm.toml pin   marker.rs — category pinned inside the project (travels with repo)
2. rules.toml        rules.rs — custom rules, first match wins (priority-sorted)
3. doc-lab.md        legacy labs marker (kept)
4. built-in logic    monorepo → suffix → stack → name heuristics → labs default
```

`classify()` keeps its signature; `classify_explained()` additionally reports the
`ClassificationSource` (marker / rule index + description / heuristic label) that
powers `projm rules test` and the desktop "Test a path" feature.

## Schema

All new fields are optional and AND-combined with the v1 fields:

| Field | Meaning |
|---|---|
| `name_glob`, `name_regex` | Pattern matching on the directory name (globset / regex, compiled at load) |
| `parent_dir`, `path_glob` | Path-based matching |
| `markers` / `any_marker` | ALL / at-least-one file-presence sets |
| `has_deps` | ALL dependencies present |
| `dep_version = { name, req }` | Semver check against the declared minimum version (best-effort; `workspace:*`/git → no match) |
| `stack` | Detected stack id (shares `detect_stack` with `projm run`) |
| `any_of = [{...}]` | OR groups — at least one matcher table fully matches |
| `none_of = [{...}]` | Negation — rule fails if any table matches |
| `description`, `enabled`, `priority` | Rule management (priority: lower first, stable vs file order) |

Invalid rules (bad glob/regex/semver/stack, empty `any_of`/`none_of` tables) warn
and are skipped — same graceful degradation as v1.

## Key modules

- `crates/projm-core/src/rules.rs` — `RawMatcher`/`CustomRule` (serde, flattened),
  `CompiledMatcher`/`ValidatedRule` (patterns compiled once), `MatchContext`
  (lazy per-path stack + dependency caches), `evaluate_rules(_verbose)`,
  `validate_rule`, `read_all_deps`, `extract_base_version`.
- `crates/projm-core/src/marker.rs` — `.projm.toml` pin: `category`/`group`/`hidden`,
  whitelisted keys only, path-separator values rejected (repo-supplied file).
  `group` overrides the prefix-derived group folder in `organize`.
- `crates/projm-core/src/rules_edit.rs` — comment-preserving mutations via
  `toml_edit`: `list/append/remove/set_all/export/import` (+ `_at` test variants).
  Import merges with exact-equality dedup; file-only in v1 (no URL import).
- `crates/projm-cli/src/rules_cmd.rs` — `projm rules
  list|add|remove|test|edit|export|import|pin|assign`.
- `src-tauri/src/lib.rs` — `cmd_rules_{list,add,set_all,remove,test,export,import}`,
  `cmd_explain_classification`, `cmd_assign_category`.
- `app/components/settings-panel.tsx` — visual designer now backed by the granular
  commands (fixes silent field/comment loss), new field inputs, complex rules
  (any_of/none_of/lists) shown read-only with a raw-TOML escape hatch, test-a-path
  row, export/import via native dialogs, visual/raw mode toggle.
- `app/app/page.tsx` — "Set category…" dialog (marker pin vs global rule,
  optional folder move) triggered from project rows.

## Tests

- `rules_tests.rs` — 38 tests: v1 behavior preserved, all new matchers, OR/NOT,
  enabled/priority, rule identity, backcompat template parse.
- `marker_tests.rs` — 12 tests: parsing, sanitizing, precedence, explain sources.
- `rules_edit_tests.rs` — 11 tests: comment preservation, positioned insert,
  selectors, import/export semantics.
- `crates/projm-cli/tests/rules_cli_tests.rs` — 7 end-to-end CLI tests
  (hermetic via `XDG_CONFIG_HOME`).
