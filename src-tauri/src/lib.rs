// Projm Tauri backend — desktop GUI for project organization and navigation.

use projm_core::{check, classify, organize};
use tauri::Emitter;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::io::{Read, Write};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};

// ── Types & Global State ──────────────────────────────────────────────────────

pub struct TerminalState {
    pub writer: Arc<Mutex<Option<Box<dyn std::io::Write + Send>>>>,
}

#[derive(serde::Serialize)]
struct ProjectItem {
    name: String,
    path: String,
    category: String,
    git_branch: Option<String>,
    git_dirty: Option<bool>,
}

struct GitInfo {
    branch: String,
    is_dirty: bool,
}

// ── Local Helpers ─────────────────────────────────────────────────────────────

fn is_group_folder(path: &std::path::Path) -> bool {
    let parent_name = match path.file_name() {
        Some(n) => n.to_string_lossy().to_lowercase(),
        None => return false,
    };
    std::fs::read_dir(path)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .any(|e| {
                    let child = e.file_name().to_string_lossy().to_lowercase();
                    child.starts_with(&format!("{}-", parent_name))
                        || child.starts_with(&format!("{}_", parent_name))
                })
        })
        .unwrap_or(false)
}

fn get_git_info(path: &std::path::Path) -> Option<GitInfo> {
    if !path.join(".git").exists() {
        return None;
    }

    let branch_output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(path)
        .output()
        .ok()?;

    if !branch_output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&branch_output.stdout).trim().to_string();
    if branch.is_empty() {
        return None;
    }

    let status_output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()
        .ok()?;

    let is_dirty = status_output.status.success() && !status_output.stdout.is_empty();

    Some(GitInfo { branch, is_dirty })
}

// ── Tauri Commands ────────────────────────────────────────────────────────────

#[tauri::command]
async fn cmd_spawn_terminal(
    cwd: String,
    state: tauri::State<'_, TerminalState>,
    window: tauri::Window,
) -> Result<(), String> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let shell_path = if std::path::Path::new("/bin/zsh").exists() {
        "/bin/zsh"
    } else if std::path::Path::new("/bin/bash").exists() {
        "/bin/bash"
    } else {
        "/bin/sh"
    };

    let mut cmd = CommandBuilder::new(shell_path);
    cmd.cwd(cwd);

    let _child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    let mut state_writer = state.writer.lock().await;
    *state_writer = Some(writer);

    let window_clone = window.clone();
    std::thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let _ = window_clone.emit("terminal-data", text);
                }
                Err(_) => break,
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn cmd_write_terminal(
    input: String,
    state: tauri::State<'_, TerminalState>,
) -> Result<(), String> {
    let mut writer_lock = state.writer.lock().await;
    if let Some(ref mut writer) = *writer_lock {
        writer.write_all(input.as_bytes()).map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn cmd_list_projects() -> Result<Vec<ProjectItem>, String> {
    let base = projm_core::config::load().base;
    if !base.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();

    for cat in projm_core::classify::Category::all() {
        let cat_dir = base.join(cat.dir_name());
        if !cat_dir.exists() {
            continue;
        }

        let mut top: Vec<_> = std::fs::read_dir(&cat_dir)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.'))
            .collect();
        top.sort_by_key(|e| e.file_name());

        for entry in top {
            let is_group = is_group_folder(&entry.path());
            if is_group {
                let mut children: Vec<_> = std::fs::read_dir(entry.path())
                    .map_err(|e| e.to_string())?
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.')
                    })
                    .collect();
                children.sort_by_key(|e| e.file_name());
                for child in children {
                    let child_path = child.path();
                    let git_info = get_git_info(&child_path);
                    projects.push(ProjectItem {
                        name: child.file_name().to_string_lossy().to_string(),
                        path: child_path.to_string_lossy().to_string(),
                        category: cat.dir_name().to_string(),
                        git_branch: git_info.as_ref().map(|g| g.branch.clone()),
                        git_dirty: git_info.as_ref().map(|g| g.is_dirty),
                    });
                }
            } else {
                let entry_path = entry.path();
                let git_info = get_git_info(&entry_path);
                projects.push(ProjectItem {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry_path.to_string_lossy().to_string(),
                    category: cat.dir_name().to_string(),
                    git_branch: git_info.as_ref().map(|g| g.branch.clone()),
                    git_dirty: git_info.as_ref().map(|g| g.is_dirty),
                });
            }
        }
    }

    Ok(projects)
}

#[tauri::command]
fn cmd_scan_directory(path: String, dry_run: bool) -> Result<String, String> {
    let path = std::path::PathBuf::from(&path);
    organize::run(&path, dry_run).map_err(|e| e.to_string())?;
    Ok("ok".into())
}

#[tauri::command]
fn cmd_set_base(path: String) -> Result<(), String> {
    projm_core::config::set_base(&std::path::PathBuf::from(&path)).map_err(|e| e.to_string())?;
    Ok(())
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
        .manage(TerminalState {
            writer: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            cmd_scan_directory,
            cmd_check_environment,
            cmd_set_base,
            cmd_get_editors,
            cmd_get_config,
            cmd_classify_project,
            cmd_spawn_terminal,
            cmd_write_terminal,
            cmd_list_projects,
        ])
        .run(tauri::generate_context!())
        .expect("error while running projm tauri application");
}
