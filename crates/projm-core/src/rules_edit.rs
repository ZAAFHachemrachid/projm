//! Programmatic, comment-preserving edits to rules.toml.
//!
//! All mutations either append serialized fragments to the raw file text or
//! go through `toml_edit::DocumentMut`, so user comments and formatting in
//! untouched rules survive every operation. Each helper has an `_at` variant
//! taking an explicit file path for hermetic tests; the plain wrappers use
//! the standard `rules::rules_path()` location.

use crate::rules::{
    init_default_rules, rules_path, save_rules_raw_at, validate_rule, CustomRule, RulesConfig,
    DEFAULT_RULES_TEMPLATE,
};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleSelector {
    /// 1-based position, matching the "#N" shown by `projm rules list`.
    Index(usize),
    /// Matches a rule whose `name` field equals this value.
    Name(String),
}

impl RuleSelector {
    /// Parse a CLI selector: a number is an index, anything else a name.
    pub fn parse(s: &str) -> Self {
        match s.parse::<usize>() {
            Ok(n) => Self::Index(n),
            Err(_) => Self::Name(s.to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMode {
    /// Append source rules that don't already exist (exact-equality dedup).
    Merge,
    /// Replace the whole rules file with the source file.
    Replace,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportReport {
    pub added: usize,
    pub skipped_duplicates: usize,
    pub replaced: bool,
}

// ── Raw IO helpers ────────────────────────────────────────────────────────────

fn read_or_init_at(path: &Path) -> Result<String, String> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, DEFAULT_RULES_TEMPLATE).map_err(|e| e.to_string())?;
    }
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

fn parse_config(content: &str) -> Result<RulesConfig, String> {
    toml::from_str(content).map_err(|e| format!("Invalid TOML/Rules syntax: {}", e))
}

/// Serialize a single rule to a `[[rule]]` TOML fragment.
fn rule_to_fragment(rule: &CustomRule) -> Result<String, String> {
    let cfg = RulesConfig {
        rules: vec![rule.clone()],
    };
    toml::to_string(&cfg).map_err(|e| e.to_string())
}

// ── List ──────────────────────────────────────────────────────────────────────

pub fn list_rules() -> Result<Vec<CustomRule>, String> {
    let _ = init_default_rules();
    list_rules_at(&rules_path())
}

pub fn list_rules_at(path: &Path) -> Result<Vec<CustomRule>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(parse_config(&content)?.rules)
}

// ── Append ────────────────────────────────────────────────────────────────────

/// Add a rule. `at` is a 1-based position (1 = evaluated first); `None`
/// appends at the end. Returns the rule's 1-based position.
pub fn append_rule(rule: &CustomRule, at: Option<usize>) -> Result<usize, String> {
    append_rule_at(&rules_path(), rule, at)
}

pub fn append_rule_at(path: &Path, rule: &CustomRule, at: Option<usize>) -> Result<usize, String> {
    validate_rule(rule)?;
    let content = read_or_init_at(path)?;
    let existing = parse_config(&content)?; // also guards against clobbering a broken file
    let count = existing.rules.len();
    let fragment = rule_to_fragment(rule)?;

    // Positioned insert into an empty list is just an append; the text path
    // also keeps a comments-only template intact (toml_edit would emit the
    // new table above the document's trailing comment block).
    let at = if count == 0 { None } else { at };

    match at {
        // Append at the end: plain text concatenation preserves everything.
        None => {
            let mut out = content;
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            if !out.is_empty() && !out.ends_with("\n\n") {
                out.push('\n');
            }
            out.push_str(&fragment);
            save_rules_raw_at(path, &out)?;
            Ok(count + 1)
        }
        Some(n) => {
            let pos = n.clamp(1, count + 1);
            let mut doc: toml_edit::DocumentMut = content
                .parse()
                .map_err(|e| format!("Invalid TOML/Rules syntax: {}", e))?;
            let mut new_table = fragment_first_table(&fragment)?;
            set_table_prefix(&mut new_table, "\n".to_string());

            let arr = ensure_rule_array(&mut doc)?;
            let mut tables: Vec<toml_edit::Table> = arr.iter().cloned().collect();
            if pos == 1 {
                // Keep the file's header block at the top of the file: move it
                // from the old first table onto the inserted one.
                if let Some(old_first) = tables.first_mut() {
                    let header = table_prefix(old_first);
                    set_table_prefix(old_first, "\n".to_string());
                    set_table_prefix(&mut new_table, header);
                }
            }
            tables.insert(pos - 1, new_table);
            arr.clear();
            for t in tables {
                arr.push(t);
            }
            save_rules_raw_at(path, &doc.to_string())?;
            Ok(pos)
        }
    }
}

/// The raw text (comments/whitespace) attached above a `[[rule]]` header.
/// The file's header comment block lives in the FIRST table's prefix, so
/// structural edits touching position 0 must transfer it or it is lost.
fn table_prefix(t: &toml_edit::Table) -> String {
    t.decor()
        .prefix()
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_string()
}

fn set_table_prefix(t: &mut toml_edit::Table, prefix: String) {
    t.decor_mut().set_prefix(prefix);
}

fn fragment_first_table(fragment: &str) -> Result<toml_edit::Table, String> {
    let doc: toml_edit::DocumentMut = fragment.parse().map_err(|e| format!("{}", e))?;
    doc.get("rule")
        .and_then(|i| i.as_array_of_tables())
        .and_then(|a| a.get(0))
        .cloned()
        .ok_or_else(|| "internal error: serialized rule fragment has no [[rule]]".to_string())
}

fn ensure_rule_array(
    doc: &mut toml_edit::DocumentMut,
) -> Result<&mut toml_edit::ArrayOfTables, String> {
    if doc.get("rule").is_none() {
        doc.insert(
            "rule",
            toml_edit::Item::ArrayOfTables(toml_edit::ArrayOfTables::new()),
        );
    }
    doc.get_mut("rule")
        .and_then(|i| i.as_array_of_tables_mut())
        .ok_or_else(|| "rules.toml: `rule` exists but is not an array of tables".to_string())
}

// ── Set all (visual editors) ──────────────────────────────────────────────────

/// Replace the whole rule list while preserving the file's header comments.
/// Rules that are unchanged at the same position keep their own attached
/// comments; changed/new rules are re-serialized from the structs.
pub fn set_all_rules(rules: &[CustomRule]) -> Result<(), String> {
    set_all_rules_at(&rules_path(), rules)
}

pub fn set_all_rules_at(path: &Path, rules: &[CustomRule]) -> Result<(), String> {
    for (i, rule) in rules.iter().enumerate() {
        validate_rule(rule).map_err(|e| format!("rule #{}: {}", i + 1, e))?;
    }
    let content = read_or_init_at(path)?;
    let existing = parse_config(&content)?.rules;

    let mut doc: toml_edit::DocumentMut = content
        .parse()
        .map_err(|e| format!("Invalid TOML/Rules syntax: {}", e))?;

    let old_tables: Vec<toml_edit::Table> = doc
        .get("rule")
        .and_then(|i| i.as_array_of_tables())
        .map(|a| a.iter().cloned().collect())
        .unwrap_or_default();
    let old_header = old_tables.first().map(table_prefix).unwrap_or_default();

    let mut new_tables: Vec<toml_edit::Table> = Vec::with_capacity(rules.len());
    let mut first_kept_old = false;
    for (i, rule) in rules.iter().enumerate() {
        if existing.get(i) == Some(rule) {
            // Unchanged in place — keep the original table and its comments.
            if i == 0 {
                first_kept_old = true;
            }
            new_tables.push(old_tables[i].clone());
        } else {
            let mut t = fragment_first_table(&rule_to_fragment(rule)?)?;
            set_table_prefix(&mut t, "\n".to_string());
            new_tables.push(t);
        }
    }

    // Keep the file's header block (attached to the old first table) at the top.
    if !first_kept_old && !old_header.trim().is_empty() {
        match new_tables.first_mut() {
            Some(first) => set_table_prefix(first, old_header),
            None => {
                let trailing = doc.trailing().as_str().unwrap_or("").to_string();
                doc.set_trailing(format!("{}{}", old_header, trailing));
            }
        }
    }

    let arr = ensure_rule_array(&mut doc)?;
    arr.clear();
    for t in new_tables {
        arr.push(t);
    }
    save_rules_raw_at(path, &doc.to_string())
}

// ── Remove ────────────────────────────────────────────────────────────────────

/// Remove a rule by 1-based index or by exact `name` value. Returns the
/// removed rule so callers can echo it.
pub fn remove_rule(selector: &RuleSelector) -> Result<CustomRule, String> {
    remove_rule_at(&rules_path(), selector)
}

pub fn remove_rule_at(path: &Path, selector: &RuleSelector) -> Result<CustomRule, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|_| format!("no rules file at {}", path.display()))?;
    let rules = parse_config(&content)?.rules;

