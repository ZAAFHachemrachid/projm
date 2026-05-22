use projm::classify::Category;
use projm::rules::ValidatedRule;
use std::fs;
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_exact_name_match() {
    let (_root, path) = named_project("pioneers-website", &[]);
    let rule = ValidatedRule {
        name: Some("pioneers-website".to_string()),
        marker: None,
        name_contains: None,
        suffix: None,
        has_dep: None,
        category: Category::Ui,
    };
    assert!(rule.matches(&path));

    let (_root2, path2) = named_project("other-website", &[]);
    assert!(!rule.matches(&path2));
}

#[test]
fn test_name_contains_match() {
    let (_root, path) = named_project("adrar-labs-project", &[]);
    let rule = ValidatedRule {
        name: None,
        marker: None,
        name_contains: Some("adrar".to_string()),
        suffix: None,
        has_dep: None,
        category: Category::Labs,
    };
    assert!(rule.matches(&path));

    let (_root2, path2) = named_project("other-labs", &[]);
    assert!(!rule.matches(&path2));
}

#[test]
fn test_marker_match() {
    let dir = project(&["rocket.toml"]);
    let rule = ValidatedRule {
        name: None,
        marker: Some("rocket.toml".to_string()),
        name_contains: None,
        suffix: None,
        has_dep: None,
        category: Category::Services,
    };
    assert!(rule.matches(dir.path()));

    let dir2 = project(&[]);
    assert!(!rule.matches(dir2.path()));
}

#[test]
fn test_suffix_match() {
    let (_root, path) = named_project("rocket-telemetry-fw", &[]);
    let rule = ValidatedRule {
        name: None,
        marker: None,
        name_contains: None,
        suffix: Some("fw".to_string()),
        has_dep: None,
        category: Category::Embedded,
    };
    assert!(rule.matches(&path));

    let (_root2, path2) = named_project("rocket-telemetry-api", &[]);
    assert!(!rule.matches(&path2));
}

#[test]
fn test_and_logic() {
    let (_root, path) = named_project("my-service-api", &["package.json"]);
    fs::write(path.join("package.json"), r#"{"dependencies":{"hono":"^4.0.0"}}"#).unwrap();

    let rule = ValidatedRule {
        name: None,
        marker: None,
        name_contains: None,
        suffix: Some("api".to_string()),
        has_dep: Some("hono".to_string()),
        category: Category::Services,
    };
    assert!(rule.matches(&path));

    // Missing suffix
    let (_root2, path2) = named_project("my-service", &["package.json"]);
    fs::write(path2.join("package.json"), r#"{"dependencies":{"hono":"^4.0.0"}}"#).unwrap();
    assert!(!rule.matches(&path2));

    // Missing dependency
    let (_root3, path3) = named_project("my-service-api", &["package.json"]);
    fs::write(path3.join("package.json"), r#"{"dependencies":{"express":"^4.0.0"}}"#).unwrap();
    assert!(!rule.matches(&path3));
}

#[test]
fn test_has_dep_cargo() {
    let dir = project(&["Cargo.toml"]);
    fs::write(dir.path().join("Cargo.toml"), "[dependencies]\nburn = \"0.12.0\"\n").unwrap();

    let rule = ValidatedRule {
        name: None,
        marker: None,
        name_contains: None,
        suffix: None,
        has_dep: Some("burn".to_string()),
        category: Category::Ml,
    };
    assert!(rule.matches(dir.path()));

    let dir2 = project(&["Cargo.toml"]);
    fs::write(dir2.path().join("Cargo.toml"), "[dependencies]\ntokio = \"1.0\"\n").unwrap();
    assert!(!rule.matches(dir2.path()));
}

#[test]
fn test_has_dep_package_json() {
    let dir = project(&["package.json"]);
    fs::write(dir.path().join("package.json"), r#"{"dependencies":{"tensorflow":"^2.0"}}"#).unwrap();

    let rule = ValidatedRule {
        name: None,
        marker: None,
        name_contains: None,
        suffix: None,
        has_dep: Some("tensorflow".to_string()),
        category: Category::Ml,
    };
    assert!(rule.matches(dir.path()));

    let dir2 = project(&["package.json"]);
    fs::write(dir2.path().join("package.json"), r#"{"dependencies":{"react":"^18.0"}}"#).unwrap();
    assert!(!rule.matches(dir2.path()));
}

#[test]
fn test_has_dep_requirements_txt() {
    let dir = project(&["requirements.txt"]);
    fs::write(dir.path().join("requirements.txt"), "tensorflow>=2.0\nscikit-learn\n# comment").unwrap();

    let rule = ValidatedRule {
        name: None,
        marker: None,
        name_contains: None,
        suffix: None,
        has_dep: Some("tensorflow".to_string()),
        category: Category::Ml,
    };
    assert!(rule.matches(dir.path()));

    let rule2 = ValidatedRule {
        name: None,
        marker: None,
        name_contains: None,
        suffix: None,
        has_dep: Some("scikit-learn".to_string()),
        category: Category::Ml,
    };
    assert!(rule2.matches(dir.path()));

    let rule3 = ValidatedRule {
        name: None,
        marker: None,
        name_contains: None,
        suffix: None,
        has_dep: Some("pytorch".to_string()),
        category: Category::Ml,
    };
    assert!(!rule3.matches(dir.path()));
}

#[test]
fn test_has_dep_pyproject_toml() {
    let dir = project(&["pyproject.toml"]);
    fs::write(dir.path().join("pyproject.toml"), "[project]\ndependencies = [\n  \"tensorflow>=2.0\",\n  \"pandas\"\n]\n").unwrap();

    let rule = ValidatedRule {
        name: None,
        marker: None,
        name_contains: None,
        suffix: None,
        has_dep: Some("tensorflow".to_string()),
        category: Category::Ml,
    };
    assert!(rule.matches(dir.path()));

    let dir2 = project(&["pyproject.toml"]);
    fs::write(dir2.path().join("pyproject.toml"), "[tool.poetry.dependencies]\ntensorflow = \"^2.0\"\n").unwrap();
    assert!(rule.matches(dir2.path()));
}
