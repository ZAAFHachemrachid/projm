use projm_core::rules::{CustomRule, RawMatcher, DEFAULT_RULES_TEMPLATE};
use projm_core::rules_edit::{
    append_rule_at, export_rules_at, import_rules_at, list_rules_at, remove_rule_at, ImportMode,
    RuleSelector,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn rules_file(content: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rules.toml");
    fs::write(&path, content).unwrap();
    (dir, path)
}

fn simple_rule(name: &str, category: &str) -> CustomRule {
    CustomRule {
        matcher: RawMatcher {
            name: Some(name.to_string()),
            ..Default::default()
        },
        category: category.to_string(),
        ..Default::default()
    }
}

const USER_FILE: &str = r#"# My precious hand-written header
# with two comment lines

# This rule catches adrar experiments
[[rule]]
name_contains = "adrar"
category      = "labs"

[[rule]]
suffix   = "fw"
category = "embedded"
"#;

#[test]
fn test_append_on_default_template_preserves_header() {
    let (_dir, path) = rules_file(DEFAULT_RULES_TEMPLATE);
    let pos = append_rule_at(&path, &simple_rule("my-proj", "ui"), None).unwrap();
    assert_eq!(pos, 1);

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("Projm Custom Classification Rules Configuration"));
    assert!(content.contains("name = \"my-proj\""));

    let rules = list_rules_at(&path).unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].category, "ui");
    assert!(rules[0].enabled);
}

#[test]
fn test_append_then_remove_preserves_user_comments() {
    let (_dir, path) = rules_file(USER_FILE);
    let pos = append_rule_at(&path, &simple_rule("extra", "tools"), None).unwrap();
    assert_eq!(pos, 3);
    assert_eq!(list_rules_at(&path).unwrap().len(), 3);

    let removed = remove_rule_at(&path, &RuleSelector::Index(3)).unwrap();
    assert_eq!(removed.matcher.name.as_deref(), Some("extra"));

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("My precious hand-written header"));
    assert!(content.contains("This rule catches adrar experiments"));
    assert_eq!(list_rules_at(&path).unwrap().len(), 2);
}

#[test]
fn test_append_at_position_one() {
    let (_dir, path) = rules_file(USER_FILE);
    let pos = append_rule_at(&path, &simple_rule("first", "apps"), Some(1)).unwrap();
    assert_eq!(pos, 1);

    let rules = list_rules_at(&path).unwrap();
    assert_eq!(rules.len(), 3);
    assert_eq!(rules[0].matcher.name.as_deref(), Some("first"));
    assert_eq!(rules[1].matcher.name_contains.as_deref(), Some("adrar"));

    // Comments on the pre-existing rules survive the array rebuild.
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("This rule catches adrar experiments"));
}

#[test]
fn test_append_rejects_matcherless_rule() {
    let (_dir, path) = rules_file(USER_FILE);
    let bad = CustomRule {
        category: "ui".to_string(),
        ..Default::default()
    };
    let err = append_rule_at(&path, &bad, None).unwrap_err();
    assert!(err.contains("matcher"));
    assert_eq!(list_rules_at(&path).unwrap().len(), 2);
}

#[test]
fn test_append_rejects_invalid_glob() {
    let (_dir, path) = rules_file(USER_FILE);
    let bad = CustomRule {
        matcher: RawMatcher {
            name_glob: Some("[".to_string()),
            ..Default::default()
        },
        category: "ui".to_string(),
        ..Default::default()
    };
    assert!(append_rule_at(&path, &bad, None).is_err());
}

#[test]
fn test_remove_first_rule_keeps_header() {
    let (_dir, path) = rules_file(USER_FILE);
    remove_rule_at(&path, &RuleSelector::Index(1)).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("My precious hand-written header"));
    let rules = list_rules_at(&path).unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].matcher.suffix.as_deref(), Some("fw"));
}

#[test]
fn test_remove_only_rule_keeps_header() {
    let (_dir, path) = rules_file(
        "# lone header comment\n[[rule]]\nname = \"x\"\ncategory = \"ui\"\n",
    );
    remove_rule_at(&path, &RuleSelector::Index(1)).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("# lone header comment"));
    assert!(list_rules_at(&path).unwrap().is_empty());
}

#[test]
fn test_insert_at_top_keeps_header_on_top() {
    let (_dir, path) = rules_file(USER_FILE);
    append_rule_at(&path, &simple_rule("first", "apps"), Some(1)).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    let header_pos = content.find("My precious hand-written header").unwrap();
    let first_rule_pos = content.find("[[rule]]").unwrap();
    assert!(header_pos < first_rule_pos, "header must stay above the first rule");

    let rules = list_rules_at(&path).unwrap();
    assert_eq!(rules[0].matcher.name.as_deref(), Some("first"));
}