    let idx = match selector {
        RuleSelector::Index(n) => {
            if *n == 0 || *n > rules.len() {
                return Err(format!(
                    "rule #{} does not exist ({} rule{} defined)",
                    n,
                    rules.len(),
                    if rules.len() == 1 { "" } else { "s" }
                ));
            }
            *n - 1
        }
        RuleSelector::Name(name) => {
            let matches: Vec<usize> = rules
                .iter()
                .enumerate()
                .filter(|(_, r)| r.matcher.name.as_deref() == Some(name.as_str()))
                .map(|(i, _)| i)
                .collect();
            match matches.as_slice() {
                [] => {
                    return Err(format!(
                        "no rule with name = \"{}\"; use an index from `projm rules list`",
                        name
                    ))
                }
                [one] => *one,
                many => {
                    let positions: Vec<String> =
                        many.iter().map(|i| format!("#{}", i + 1)).collect();
                    return Err(format!(
                        "name \"{}\" is ambiguous (rules {}); remove by index instead",
                        name,
                        positions.join(", ")
                    ));
                }
            }
        }
    };

    let mut doc: toml_edit::DocumentMut = content
        .parse()
        .map_err(|e| format!("Invalid TOML/Rules syntax: {}", e))?;
    let arr = doc
        .get_mut("rule")
        .and_then(|i| i.as_array_of_tables_mut())
        .ok_or_else(|| "rules.toml has no [[rule]] entries".to_string())?;

