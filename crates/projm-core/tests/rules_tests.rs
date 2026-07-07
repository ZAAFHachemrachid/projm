use projm_core::classify::Category;
use projm_core::rules::{
    evaluate_rules, evaluate_rules_verbose, parse_and_validate, ValidatedRule,
    DEFAULT_RULES_TEMPLATE,
};
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

/// Build a single validated rule from TOML fields (without the `[[rule]]` header).
fn rule(fields: &str) -> ValidatedRule {
    let rules = parse_and_validate(&format!("[[rule]]\n{}", fields));
    assert_eq!(rules.len(), 1, "expected exactly one valid rule from: {fields}");
    rules.into_iter().next().unwrap()
}

// ── Legacy matchers (v1 behavior preserved) ───────────────────────────────────

#[test]
fn test_exact_name_match() {
    let (_root, path) = named_project("pioneers-website", &[]);
    let r = rule("name = \"pioneers-website\"\ncategory = \"ui\"");
    assert!(r.matches(&path));
    assert_eq!(r.category, Category::Ui);

    let (_root2, path2) = named_project("other-website", &[]);
    assert!(!r.matches(&path2));
}

#[test]
fn test_name_contains_match() {
    let (_root, path) = named_project("adrar-labs-project", &[]);
    let r = rule("name_contains = \"adrar\"\ncategory = \"labs\"");
    assert!(r.matches(&path));

    let (_root2, path2) = named_project("other-labs", &[]);
    assert!(!r.matches(&path2));
}

#[test]
fn test_marker_match() {
    let dir = project(&["rocket.toml"]);
    let r = rule("marker = \"rocket.toml\"\ncategory = \"services\"");
    assert!(r.matches(dir.path()));

    let dir2 = project(&[]);
    assert!(!r.matches(dir2.path()));
}

#[test]
fn test_suffix_match() {
    let (_root, path) = named_project("rocket-telemetry-fw", &[]);
    let r = rule("suffix = \"fw\"\ncategory = \"embedded\"");
    assert!(r.matches(&path));

    let (_root2, path2) = named_project("rocket-telemetry-api", &[]);
    assert!(!r.matches(&path2));
}

#[test]
fn test_and_logic() {
    let r = rule("suffix = \"api\"\nhas_dep = \"hono\"\ncategory = \"services\"");

    let (_root, path) = named_project("my-service-api", &[]);
    fs::write(
        path.join("package.json"),
        r#"{"dependencies":{"hono":"^4.0.0"}}"#,
    )
    .unwrap();
    assert!(r.matches(&path));

    // Missing suffix
    let (_root2, path2) = named_project("my-service", &[]);
    fs::write(
        path2.join("package.json"),
        r#"{"dependencies":{"hono":"^4.0.0"}}"#,
    )
    .unwrap();
    assert!(!r.matches(&path2));

    // Missing dependency
    let (_root3, path3) = named_project("my-service-api", &[]);
    fs::write(
        path3.join("package.json"),
        r#"{"dependencies":{"express":"^4.0.0"}}"#,
    )
    .unwrap();
    assert!(!r.matches(&path3));
}

#[test]
fn test_has_dep_cargo() {
    let dir = project(&[]);
    fs::write(
        dir.path().join("Cargo.toml"),
        "[dependencies]\nburn = \"0.12.0\"\n",
    )
    .unwrap();
    let r = rule("has_dep = \"burn\"\ncategory = \"ml\"");
    assert!(r.matches(dir.path()));

    let dir2 = project(&[]);
    fs::write(
        dir2.path().join("Cargo.toml"),
        "[dependencies]\ntokio = \"1.0\"\n",
    )
    .unwrap();
    assert!(!r.matches(dir2.path()));
}

#[test]
fn test_has_dep_package_json() {
    let dir = project(&[]);
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"tensorflow":"^2.0"}}"#,
    )
    .unwrap();
    let r = rule("has_dep = \"tensorflow\"\ncategory = \"ml\"");
    assert!(r.matches(dir.path()));

    let dir2 = project(&[]);
    fs::write(
        dir2.path().join("package.json"),
        r#"{"dependencies":{"react":"^18.0"}}"#,
    )
    .unwrap();
    assert!(!r.matches(dir2.path()));
}

#[test]
fn test_has_dep_requirements_txt() {
    let dir = project(&[]);
    fs::write(
        dir.path().join("requirements.txt"),
        "tensorflow>=2.0\nscikit-learn\n# comment",
    )
    .unwrap();

    assert!(rule("has_dep = \"tensorflow\"\ncategory = \"ml\"").matches(dir.path()));
    assert!(rule("has_dep = \"scikit-learn\"\ncategory = \"ml\"").matches(dir.path()));
    assert!(!rule("has_dep = \"pytorch\"\ncategory = \"ml\"").matches(dir.path()));
}

