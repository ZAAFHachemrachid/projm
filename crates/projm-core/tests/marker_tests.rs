use projm_core::classify::{classify_explained, Category, ClassificationSource};
use projm_core::marker::{read_marker, write_marker, ProjectMarker, MARKER_FILE};
use projm_core::rules::parse_and_validate;
use std::fs;
use tempfile::TempDir;

fn project_with_marker(marker_toml: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(MARKER_FILE), marker_toml).unwrap();
    dir
}

#[test]
fn test_read_marker_full() {
    let dir = project_with_marker("category = \"ml\"\ngroup = \"drivetrack\"\nhidden = true\n");
    let m = read_marker(dir.path()).unwrap();
    assert_eq!(m.category.as_deref(), Some("ml"));
    assert_eq!(m.group.as_deref(), Some("drivetrack"));
    assert_eq!(m.hidden, Some(true));
}

#[test]
fn test_read_marker_ignores_unknown_keys_and_run_section() {
    let dir = project_with_marker(
        "category = \"tools\"\nfuture_key = \"whatever\"\n\n[run]\ncommand = \"cargo run\"\n",
    );
    let m = read_marker(dir.path()).unwrap();
    assert_eq!(m.category.as_deref(), Some("tools"));
}

#[test]
fn test_read_marker_none_on_invalid_toml() {
    let dir = project_with_marker("category = [not toml");
    assert!(read_marker(dir.path()).is_none());
}

#[test]
fn test_read_marker_none_when_absent_or_empty() {
    let dir = tempfile::tempdir().unwrap();
    assert!(read_marker(dir.path()).is_none());

    // A pure run-config .projm.toml carries no marker data.
    let run_only = project_with_marker("[run]\ncommand = \"bun dev\"\n");
    assert!(read_marker(run_only.path()).is_none());
}

#[test]
fn test_unsafe_values_rejected() {
    let dir = project_with_marker("category = \"../../etc\"\ngroup = \"a/b\"\n");
    // Both fields are unsafe → nothing usable remains.
    assert!(read_marker(dir.path()).is_none());

    let dir2 = project_with_marker("category = \"ml\"\ngroup = \"..\"\n");
    let m = read_marker(dir2.path()).unwrap();
    assert_eq!(m.category.as_deref(), Some("ml"));
    assert!(m.group.is_none());
}

#[test]
fn test_write_marker_roundtrip_preserves_run_section() {
    let dir = project_with_marker("# my run config\n[run]\ncommand = \"cargo run\"\n");
    write_marker(
        dir.path(),
        &ProjectMarker {
            category: Some("ml".to_string()),
            group: None,
            hidden: None,
        },
    )
    .unwrap();

    let content = fs::read_to_string(dir.path().join(MARKER_FILE)).unwrap();
    assert!(content.contains("# my run config"));
    assert!(content.contains("[run]"));
    assert!(content.contains("command = \"cargo run\""));

    let m = read_marker(dir.path()).unwrap();
    assert_eq!(m.category.as_deref(), Some("ml"));
}

#[test]
fn test_write_marker_rejects_unsafe() {
    let dir = tempfile::tempdir().unwrap();
    let res = write_marker(
        dir.path(),
        &ProjectMarker {
            category: Some("../evil".to_string()),
            group: None,
            hidden: None,
        },
    );
    assert!(res.is_err());
}

// ── Classification precedence ─────────────────────────────────────────────────

#[test]
fn test_marker_beats_matching_rule() {
    let dir = project_with_marker("category = \"ml\"\n");
    // A catch-all rule that would otherwise classify this as ui.
    let rules = parse_and_validate("[[rule]]\ncategory = \"ui\"");
    let c = classify_explained(dir.path(), &rules);
    assert_eq!(c.category, Category::Ml);
    assert!(matches!(c.source, ClassificationSource::ProjectMarker));
}

#[test]
fn test_marker_beats_doc_lab() {
    let dir = project_with_marker("category = \"tools\"\n");
    fs::write(dir.path().join("doc-lab.md"), "").unwrap();
    let c = classify_explained(dir.path(), &[]);
    assert_eq!(c.category, Category::Tools);
}

#[test]
fn test_doc_lab_still_wins_without_marker_or_rule() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("doc-lab.md"), "").unwrap();
    let c = classify_explained(dir.path(), &[]);
    assert_eq!(c.category, Category::Labs);
    assert!(matches!(
        c.source,
        ClassificationSource::Heuristic("doc-lab.md marker")
    ));
}

#[test]
fn test_classify_explained_reports_rule_index() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("adrar-thing");
    fs::create_dir_all(&dir).unwrap();
    let rules = parse_and_validate(
        "[[rule]]\nname = \"nope\"\ncategory = \"ui\"\n\n[[rule]]\nname_contains = \"adrar\"\ncategory = \"labs\"",
    );
    let c = classify_explained(&dir, &rules);
    assert_eq!(c.category, Category::Labs);
    match c.source {
        ClassificationSource::Rule { index, .. } => assert_eq!(index, 2),
        other => panic!("expected rule source, got {:?}", other),
    }
}

#[test]
fn test_classify_explained_suffix_heuristic() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("foo-fw");
    fs::create_dir_all(&dir).unwrap();
    let c = classify_explained(&dir, &[]);
    assert_eq!(c.category, Category::Embedded);
    assert!(matches!(
        c.source,
        ClassificationSource::Heuristic("suffix: fw")
    ));
    assert!(c.reason().contains("suffix: fw"));
}
