use projm_core::{classify::Category, organize::resolve_dest};
use std::{collections::HashMap, path::PathBuf};

fn base() -> PathBuf {
    PathBuf::from("/home/rachid/projects")
}

fn counts(pairs: &[(&str, usize)]) -> HashMap<String, usize> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

// ── No grouping ───────────────────────────────────────────────────────────────

#[test]
fn solo_project_goes_directly_into_category() {
    let (dest, group): (PathBuf, Option<String>) =
        resolve_dest(&base(), "trashnet", &Category::Ml, &counts(&[]));
    assert_eq!(dest, PathBuf::from("/home/rachid/projects/ml/trashnet"));
    assert!(group.is_none());
}

#[test]
fn single_suffixed_project_not_grouped() {
    // Only one project with prefix "drivetrack" → no group folder
    let (dest, group): (PathBuf, Option<String>) = resolve_dest(
        &base(),
        "drivetrack-api",
        &Category::Services,
        &counts(&[("drivetrack", 1)]),
    );
    assert_eq!(
        dest,
        PathBuf::from("/home/rachid/projects/services/drivetrack-api")
    );
    assert!(group.is_none());
}

// ── Grouping ──────────────────────────────────────────────────────────────────

#[test]
fn two_siblings_are_grouped_under_prefix() {
    let prefix_count = counts(&[("drivetrack", 2)]);

    let (dest_api, group_api) = resolve_dest(
        &base(),
        "drivetrack-api",
        &Category::Services,
        &prefix_count,
    );
    let (dest_web, group_web) =
        resolve_dest(&base(), "drivetrack-web", &Category::Ui, &prefix_count);

    assert_eq!(
        dest_api,
        PathBuf::from("/home/rachid/projects/services/drivetrack/drivetrack-api")
    );
    assert_eq!(group_api, Some("drivetrack".into()));

    assert_eq!(
        dest_web,
        PathBuf::from("/home/rachid/projects/ui/drivetrack/drivetrack-web")
    );
    assert_eq!(group_web, Some("drivetrack".into()));
}

#[test]
fn three_siblings_all_grouped() {
    let prefix_count = counts(&[("medlink", 3)]);

    for suffix in ["api", "web", "desk"] {
        let name = format!("medlink-{}", suffix);
        let (dest, group): (PathBuf, Option<String>) =
            resolve_dest(&base(), &name, &Category::Apps, &prefix_count);
        assert_eq!(
            dest,
            base().join("apps").join("medlink").join(&name),
            "expected group folder in path, got: {}",
            dest.display()
        );
        assert_eq!(group, Some("medlink".into()));
    }
}

#[test]
fn group_folder_is_just_the_prefix_not_full_name() {
    let prefix_count = counts(&[("pioneers", 2)]);
    let (dest, _) = resolve_dest(&base(), "pioneers-website", &Category::Ui, &prefix_count);
    // Should be  .../ui/pioneers/pioneers-website  NOT .../ui/pioneers-website/pioneers-website
    let components: Vec<std::path::Component> = dest.components().collect();
    let names: Vec<String> = components
        .iter()
        .map(|c: &std::path::Component| c.as_os_str().to_string_lossy().into_owned())
        .collect();

    assert!(
        names.contains(&"pioneers".to_string()),
        "group folder missing: {:?}",
        names
    );
    assert_eq!(names.last().unwrap(), "pioneers-website");
}

// ── Underscore separator ──────────────────────────────────────────────────────

#[test]
fn underscore_prefix_also_groups() {
    let prefix_count = counts(&[("gym", 2)]);
    let (dest, group): (PathBuf, Option<String>) =
        resolve_dest(&base(), "gym_api", &Category::Services, &prefix_count);
    assert_eq!(dest, base().join("services").join("gym").join("gym_api"));
    assert_eq!(group, Some("gym".into()));
}

// ── Category dir names in dest ────────────────────────────────────────────────

#[test]
fn dest_uses_correct_category_dir() {
    let cases = [
        ("my-api", Category::Services, "services"),
        ("my-web", Category::Ui, "ui"),
        ("my-desk", Category::Apps, "apps"),
        ("rocket-fw", Category::Embedded, "embedded"),
        ("trashnet", Category::Ml, "ml"),
        ("projm-cli", Category::Tools, "tools"),
        ("scratch", Category::Labs, "labs"),
    ];

    for (name, cat, expected_dir) in cases {
        let (dest, _): (PathBuf, Option<String>) = resolve_dest(&base(), name, &cat, &counts(&[]));
        assert!(
            dest.to_string_lossy().contains(expected_dir),
            "{name}: expected dir '{expected_dir}' in path '{}'",
            dest.display()
        );
    }
}

#[test]
fn test_organize_single_project() {
    let tmp = tempfile::tempdir().unwrap();

    // XDG_CONFIG_HOME only affects Linux; PROJM_CONFIG_DIR works on all platforms.
    std::env::set_var("XDG_CONFIG_HOME", tmp.path());
    std::env::set_var("PROJM_CONFIG_DIR", tmp.path());

    let base_dir = tmp.path().join("my_base_projects");
    let config_dir = tmp.path().join("projm");
    std::fs::create_dir_all(&config_dir).unwrap();

    // Write mock config.json
    let config_content = format!(
        r#"{{"base": "{}"}}"#,
        base_dir.display().to_string().replace('\\', "\\\\")
    );
    std::fs::write(config_dir.join("config.json"), config_content).unwrap();

    // Create a temporary source project directory
    let src_project = tmp.path().join("my-web-app");
    std::fs::create_dir_all(&src_project).unwrap();
    // Add package.json with some react dependencies to classify as UI
    std::fs::write(
        src_project.join("package.json"),
        r#"{"dependencies": {"react": "18.0.0"}}"#,
    )
    .unwrap();

    // Run organize_single
    let dest_path = projm_core::organize::organize_single(&src_project).unwrap();

    // It should be moved to my_base_projects/ui/my-web-app
    let expected_dest = base_dir.join("ui").join("my-web-app");
    assert_eq!(dest_path, expected_dest);
    assert!(expected_dest.exists());
    assert!(expected_dest.join("package.json").exists());
    assert!(!src_project.exists()); // Source should be moved
}

#[test]
fn test_is_monorepo_classification() {
    let tmp = tempfile::tempdir().unwrap();
    let rules = vec![];

    // 1. turbo.json
    let p1 = tmp.path().join("my-turbo-repo");
    std::fs::create_dir_all(&p1).unwrap();
    std::fs::write(p1.join("turbo.json"), "{}").unwrap();
    assert_eq!(projm_core::classify::classify(&p1, &rules), Category::Apps);

    // 2. package.json with workspaces
    let p2 = tmp.path().join("my-workspace-repo");
    std::fs::create_dir_all(&p2).unwrap();
    std::fs::write(p2.join("package.json"), r#"{"workspaces": ["apps/*"]}"#).unwrap();
    assert_eq!(projm_core::classify::classify(&p2, &rules), Category::Apps);

    // 3. package.json without workspaces (should be classified under UI/Services/etc. based on normal rules)
    let p3 = tmp.path().join("my-normal-repo");
    std::fs::create_dir_all(&p3).unwrap();
    std::fs::write(p3.join("package.json"), r#"{"dependencies": {"react": "18.0.0"}}"#).unwrap();
    assert_eq!(projm_core::classify::classify(&p3, &rules), Category::Ui);
}