#[test]
fn test_has_dep_pyproject_toml() {
    let dir = project(&[]);
    fs::write(
        dir.path().join("pyproject.toml"),
        "[project]\ndependencies = [\n  \"tensorflow>=2.0\",\n  \"pandas\"\n]\n",
    )
    .unwrap();
    let r = rule("has_dep = \"tensorflow\"\ncategory = \"ml\"");
    assert!(r.matches(dir.path()));

    let dir2 = project(&[]);
    fs::write(
        dir2.path().join("pyproject.toml"),
        "[tool.poetry.dependencies]\ntensorflow = \"^2.0\"\n",
    )
    .unwrap();
    assert!(r.matches(dir2.path()));
}

#[test]
fn test_has_dep_remotion_to_content() {
    let dir = project(&[]);
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"remotion":"^4.0.0"}}"#,
    )
    .unwrap();
    let r = rule("has_dep = \"remotion\"\ncategory = \"content\"");
    assert!(r.matches(dir.path()));
    assert_eq!(r.category, Category::Content);
}

// ── Backward compatibility ────────────────────────────────────────────────────

#[test]
fn test_backcompat_old_template_parses() {
    let old_template = r#"
[[rule]]
name     = "pioneers-website"
category = "ui"

[[rule]]
marker   = "rocket.toml"
category = "services"

[[rule]]
name_contains = "adrar"
category      = "labs"

[[rule]]
suffix   = "fw"
category = "embedded"

[[rule]]
has_dep  = "burn"
category = "ml"
"#;
    let rules = parse_and_validate(old_template);
    assert_eq!(rules.len(), 5);
    assert_eq!(rules[0].category, Category::Ui);
    assert_eq!(rules[4].category, Category::Ml);
    // File order preserved (no priorities set)
    assert_eq!(
        rules.iter().map(|r| r.index).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5]
    );
}

#[test]
fn test_default_template_parses_empty() {
    // The shipped template is all comments — it must parse to zero rules.
    assert!(parse_and_validate(DEFAULT_RULES_TEMPLATE).is_empty());
}

#[test]
fn test_flatten_schema_roundtrip() {
    let toml = r#"
[[rule]]
description = "full schema"
enabled     = true
priority    = 5
name_glob   = "*-api"
name_regex  = "^svc"
parent_dir  = "clients"
path_glob   = "**/experiments/**"
markers     = ["Dockerfile"]
any_marker  = ["justfile", "Makefile"]
has_deps    = ["react", "vite"]
dep_version = { name = "react", req = ">=18" }
stack       = "js"
none_of     = [ { name_contains = "legacy" } ]
any_of      = [ { name_glob = "*-api" }, { marker = "rocket.toml" } ]
category    = "services"
"#;
    let rules = parse_and_validate(toml);
    assert_eq!(rules.len(), 1);
    let r = &rules[0];
    assert_eq!(r.description.as_deref(), Some("full schema"));
    assert_eq!(r.priority, Some(5));
    assert!(r.matcher.name_glob.is_some());
    assert!(r.matcher.name_regex.is_some());
    assert_eq!(r.matcher.parent_dir.as_deref(), Some("clients"));
    assert!(r.matcher.path_glob.is_some());
    assert_eq!(r.matcher.markers, vec!["Dockerfile"]);
    assert_eq!(r.matcher.any_marker, vec!["justfile", "Makefile"]);
    assert_eq!(r.matcher.has_deps, vec!["react", "vite"]);
    assert!(r.matcher.dep_version.is_some());
    assert_eq!(r.matcher.stack, Some("js"));
    assert_eq!(r.any_of.len(), 2);
    assert_eq!(r.none_of.len(), 1);
    assert_eq!(r.category, Category::Services);
}

// ── New name/path matchers ────────────────────────────────────────────────────

#[test]
fn test_name_glob_match() {
    let r = rule("name_glob = \"*-api\"\ncategory = \"services\"");
    let (_root, path) = named_project("billing-api", &[]);
    assert!(r.matches(&path));
    let (_root2, path2) = named_project("billing-web", &[]);
    assert!(!r.matches(&path2));
}

#[test]
fn test_invalid_glob_skipped_with_warning() {
    let rules = parse_and_validate("[[rule]]\nname_glob = \"[\"\ncategory = \"ui\"");
    assert!(rules.is_empty());
}

#[test]
fn test_name_regex_match() {
    let r = rule("name_regex = \"^svc-[0-9]+$\"\ncategory = \"services\"");
    let (_root, path) = named_project("svc-42", &[]);
    assert!(r.matches(&path));
    let (_root2, path2) = named_project("svc-x", &[]);
    assert!(!r.matches(&path2));
}

