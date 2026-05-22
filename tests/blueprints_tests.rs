use projm::blueprints::{Blueprint, BlueprintsStore};
use std::fs;

#[test]
fn load_returns_default_when_file_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let store = BlueprintsStore::load_from(tmp.path().join("nonexistent.json")).unwrap();
    assert!(store.blueprints.is_empty());
}

#[test]
fn load_parses_existing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("blueprints.json");
    fs::write(
        &path,
        r#"{"blueprints":[{"name":"test-cargo","command":"cargo new {name}"}]}"#,
    )
    .unwrap();

    let store = BlueprintsStore::load_from(path).unwrap();
    assert_eq!(store.blueprints.len(), 1);
    assert_eq!(store.blueprints[0].name, "test-cargo");
    assert_eq!(store.blueprints[0].command, "cargo new {name}");
}

#[test]
fn save_creates_parent_dirs_and_saves() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("nested/dir/blueprints.json");

    let mut store = BlueprintsStore::default();
    store.blueprints.push(Blueprint {
        name: "react-vite".to_string(),
        command: "npm create vite@latest {name}".to_string(),
    });

    store.save_to(&path).unwrap();
    assert!(path.exists());

    let loaded = BlueprintsStore::load_from(path).unwrap();
    assert_eq!(loaded.blueprints.len(), 1);
    assert_eq!(loaded.blueprints[0].name, "react-vite");
    assert_eq!(
        loaded.blueprints[0].command,
        "npm create vite@latest {name}"
    );
}

#[test]
fn placeholder_substitution() {
    let bp = Blueprint {
        name: "test".to_string(),
        command: "bun create better-t-stack@latest {name} --frontend none".to_string(),
    };

    let project_name = "medlink";
    let resolved = bp.command.replace("{name}", project_name);
    assert_eq!(
        resolved,
        "bun create better-t-stack@latest medlink --frontend none"
    );
}
