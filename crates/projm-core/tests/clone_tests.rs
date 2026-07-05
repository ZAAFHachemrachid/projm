use projm_core::clone::extract_repo_name;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_url_name_extraction() {
    assert_eq!(extract_repo_name("https://github.com/rust-lang/regex.git").unwrap(), "regex");
    assert_eq!(extract_repo_name("https://github.com/rust-lang/regex").unwrap(), "regex");
    assert_eq!(extract_repo_name("git@github.com:rust-lang/regex.git").unwrap(), "regex");
    assert_eq!(extract_repo_name("https://github.com/rust-lang/regex.git?query=1#frag").unwrap(), "regex");
    assert_eq!(extract_repo_name("file:///path/to/my-project.git").unwrap(), "my-project");
    assert_eq!(extract_repo_name(""), None);
}

fn setup_mock_git_remote() -> (TempDir, String) {
    let temp = tempfile::tempdir().unwrap();
    let repo_path = temp.path().join("my-rust-service-api");
    fs::create_dir_all(&repo_path).unwrap();

    // Init repository
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Set mock user for git commits
    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Add Cargo.toml to force Services classification
    let cargo_toml = r#"[package]
name = "my-rust-service-api"
version = "0.1.0"
"#;
    fs::write(repo_path.join("Cargo.toml"), cargo_toml).unwrap();

    // Commit files
    std::process::Command::new("git")
        .args(["add", "Cargo.toml"])
        .current_dir(&repo_path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "initial commit"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // git requires file URLs with forward slashes and a leading slash
    // (file:///C:/... on Windows), so normalize the platform path.
    let url_path = repo_path.display().to_string().replace('\\', "/");
    let remote_url = if url_path.starts_with('/') {
        format!("file://{url_path}")
    } else {
        format!("file:///{url_path}")
    };
    (temp, remote_url)
}

#[test]
fn test_clone_and_organize_integration() {
    let (_remote_dir, remote_url) = setup_mock_git_remote();

    let tmp = tempfile::tempdir().unwrap();
    // XDG_CONFIG_HOME only affects Linux; PROJM_CONFIG_DIR works on all platforms.
    std::env::set_var("XDG_CONFIG_HOME", tmp.path());
    std::env::set_var("PROJM_CONFIG_DIR", tmp.path());

    let base_dir = tmp.path().join("my_base_projects");
    let config_dir = tmp.path().join("projm");
    fs::create_dir_all(&config_dir).unwrap();

    // Write mock config.json
    let config_content = format!(
        r#"{{"base": "{}"}}"#,
        base_dir.display().to_string().replace('\\', "\\\\")
    );
    fs::write(config_dir.join("config.json"), config_content).unwrap();

    // Run clone
    projm_core::clone::run(&remote_url, None, None, false).unwrap();

    // The repo has suffix "-api" and is a Rust project (Cargo.toml),
    // which maps to Services category.
    // So it should land in: base_dir/services/my-rust-service-api
    let expected_dest = base_dir.join("services").join("my-rust-service-api");
    assert!(expected_dest.exists(), "Organized repository destination does not exist at {}", expected_dest.display());
    assert!(expected_dest.join("Cargo.toml").exists());
}
