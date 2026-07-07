//! CLI integration tests for `projm rules`.
//!
//! Each test points XDG_CONFIG_HOME (honored by `dirs::config_dir()` on
//! Linux) at a fresh tempdir so the real user config is never touched.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn projm(config_home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("projm").unwrap();
    cmd.env("XDG_CONFIG_HOME", config_home.path());
    cmd.env_remove("HOME"); // belt-and-braces: never fall back to the real config
    cmd.env("HOME", config_home.path());
    cmd
}

#[test]
fn add_list_remove_flow() {
    let cfg = tempfile::tempdir().unwrap();

    projm(&cfg)
        .args(["rules", "add", "--name", "my-proj", "--category", "ui"])
        .assert()
        .success()
        .stdout(predicate::str::contains("added rule #1"));

    projm(&cfg)
        .args(["rules", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"my-proj\""))
        .stdout(predicate::str::contains("\"category\": \"ui\""));

    projm(&cfg)
        .args(["rules", "remove", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed rule"));

    projm(&cfg)
        .args(["rules", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No rules defined"));
}

#[test]
fn add_without_matcher_fails() {
    let cfg = tempfile::tempdir().unwrap();
    projm(&cfg)
        .args(["rules", "add", "--category", "ui"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("matcher"));
}

#[test]
fn add_with_invalid_glob_fails() {
    let cfg = tempfile::tempdir().unwrap();
    projm(&cfg)
        .args(["rules", "add", "--name-glob", "[", "--category", "ui"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid name_glob"));
}

#[test]
fn test_explains_rule_match() {
    let cfg = tempfile::tempdir().unwrap();
    projm(&cfg)
        .args(["rules", "add", "--name-glob", "*-api", "--category", "services"])
        .assert()
        .success();

    let proj_root = tempfile::tempdir().unwrap();
    let proj = proj_root.path().join("billing-api");
    fs::create_dir_all(&proj).unwrap();

    projm(&cfg)
        .args(["rules", "test", proj.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("services"))
        .stdout(predicate::str::contains("matched rule #1"));
}

#[test]
fn test_explains_heuristic_with_hint() {
    let cfg = tempfile::tempdir().unwrap();
    let proj_root = tempfile::tempdir().unwrap();
    let proj = proj_root.path().join("foo-fw");
    fs::create_dir_all(&proj).unwrap();

    projm(&cfg)
        .args(["rules", "test", proj.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("embedded"))
        .stdout(predicate::str::contains("suffix: fw"))
        .stdout(predicate::str::contains("pin it"));
}

#[test]
fn test_explains_marker_pin() {
    let cfg = tempfile::tempdir().unwrap();
    let proj_root = tempfile::tempdir().unwrap();
    let proj = proj_root.path().join("trainer");
    fs::create_dir_all(&proj).unwrap();

    projm(&cfg)
        .args(["rules", "pin", proj.to_str().unwrap(), "--category", "ml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pinned"));

    assert!(proj.join(".projm.toml").exists());

    projm(&cfg)
        .args(["rules", "test", proj.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("ml"))
        .stdout(predicate::str::contains("pinned by .projm.toml"));
}

#[test]
fn export_import_roundtrip_skips_duplicates() {
    let cfg = tempfile::tempdir().unwrap();
    projm(&cfg)
        .args(["rules", "add", "--has-dep", "burn", "--category", "ml"])
        .assert()
        .success();

    let out = tempfile::tempdir().unwrap();
    let pack = out.path().join("pack.toml");
    projm(&cfg)
        .args(["rules", "export", pack.to_str().unwrap()])
        .assert()
        .success();
    assert!(pack.exists());

    projm(&cfg)
        .args(["rules", "import", pack.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("imported 0 rules"))
        .stdout(predicate::str::contains("skipped 1 duplicate"));
}