#[test]
fn test_set_all_reorder_keeps_header() {
    use projm_core::rules_edit::set_all_rules_at;

    let (_dir, path) = rules_file(USER_FILE);
    let mut rules = list_rules_at(&path).unwrap();
    rules.swap(0, 1); // reorder → neither index keeps its old table

    set_all_rules_at(&path, &rules).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("My precious hand-written header"));
    let after = list_rules_at(&path).unwrap();
    assert_eq!(after[0].matcher.suffix.as_deref(), Some("fw"));
}

#[test]
fn test_remove_by_name_and_errors() {
    let (_dir, path) = rules_file(
        "[[rule]]\nname = \"alpha\"\ncategory = \"ui\"\n\n[[rule]]\nname = \"beta\"\ncategory = \"ml\"\n\n[[rule]]\nname = \"alpha\"\ncategory = \"labs\"\n",
    );

    // Ambiguous name
    let err = remove_rule_at(&path, &RuleSelector::Name("alpha".to_string())).unwrap_err();
    assert!(err.contains("ambiguous"));
    assert!(err.contains("#1"));
    assert!(err.contains("#3"));

    // Missing name
    let err = remove_rule_at(&path, &RuleSelector::Name("gamma".to_string())).unwrap_err();
    assert!(err.contains("no rule with name"));

    // Unique name works
    let removed = remove_rule_at(&path, &RuleSelector::Name("beta".to_string())).unwrap();
    assert_eq!(removed.category, "ml");
    assert_eq!(list_rules_at(&path).unwrap().len(), 2);

    // Out-of-range index
    let err = remove_rule_at(&path, &RuleSelector::Index(9)).unwrap_err();
    assert!(err.contains("does not exist"));
}

#[test]
fn test_export_roundtrip() {
    let (_dir, path) = rules_file(USER_FILE);
    let exported = export_rules_at(&path, None).unwrap();
    assert_eq!(exported, USER_FILE);

    let dest_dir = tempfile::tempdir().unwrap();
    let dest = dest_dir.path().join("pack.toml");
    export_rules_at(&path, Some(&dest)).unwrap();
    assert_eq!(fs::read_to_string(&dest).unwrap(), USER_FILE);
}

#[test]
fn test_import_merge_dedups_and_reports() {
    let (_dir, path) = rules_file(USER_FILE);
    // Source has one duplicate (adrar) and one new rule.
    let (_sdir, src) = rules_file(
        "[[rule]]\nname_contains = \"adrar\"\ncategory = \"labs\"\n\n[[rule]]\nhas_dep = \"burn\"\ncategory = \"ml\"\n",
    );

    let report = import_rules_at(&path, &src, ImportMode::Merge).unwrap();
    assert_eq!(report.added, 1);
    assert_eq!(report.skipped_duplicates, 1);
    assert!(!report.replaced);

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("My precious hand-written header"));
    assert!(content.contains("# imported from"));
    assert_eq!(list_rules_at(&path).unwrap().len(), 3);

    // Re-import is a no-op.
    let report2 = import_rules_at(&path, &src, ImportMode::Merge).unwrap();
    assert_eq!(report2.added, 0);
    assert_eq!(report2.skipped_duplicates, 2);
}

#[test]
fn test_import_replace_preserves_source_comments() {
    let (_dir, path) = rules_file(USER_FILE);
    let (_sdir, src) = rules_file("# shared team pack\n[[rule]]\nhas_dep = \"burn\"\ncategory = \"ml\"\n");

    let report = import_rules_at(&path, &src, ImportMode::Replace).unwrap();
    assert!(report.replaced);
    assert_eq!(report.added, 1);

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("# shared team pack"));
    assert!(!content.contains("My precious hand-written header"));
}

#[test]
fn test_set_all_rules_preserves_header_and_unchanged_comments() {
    use projm_core::rules_edit::set_all_rules_at;

    let (_dir, path) = rules_file(USER_FILE);
    let mut rules = list_rules_at(&path).unwrap();
    // Change only the second rule's category; first stays identical.
    rules[1].category = "tools".to_string();
    rules.push(simple_rule("new-one", "apps"));

    set_all_rules_at(&path, &rules).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("My precious hand-written header"));
    // Unchanged rule #1 keeps its own comment.
    assert!(content.contains("This rule catches adrar experiments"));

    let after = list_rules_at(&path).unwrap();
    assert_eq!(after.len(), 3);
    assert_eq!(after[1].category, "tools");
    assert_eq!(after[2].matcher.name.as_deref(), Some("new-one"));
}

#[test]
fn test_import_invalid_source_leaves_file_untouched() {
    let (_dir, path) = rules_file(USER_FILE);
    let (_sdir, src) = rules_file("[[rule]\nbroken =");

    assert!(import_rules_at(&path, &src, ImportMode::Merge).is_err());
    assert!(import_rules_at(&path, &src, ImportMode::Replace).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), USER_FILE);
}
