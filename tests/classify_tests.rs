use projm::classify::{classify, prefix_key, split_suffix, Category};
use std::fs;
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Create a temp dir and touch the given file paths inside it.
fn project(files: &[&str]) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    for f in files {
        let p = dir.path().join(f);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&p, "").unwrap();
    }
    dir
}

/// Create a temp dir whose last component is `name`, touching the given files.
fn named_project(name: &str, files: &[&str]) -> (TempDir, std::path::PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join(name);
    fs::create_dir_all(&dir).unwrap();
    for f in files {
        let p = dir.join(f);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&p, "").unwrap();
    }
    (root, dir)
}

// ── split_suffix ──────────────────────────────────────────────────────────────

#[test]
fn split_suffix_dash() {
    assert_eq!(split_suffix("drivetrack-api"), Some(("drivetrack", "api")));
}

#[test]
fn split_suffix_underscore() {
    assert_eq!(split_suffix("drivetrack_web"), Some(("drivetrack", "web")));
}

#[test]
fn split_suffix_case_insensitive() {
    // suffix matching is case-insensitive
    assert_eq!(split_suffix("DriveTrack-API"), Some(("DriveTrack", "API")));
}

#[test]
fn split_suffix_unknown_suffix() {
    assert!(split_suffix("drivetrack-xyz").is_none());
}

#[test]
fn split_suffix_no_separator() {
    assert!(split_suffix("trashnet").is_none());
}

#[test]
fn split_suffix_fw() {
    assert_eq!(
        split_suffix("rocket-telemetry-fw"),
        Some(("rocket-telemetry", "fw"))
    );
}

#[test]
fn split_suffix_uses_last_separator() {
    // "drivetrack-api" has one dash; pick the rightmost
    let (prefix, suffix) = split_suffix("my-app-api").unwrap();
    assert_eq!(prefix, "my-app");
    assert_eq!(suffix.to_lowercase(), "api");
}

// ── prefix_key ────────────────────────────────────────────────────────────────

#[test]
fn prefix_key_lowercases() {
    assert_eq!(prefix_key("DriveTrack-Api"), Some("drivetrack".into()));
    assert_eq!(prefix_key("DriveTrack-Web"), Some("drivetrack".into()));
}

#[test]
fn prefix_key_none_for_no_suffix() {
    assert!(prefix_key("trashnet").is_none());
}

// ── classify: explicit labs marker ───────────────────────────────────────────

#[test]
fn classify_doc_lab_wins_over_everything() {
    // Even a Tauri project gets overridden by doc-lab.md
    let dir = project(&["doc-lab.md", "Cargo.toml", "package.json", "src-tauri"]);
    assert_eq!(classify(dir.path()), Category::Labs);
}

// ── classify: embedded ───────────────────────────────────────────────────────

#[test]
fn classify_memory_x() {
    let dir = project(&["Cargo.toml", "memory.x"]);
    assert_eq!(classify(dir.path()), Category::Embedded);
}

#[test]
fn classify_openocd() {
    let dir = project(&["Cargo.toml", "openocd.cfg"]);
    assert_eq!(classify(dir.path()), Category::Embedded);
}

#[test]
fn classify_embedded_via_cargo_target() {
    let dir = project(&["Cargo.toml", ".cargo/config.toml"]);
    fs::write(
        dir.path().join(".cargo/config.toml"),
        "[build]\ntarget = \"thumbv7em-none-eabihf\"\n",
    )
    .unwrap();
    assert_eq!(classify(dir.path()), Category::Embedded);
}

#[test]
fn classify_fw_suffix_wins() {
    let (_root, dir) = named_project("rocket-telemetry-fw", &["Cargo.toml"]);
    assert_eq!(classify(&dir), Category::Embedded);
}

// ── classify: apps ────────────────────────────────────────────────────────────

#[test]
fn classify_tauri() {
    let dir = project(&["Cargo.toml", "package.json", "src-tauri"]);
    assert_eq!(classify(dir.path()), Category::Apps);
}

