use projm::{classify::Category, organize::resolve_dest};
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
        assert!(
            dest.to_string_lossy().contains("/apps/medlink/"),
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
    assert!(dest.to_string_lossy().contains("/services/gym/gym_api"));
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
