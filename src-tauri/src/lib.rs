// Projm Tauri backend — desktop GUI for project organization and navigation.

use projm_core::{check, classify, organize};

#[tauri::command]
fn cmd_scan_directory(path: String, dry_run: bool) -> Result<String, String> {
    let path = std::path::PathBuf::from(&path);
    organize::run(&path, dry_run).map_err(|e| e.to_string())?;
    Ok("ok".into())
}

#[tauri::command]
fn cmd_check_environment() -> Result<String, String> {
    check::run().map_err(|e| e.to_string())?;
    Ok("ok".into())
}

#[tauri::command]
fn cmd_get_editors() -> Vec<serde_json::Value> {
    projm_core::editors::detect_installed()
        .iter()
        .map(|e| {
            serde_json::json!({
                "binary": e.binary,
                "name": e.name,
                "path": e.path.to_string_lossy(),
            })
        })
        .collect()
}

#[tauri::command]
fn cmd_get_config() -> serde_json::Value {
    let cfg = projm_core::config::load();
    serde_json::json!({
        "base": cfg.base.to_string_lossy(),
    })
}

#[tauri::command]
fn cmd_classify_project(path: String) -> Result<String, String> {
    let path = std::path::Path::new(&path);
    let custom_rules = projm_core::rules::load_rules();
    let category = classify::classify(path, &custom_rules);
    Ok(category.dir_name().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            cmd_scan_directory,
            cmd_check_environment,
            cmd_get_editors,
            cmd_get_config,
            cmd_classify_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running projm tauri application");
}
