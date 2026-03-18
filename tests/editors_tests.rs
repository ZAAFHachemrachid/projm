use projm::editors::{detect_installed, KNOWN_EDITORS};
use std::{fs, os::unix::fs::PermissionsExt};
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Put fake executables in a temp dir and replace PATH with *only* that dir.
/// Isolates detection from whatever editors are installed on the host machine.
/// Returns the TempDir (must be kept alive) and a guard that restores PATH on drop.
fn fake_path(binaries: &[&str]) -> (TempDir, PathGuard) {
    let dir = tempfile::tempdir().unwrap();
    for bin in binaries {
        let p = dir.path().join(bin);
        fs::write(&p, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let old = std::env::var("PATH").unwrap_or_default();

    std::env::set_var("PATH", dir.path());
    (dir, PathGuard(old))
}

struct PathGuard(String);
impl Drop for PathGuard {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.0);
    }
}

// ── detect_installed ─────────────────────────────────────────────────────────

#[test]
fn detects_nothing_when_none_installed() {
    // Use a path that contains only an empty dir — none of our editors are there
    let dir = tempfile::tempdir().unwrap();
    let guard = {
        let old = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", dir.path());
        PathGuard(old)
    };

    assert!(detect_installed().is_empty());
    drop(guard);
}

#[test]
fn detects_single_installed_editor() {
    let (_dir, _guard) = fake_path(&["nvim"]);
    let installed = detect_installed();
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].binary, "nvim");
    assert_eq!(installed[0].name, "Neovim");
}

#[test]
fn detects_multiple_editors() {
    let (_dir, _guard) = fake_path(&["nvim", "zed", "hx"]);
    let installed = detect_installed();
    let binaries: Vec<&str> = installed.iter().map(|e| e.binary).collect();
    assert!(binaries.contains(&"nvim"));
    assert!(binaries.contains(&"zed"));
    assert!(binaries.contains(&"hx"));
}

#[test]
fn does_not_return_unknown_binaries() {
    let (_dir, _guard) = fake_path(&["nvim", "some-random-tool"]);
    let installed = detect_installed();
    assert!(
        installed
            .iter()
            .all(|e| KNOWN_EDITORS.iter().any(|(b, _)| *b == e.binary)),
        "returned an editor not in KNOWN_EDITORS"
    );
}

#[test]
fn resolved_path_points_to_file() {
    let (_dir, _guard) = fake_path(&["nvim"]);
    let installed = detect_installed();
    assert!(installed[0].path.is_file());
}

#[test]
fn preserves_known_editors_order() {
    // All editors installed → result should respect KNOWN_EDITORS order
    let all_binaries: Vec<&str> = KNOWN_EDITORS.iter().map(|(b, _)| *b).collect();
    let (_dir, _guard) = fake_path(&all_binaries);
    let installed = detect_installed();

    let result_binaries: Vec<&str> = installed.iter().map(|e| e.binary).collect();
    let expected: Vec<&str> = all_binaries.clone();
    assert_eq!(result_binaries, expected);
}

// ── KNOWN_EDITORS sanity ──────────────────────────────────────────────────────

#[test]
fn no_duplicate_binaries_in_known_editors() {
    let binaries: Vec<&str> = KNOWN_EDITORS.iter().map(|(b, _)| *b).collect();
    let unique: std::collections::HashSet<_> = binaries.iter().collect();
    assert_eq!(
        binaries.len(),
        unique.len(),
        "duplicate binary in KNOWN_EDITORS"
    );
}

#[test]
fn no_duplicate_names_in_known_editors() {
    let names: Vec<&str> = KNOWN_EDITORS.iter().map(|(_, n)| *n).collect();
    let unique: std::collections::HashSet<_> = names.iter().collect();
    assert_eq!(names.len(), unique.len(), "duplicate name in KNOWN_EDITORS");
}

#[test]
fn antigravity_not_in_known_editors() {
    assert!(
        !KNOWN_EDITORS.iter().any(|(b, _)| *b == "antigravity"),
        "antigravity should not be in the editor list"
    );
}