#[test]
fn test_invalid_regex_skipped() {
    let rules = parse_and_validate("[[rule]]\nname_regex = \"(\"\ncategory = \"ui\"");
    assert!(rules.is_empty());
}

#[test]
fn test_parent_dir_match() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("clients").join("acme-site");
    fs::create_dir_all(&dir).unwrap();
    let r = rule("parent_dir = \"clients\"\ncategory = \"ui\"");
    assert!(r.matches(&dir));

    let other = root.path().join("internal").join("acme-site");
    fs::create_dir_all(&other).unwrap();
    assert!(!r.matches(&other));
}

#[test]
fn test_path_glob_match() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("experiments").join("thing");
    fs::create_dir_all(&dir).unwrap();
    let r = rule("path_glob = \"**/experiments/**\"\ncategory = \"labs\"");
    assert!(r.matches(&dir));

    let other = root.path().join("prod").join("thing");
    fs::create_dir_all(&other).unwrap();
    assert!(!r.matches(&other));
}

// ── Marker sets ───────────────────────────────────────────────────────────────

#[test]
fn test_markers_all_required() {
    let r = rule("markers = [\"Dockerfile\", \"fly.toml\"]\ncategory = \"services\"");
    let both = project(&["Dockerfile", "fly.toml"]);
    assert!(r.matches(both.path()));
    let one = project(&["Dockerfile"]);
    assert!(!r.matches(one.path()));
}

#[test]
fn test_any_marker_one_suffices() {
    let r = rule("any_marker = [\"justfile\", \"Makefile\"]\ncategory = \"tools\"");
    let just = project(&["justfile"]);
    assert!(r.matches(just.path()));
    let mk = project(&["Makefile"]);
    assert!(r.matches(mk.path()));
    let none = project(&["README.md"]);
    assert!(!r.matches(none.path()));
}

// ── Dependency sets & versions ────────────────────────────────────────────────

#[test]
fn test_has_deps_all_required() {
    let r = rule("has_deps = [\"react\", \"vite\"]\ncategory = \"ui\"");
    let dir = project(&[]);
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"react":"^18.0"},"devDependencies":{"vite":"^5.0"}}"#,
    )
    .unwrap();
    assert!(r.matches(dir.path()));

    let dir2 = project(&[]);
    fs::write(
        dir2.path().join("package.json"),
        r#"{"dependencies":{"react":"^18.0"}}"#,
    )
    .unwrap();
    assert!(!r.matches(dir2.path()));
}

#[test]
fn test_dep_version_package_json_caret() {
    let r = rule("dep_version = { name = \"react\", req = \">=18\" }\ncategory = \"ui\"");
    let dir = project(&[]);
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"react":"^18.2.0"}}"#,
    )
    .unwrap();
    assert!(r.matches(dir.path()));

    let dir2 = project(&[]);
    fs::write(
        dir2.path().join("package.json"),
        r#"{"dependencies":{"react":"^17.0.2"}}"#,
    )
    .unwrap();
    assert!(!r.matches(dir2.path()));
}

#[test]
fn test_dep_version_cargo_min_version() {
    let r = rule("dep_version = { name = \"tokio\", req = \">=1\" }\ncategory = \"services\"");
    let dir = project(&[]);
    fs::write(
        dir.path().join("Cargo.toml"),
        "[dependencies]\ntokio = { version = \"1.35\", features = [\"full\"] }\n",
    )
    .unwrap();
    assert!(r.matches(dir.path()));
}

#[test]
fn test_dep_version_requirements_pin() {
    let r = rule("dep_version = { name = \"django\", req = \">=4\" }\ncategory = \"services\"");
    let dir = project(&[]);
    fs::write(dir.path().join("requirements.txt"), "django==4.2.1\n").unwrap();
    assert!(r.matches(dir.path()));

    let dir2 = project(&[]);
    fs::write(dir2.path().join("requirements.txt"), "django==3.2\n").unwrap();
    assert!(!r.matches(dir2.path()));
}

#[test]
fn test_dep_version_unparseable_is_no_match() {
    let r = rule("dep_version = { name = \"shared\", req = \">=1\" }\ncategory = \"ui\"");
    let dir = project(&[]);
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"shared":"workspace:*"}}"#,
    )
    .unwrap();
    assert!(!r.matches(dir.path()));
}

// ── Stack matcher ─────────────────────────────────────────────────────────────

#[test]
fn test_stack_matcher_rust() {
    let r = rule("stack = \"rust\"\ncategory = \"tools\"");
    let dir = project(&["Cargo.toml"]);
    assert!(r.matches(dir.path()));
    let js = project(&["package.json"]);
    assert!(!r.matches(js.path()));
}

#[test]
fn test_stack_matcher_js() {
    let r = rule("stack = \"js\"\ncategory = \"ui\"");
    let dir = project(&["package.json"]);
    assert!(r.matches(dir.path()));
}