    // The file's header comment block is attached to the first table —
    // removing rule #1 must hand it to the new first rule (or the document
    // trailing when no rules remain) or it silently disappears.
    let header = if idx == 0 {
        arr.get(0).map(table_prefix).unwrap_or_default()
    } else {
        String::new()
    };
    arr.remove(idx);
    if idx == 0 && !header.trim().is_empty() {
        if let Some(next) = arr.get_mut(0) {
            let existing = table_prefix(next);
            set_table_prefix(next, format!("{}{}", header, existing));
        } else {
            let trailing = doc
                .trailing()
                .as_str()
                .unwrap_or("")
                .to_string();
            doc.set_trailing(format!("{}{}", header, trailing));
        }
    }
    save_rules_raw_at(path, &doc.to_string())?;
    Ok(rules[idx].clone())
}

// ── Export / Import ───────────────────────────────────────────────────────────

/// Copy the raw rules file (comments included) to `dest`, or return the raw
/// string when `dest` is None (for stdout).
pub fn export_rules(dest: Option<&Path>) -> Result<String, String> {
    let _ = init_default_rules();
    export_rules_at(&rules_path(), dest)
}

pub fn export_rules_at(path: &Path, dest: Option<&Path>) -> Result<String, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|_| format!("no rules file at {}", path.display()))?;
    parse_config(&content)?; // never export a broken file
    if let Some(dest) = dest {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(dest, &content).map_err(|e| e.to_string())?;
    }
    Ok(content)
}

pub fn import_rules(src: &Path, mode: ImportMode) -> Result<ImportReport, String> {
    let _ = init_default_rules();
    import_rules_at(&rules_path(), src, mode)
}

pub fn import_rules_at(path: &Path, src: &Path, mode: ImportMode) -> Result<ImportReport, String> {
    let src_content = std::fs::read_to_string(src)
        .map_err(|e| format!("cannot read {}: {}", src.display(), e))?;
    let src_rules = parse_config(&src_content)?.rules;

    match mode {
        ImportMode::Replace => {
            // Source validated above; wholesale copy keeps its comments.
            save_rules_raw_at(path, &src_content)?;
            Ok(ImportReport {
                added: src_rules.len(),
                skipped_duplicates: 0,
                replaced: true,
            })
        }
        ImportMode::Merge => {
            let mut content = read_or_init_at(path)?;
            let existing = parse_config(&content)?.rules;

            let mut added = 0;
            let mut skipped = 0;
            let mut additions = String::new();
            for rule in &src_rules {
                if existing.contains(rule) {
                    skipped += 1;
                    continue;
                }
                additions.push_str(&format!("\n# imported from {}\n", src.display()));
                additions.push_str(&rule_to_fragment(rule)?);
                added += 1;
            }

            if added > 0 {
                if !content.is_empty() && !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push_str(&additions);
                save_rules_raw_at(path, &content)?;
            }
            Ok(ImportReport {
                added,
                skipped_duplicates: skipped,
                replaced: false,
            })
        }
    }
}
