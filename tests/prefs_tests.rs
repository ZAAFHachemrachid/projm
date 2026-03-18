use projm::prefs::Prefs;
use std::{fs, path::PathBuf};
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Redirect the prefs file into a temp dir so tests never touch ~/.config.
/// Returns (TempDir, path-to-use-as-project).
fn setup() -> (TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("my-project");
    fs::create_dir_all(&project).unwrap();
    (tmp, project)
}

// ── load ─────────────────────────────────────────────────────────────────────

#[test]
fn load_returns_default_when_file_missing() {
    let prefs = Prefs::load_from(PathBuf::from("/tmp/projm-nonexistent-xyz/prefs.json"));
    assert!(prefs.last_editor.is_empty());
}

#[test]
fn load_parses_existing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("prefs.json");
    fs::write(
        &path,
        r#"{"last_editor":{"/home/rachid/projects/drivetrack-api":"nvim"}}"#,
    )
    .unwrap();

    let prefs = Prefs::load_from(path);
    assert_eq!(
        prefs
            .last_editor
            .get("/home/rachid/projects/drivetrack-api")
            .map(String::as_str),
        Some("nvim")
    );
}

// ── save / roundtrip ─────────────────────────────────────────────────────────

#[test]
fn save_creates_parent_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("a/b/c/prefs.json");

    let mut prefs = Prefs::default();
    prefs
        .last_editor
        .insert("/some/project".into(), "hx".into());
    prefs.save_to(&path).unwrap();

    assert!(path.exists());
}

#[test]
fn roundtrip_last_editor() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("prefs.json");

    let project = PathBuf::from("/projects/trashnet");

    let mut prefs = Prefs::default();
    prefs.set_last_editor_at(&path, &project, "nvim").unwrap();

    let loaded = Prefs::load_from(path);
    assert_eq!(loaded.last_editor_for(&project), Some("nvim"));
}

#[test]
fn set_last_editor_overwrites_previous() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("prefs.json");

    let project = PathBuf::from("/projects/medlink-api");

    let mut prefs = Prefs::default();
    prefs.set_last_editor_at(&path, &project, "code").unwrap();
    prefs.set_last_editor_at(&path, &project, "nvim").unwrap();

    let loaded = Prefs::load_from(path);
    assert_eq!(loaded.last_editor_for(&project), Some("nvim"));
}

#[test]
fn multiple_projects_are_independent() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("prefs.json");

    let proj_a = PathBuf::from("/projects/drivetrack-api");
    let proj_b = PathBuf::from("/projects/drivetrack-web");

    let mut prefs = Prefs::default();
    prefs.set_last_editor_at(&path, &proj_a, "nvim").unwrap();
    prefs.set_last_editor_at(&path, &proj_b, "zed").unwrap();

    let loaded = Prefs::load_from(path);
    assert_eq!(loaded.last_editor_for(&proj_a), Some("nvim"));
    assert_eq!(loaded.last_editor_for(&proj_b), Some("zed"));
}

#[test]
fn last_editor_for_unknown_project_is_none() {
    let prefs = Prefs::default();
    assert!(prefs
        .last_editor_for(&PathBuf::from("/projects/unknown"))
        .is_none());
}