#[test]
fn test_unknown_stack_id_skipped() {
    let rules = parse_and_validate("[[rule]]\nstack = \"cobol\"\ncategory = \"labs\"");
    assert!(rules.is_empty());
}

// ── OR / negation ─────────────────────────────────────────────────────────────

#[test]
fn test_any_of_or_logic() {
    let r = rule(
        "any_of = [ { name_glob = \"*-api\" }, { marker = \"rocket.toml\" } ]\ncategory = \"services\"",
    );
    let (_root, by_name) = named_project("billing-api", &[]);
    assert!(r.matches(&by_name));

    let by_marker = project(&["rocket.toml"]);
    assert!(r.matches(by_marker.path()));

    let (_root2, neither) = named_project("frontend-app", &[]);
    assert!(!r.matches(&neither));
}

#[test]
fn test_any_of_combined_with_top_level_and() {
    let r = rule(
        "name_contains = \"billing\"\nany_of = [ { name_glob = \"*-api\" }, { marker = \"rocket.toml\" } ]\ncategory = \"services\"",
    );
    let (_root, ok) = named_project("billing-api", &[]);
    assert!(r.matches(&ok));

    // any_of matches but top-level AND fails
    let (_root2, wrong_name) = named_project("payments-api", &[]);
    assert!(!r.matches(&wrong_name));
}

#[test]
fn test_none_of_negation_name() {
    let r = rule(
        "name_glob = \"*-api\"\nnone_of = [ { name_contains = \"legacy\" } ]\ncategory = \"services\"",
    );
    let (_root, ok) = named_project("billing-api", &[]);
    assert!(r.matches(&ok));

    let (_root2, legacy) = named_project("legacy-billing-api", &[]);
    assert!(!r.matches(&legacy));
}

#[test]
fn test_none_of_negation_marker() {
    let r = rule("none_of = [ { marker = \"doc-lab.md\" } ]\ncategory = \"tools\"");
    let plain = project(&["Cargo.toml"]);
    assert!(r.matches(plain.path()));

    let lab = project(&["doc-lab.md"]);
    assert!(!r.matches(lab.path()));
}

#[test]
fn test_empty_matcher_in_any_of_rejected() {
    let rules = parse_and_validate("[[rule]]\nany_of = [ {} ]\ncategory = \"ui\"");
    assert!(rules.is_empty());
    let rules = parse_and_validate("[[rule]]\nnone_of = [ {} ]\ncategory = \"ui\"");
    assert!(rules.is_empty());
}

// ── Rule management: enabled / priority / identity ────────────────────────────

#[test]
fn test_enabled_false_skipped() {
    let toml = r#"
[[rule]]
enabled  = false
name     = "x"
category = "ui"

[[rule]]
name     = "y"
category = "ml"
"#;
    let rules = parse_and_validate(toml);
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].index, 2);
    assert_eq!(rules[0].category, Category::Ml);
}

#[test]
fn test_priority_reorders_before_file_order() {
    let toml = r#"
[[rule]]
name_contains = "proj"
category      = "ui"

[[rule]]
priority      = 1
name_contains = "proj"
category      = "ml"
"#;
    let rules = parse_and_validate(toml);
    assert_eq!(rules.len(), 2);
    // The prioritized rule (file position 2) evaluates first.
    assert_eq!(rules[0].index, 2);

    let (_root, path) = named_project("proj-thing", &[]);
    let matched = evaluate_rules(&path, &rules).unwrap();
    assert_eq!(matched.category, Category::Ml);
}

#[test]
fn test_evaluate_rules_returns_rule_identity() {
    let toml = r#"
[[rule]]
name     = "no-match-here"
category = "ui"

[[rule]]
description   = "adrar catch"
name_contains = "adrar"
category      = "labs"
"#;
    let rules = parse_and_validate(toml);
    let (_root, path) = named_project("adrar-experiment", &[]);
    let matched = evaluate_rules(&path, &rules).unwrap();
    assert_eq!(matched.index, 2);
    assert_eq!(matched.description.as_deref(), Some("adrar catch"));
}

#[test]
fn test_evaluate_rules_verbose_flags() {
    let toml = r#"
[[rule]]
name     = "no-match-here"
category = "ui"

[[rule]]
name_contains = "adrar"
category      = "labs"
"#;
    let rules = parse_and_validate(toml);
    let (_root, path) = named_project("adrar-experiment", &[]);
    let evals = evaluate_rules_verbose(&path, &rules);
    assert_eq!(evals.len(), 2);
    assert!(!evals[0].matched);
    assert!(evals[1].matched);
    assert_eq!(evals[1].index, 2);
    assert_eq!(evals[1].category, "labs");
}