#[test]
fn classify_cargo_plus_package_json_is_apps() {
    let dir = project(&["Cargo.toml", "package.json"]);
    // Without a suffix hint, cargo + pkg = apps
    assert_eq!(classify(dir.path()), Category::Apps);
}

#[test]
fn classify_desk_suffix() {
    let (_root, dir) = named_project("drivetrack-desk", &["Cargo.toml", "package.json"]);
    assert_eq!(classify(&dir), Category::Apps);
}

// ── classify: services ────────────────────────────────────────────────────────

#[test]
fn classify_api_suffix_rust() {
    let (_root, dir) = named_project("drivetrack-api", &["Cargo.toml"]);
    assert_eq!(classify(&dir), Category::Services);
}

#[test]
fn classify_pure_rust_is_services() {
    let (_root, dir) = named_project("medlink-core", &["Cargo.toml"]);
    assert_eq!(classify(&dir), Category::Services);
}

#[test]
fn classify_hono_backend() {
    let (_root, dir) = named_project("medlink-api", &["package.json"]);
    fs::write(
        dir.join("package.json"),
        r#"{"dependencies":{"hono":"^4.0.0"}}"#,
    )
    .unwrap();
    assert_eq!(classify(&dir), Category::Services);
}

// ── classify: ui ─────────────────────────────────────────────────────────────

#[test]
fn classify_web_suffix() {
    let (_root, dir) = named_project("drivetrack-web", &["package.json"]);
    assert_eq!(classify(&dir), Category::Ui);
}

#[test]
fn classify_react_frontend() {
    let dir = project(&["package.json"]);
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"react":"^18.0.0","vite":"^5.0.0"}}"#,
    )
    .unwrap();
    assert_eq!(classify(dir.path()), Category::Ui);
}

#[test]
fn classify_mob_suffix() {
    let (_root, dir) = named_project("pioneers-mob", &["package.json"]);
    assert_eq!(classify(&dir), Category::Ui);
}

// ── classify: fullstack ───────────────────────────────────────────────────────

#[test]
fn classify_fullstack_package_json() {
    let dir = project(&["package.json"]);
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"react":"^18.0.0","hono":"^4.0.0"}}"#,
    )
    .unwrap();
    assert_eq!(classify(dir.path()), Category::Apps);
}

// ── classify: ml ─────────────────────────────────────────────────────────────

#[test]
fn classify_ml_with_train_py() {
    let dir = project(&["requirements.txt", "train.py"]);
    assert_eq!(classify(dir.path()), Category::Ml);
}

#[test]
fn classify_ml_with_uv_and_model() {
    let dir = project(&["uv.lock", "model.py"]);
    assert_eq!(classify(dir.path()), Category::Ml);
}

#[test]
fn classify_python_no_ml_markers_is_tools() {
    let dir = project(&["requirements.txt"]);
    assert_eq!(classify(dir.path()), Category::Tools);
}

// ── classify: tools ───────────────────────────────────────────────────────────

#[test]
fn classify_rust_cli_by_name() {
    let (_root, dir) = named_project("projm-cli", &["Cargo.toml"]);
    assert_eq!(classify(&dir), Category::Tools);
}

// ── classify: labs fallback ───────────────────────────────────────────────────

#[test]
fn classify_empty_dir_is_labs() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(classify(dir.path()), Category::Labs);
}

#[test]
fn classify_readme_only_is_labs() {
    let dir = project(&["README.md"]);
    assert_eq!(classify(dir.path()), Category::Labs);
}

// ── Category helpers ──────────────────────────────────────────────────────────

#[test]
fn category_dir_names_are_stable() {
    assert_eq!(Category::Apps.dir_name(), "apps");
    assert_eq!(Category::Services.dir_name(), "services");
    assert_eq!(Category::Ui.dir_name(), "ui");
    assert_eq!(Category::Embedded.dir_name(), "embedded");
    assert_eq!(Category::Ml.dir_name(), "ml");
    assert_eq!(Category::Tools.dir_name(), "tools");
    assert_eq!(Category::Labs.dir_name(), "labs");
}

#[test]
fn category_all_has_seven_variants() {
    assert_eq!(Category::all().len(), 7);
}
