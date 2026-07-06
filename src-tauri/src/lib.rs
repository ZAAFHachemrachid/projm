// Projm Tauri backend — desktop GUI for project organization and navigation.

use projm_core::runner::{self, process as runner_process};
use projm_core::{agents, blueprints, check, classify, organize};
use tauri::{AppHandle, Emitter, Manager};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::io::{Read, Write};
use std::time::Duration;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};

// ── Types & Global State ──────────────────────────────────────────────────────

/// One live shell PTY bound to a single project directory. Holds the master
/// writer for input, the child handle for liveness checks and kill, and the
/// master PTY itself so the pseudo-terminal is not closed while the shell runs.
struct TermSession {
    writer: Box<dyn std::io::Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    _master: Box<dyn portable_pty::MasterPty + Send>,
}

/// Tauri-managed map of project cwd → live terminal session.
pub struct TerminalState {
    sessions: StdMutex<HashMap<String, TermSession>>,
}

/// Payload for the `terminal-data` event: which project produced the bytes and
/// the decoded chunk of shell output.
#[derive(serde::Serialize, Clone)]
struct TerminalData {
    cwd: String,
    data: String,
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

/// Spawn a shell PTY for `cwd` if none is live yet. Shared by the Shell tab
/// mount and agent launch, so both paths reuse a single session per project.
fn ensure_terminal(
    cwd: &str,
    state: &TerminalState,
    window: &tauri::Window,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;

    // Reuse path: a live session for this project already exists, so the
    // frontend remount must not spawn a second shell.
    if let Some(existing) = sessions.get_mut(cwd) {
        match existing.child.try_wait() {
            Ok(None) => return Ok(()),
            // Child exited, or we can't tell — drop the stale entry and respawn.
            _ => {
                sessions.remove(cwd);
            }
        }
    }

    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let shell_path = if cfg!(target_os = "windows") {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    } else {
        if std::path::Path::new("/bin/zsh").exists() {
            "/bin/zsh".to_string()
        } else if std::path::Path::new("/bin/bash").exists() {
            "/bin/bash".to_string()
        } else {
            "/bin/sh".to_string()
        }
    };

    let mut cmd = CommandBuilder::new(&shell_path);
    cmd.cwd(cwd);

    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    sessions.insert(
        cwd.to_string(),
        TermSession {
            writer,
            child,
            _master: pair.master,
        },
    );
    drop(sessions);

    let window_clone = window.clone();
    let event_cwd = cwd.to_string();
    std::thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let _ = window_clone.emit(
                        "terminal-data",
                        TerminalData {
                            cwd: event_cwd.clone(),
                            data,
                        },
                    );
                }
                Err(_) => break,
            }
        }
    });

    Ok(())
}

// ── Tauri Commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn cmd_spawn_terminal(
    cwd: String,
    state: tauri::State<'_, TerminalState>,
    window: tauri::Window,
) -> Result<(), String> {
    ensure_terminal(&cwd, &state, &window)
}

#[tauri::command]
fn cmd_write_terminal(
    cwd: String,
    input: String,
    state: tauri::State<'_, TerminalState>,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    let session = sessions
        .get_mut(&cwd)
        .ok_or_else(|| "no terminal session for this project".to_string())?;
    session
        .writer
        .write_all(input.as_bytes())
        .map_err(|e| e.to_string())?;
    session.writer.flush().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn cmd_kill_terminal(
    cwd: String,
    state: tauri::State<'_, TerminalState>,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    if let Some(mut session) = sessions.remove(&cwd) {
        // Best effort: the child may already have exited. Reap it off-thread
        // so no zombie lingers and the command returns immediately.
        let _ = session.child.kill();
        std::thread::spawn(move || {
            let _ = session.child.wait();
        });
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
fn cmd_classify_project(path: String) -> Result<String, String> {
    let path = std::path::Path::new(&path);
    let custom_rules = projm_core::rules::load_rules();
    let category = classify::classify(path, &custom_rules);
    Ok(category.dir_name().to_string())
}

#[tauri::command]
fn cmd_set_categories(categories: Vec<String>) -> Result<(), String> {
    projm_core::config::set_categories(categories).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn cmd_get_config() -> serde_json::Value {
    let cfg = projm_core::config::load();
    let categories = cfg.categories.unwrap_or_else(|| vec![
        "apps".to_string(),
        "services".to_string(),
        "ui".to_string(),
        "embedded".to_string(),
        "ml".to_string(),
        "tools".to_string(),
        "labs".to_string(),
        "content".to_string(),
    ]);
    serde_json::json!({
        "base": cfg.base.to_string_lossy(),
        "categories": categories,
    })
}

#[tauri::command]
fn cmd_get_rules_raw() -> Result<String, String> {
    projm_core::rules::read_rules_raw()
}

#[tauri::command]
fn cmd_save_rules_raw(content: String) -> Result<(), String> {
    projm_core::rules::save_rules_raw(&content)
}

// ── Blueprint Commands ────────────────────────────────────────────────────────

#[tauri::command]
fn cmd_get_blueprints() -> Result<Vec<blueprints::Blueprint>, String> {
    let store = blueprints::BlueprintsStore::load().map_err(|e| e.to_string())?;
    Ok(store.blueprints)
}

#[tauri::command]
fn cmd_add_blueprint(name: String, command: String) -> Result<(), String> {
    let mut store = blueprints::BlueprintsStore::load().map_err(|e| e.to_string())?;
    if store.blueprints.iter().any(|b| b.name == name) {
        return Err(format!("Blueprint '{}' already exists.", name));
    }
    store.blueprints.push(blueprints::Blueprint { name, command });
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn cmd_update_blueprint(old_name: String, name: String, command: String) -> Result<(), String> {
    let mut store = blueprints::BlueprintsStore::load().map_err(|e| e.to_string())?;
    let idx = store
        .blueprints
        .iter()
        .position(|b| b.name == old_name)
        .ok_or_else(|| format!("Blueprint '{}' not found.", old_name))?;
    if name != old_name && store.blueprints.iter().any(|b| b.name == name) {
        return Err(format!("Blueprint '{}' already exists.", name));
    }
    store.blueprints[idx].name = name;
    store.blueprints[idx].command = command;
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn cmd_delete_blueprint(name: String) -> Result<(), String> {
    let mut store = blueprints::BlueprintsStore::load().map_err(|e| e.to_string())?;
    let len_before = store.blueprints.len();
    store.blueprints.retain(|b| b.name != name);
    if store.blueprints.len() == len_before {
        return Err(format!("Blueprint '{}' not found.", name));
    }
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

// ── AI Agent Commands ─────────────────────────────────────────────────────────

#[tauri::command]
fn cmd_get_agents() -> Result<Vec<serde_json::Value>, String> {
    let store = agents::AgentsStore::load().map_err(|e| e.to_string())?;
    Ok(store
        .agents
        .iter()
        .map(|a| {
            let detected = agents::detect_path(&a.command);
            serde_json::json!({
                "name": a.name,
                "command": a.command,
                "binary": agents::agent_binary(&a.command),
                "installed": detected.is_some(),
                "path": detected.map(|p| p.to_string_lossy().to_string()),
            })
        })
        .collect())
}

#[tauri::command]
fn cmd_add_agent(name: String, command: String) -> Result<(), String> {
    let mut store = agents::AgentsStore::load().map_err(|e| e.to_string())?;
    if store.agents.iter().any(|a| a.name == name) {
        return Err(format!("Agent '{}' already exists.", name));
    }
    store.agents.push(agents::Agent { name, command });
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn cmd_update_agent(old_name: String, name: String, command: String) -> Result<(), String> {
    let mut store = agents::AgentsStore::load().map_err(|e| e.to_string())?;
    let idx = store
        .agents
        .iter()
        .position(|a| a.name == old_name)
        .ok_or_else(|| format!("Agent '{}' not found.", old_name))?;
    if name != old_name && store.agents.iter().any(|a| a.name == name) {
        return Err(format!("Agent '{}' already exists.", name));
    }
    store.agents[idx].name = name;
    store.agents[idx].command = command;
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn cmd_delete_agent(name: String) -> Result<(), String> {
    let mut store = agents::AgentsStore::load().map_err(|e| e.to_string())?;
    let len_before = store.agents.len();
    store.agents.retain(|a| a.name != name);
    if store.agents.len() == len_before {
        return Err(format!("Agent '{}' not found.", name));
    }
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// Launch an AI agent inside the project's shell session: make sure the PTY
/// exists, then type the agent's command into it as if the user had.
#[tauri::command]
fn cmd_launch_agent(
    cwd: String,
    name: String,
    state: tauri::State<'_, TerminalState>,
    window: tauri::Window,
) -> Result<(), String> {
    let store = agents::AgentsStore::load().map_err(|e| e.to_string())?;
    let agent = store
        .agents
        .iter()
        .find(|a| a.name == name)
        .ok_or_else(|| format!("Agent '{}' not found.", name))?;
    if agents::detect_path(&agent.command).is_none() {
        return Err(format!(
            "'{}' is not installed — binary '{}' was not found on $PATH.",
            agent.name,
            agents::agent_binary(&agent.command)
        ));
    }

    ensure_terminal(&cwd, &state, &window)?;

    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    let session = sessions
        .get_mut(&cwd)
        .ok_or_else(|| "no terminal session for this project".to_string())?;
    session
        .writer
        .write_all(format!("{}\n", agent.command).as_bytes())
        .map_err(|e| e.to_string())?;
    session.writer.flush().map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    git_status: Option<String>,
}

#[tauri::command]
fn cmd_read_dir(path: String) -> Result<Vec<FileEntry>, String> {
    let dir_path = std::path::Path::new(&path);
    if !dir_path.exists() {
        return Err("Path does not exist".into());
    }
    if !dir_path.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    let read_entries = std::fs::read_dir(dir_path).map_err(|e| e.to_string())?;

    // Try to get git statuses for this directory
    let mut git_statuses = std::collections::HashMap::new();
    if let Ok(output) = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir_path)
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.len() > 3 {
                    let status = line[..2].trim().to_string();
                    let file_path = line[3..].trim().to_string();
                    git_statuses.insert(file_path, status);
                }
            }
        }
    }

    for entry in read_entries.filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') && name != ".github" && name != ".agents" && name != ".impeccable" && name != ".gitignore" {
            continue;
        }

        let is_dir = entry_path.is_dir();
        
        let mut git_status = None;
        for (g_path, status) in &git_statuses {
            if g_path == &name || g_path.starts_with(&format!("{}/", name)) {
                git_status = Some(status.clone());
                break;
            }
        }

        entries.push(FileEntry {
            name,
            path: entry_path.to_string_lossy().to_string(),
            is_dir,
            git_status,
        });
    }

    entries.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        } else {
            b.is_dir.cmp(&a.is_dir)
        }
    });

    Ok(entries)
}

// ── Dev runner (multi-app tabbed runner, GUI side) ────────────────────────────

/// One running dev-runner session, keyed by project root path. Holds the shared
/// runner state (from projm_core) plus a flag to stop its background poller.
struct RunnerSession {
    shared: runner_process::Shared,
    dirty: runner_process::Dirty,
    alive: Arc<AtomicBool>,
}

/// Tauri-managed map of project path → live runner session.
#[derive(Default)]
pub struct RunnerState {
    sessions: StdMutex<HashMap<String, RunnerSession>>,
}

#[derive(serde::Serialize, Clone)]
struct RunnerAppDto {
    id: String,
    label: String,
    dir: String,
    command: String,
    port: Option<u16>,
    hint: String,
    status: String,
    pid: Option<u32>,
}

#[derive(serde::Serialize, Clone)]
struct RunnerLogEvent {
    project: String,
    app_id: String,
    lines: Vec<String>,
}

#[derive(serde::Serialize, Clone)]
struct RunnerStatusEvent {
    project: String,
    app_id: String,
    status: String,
    pid: Option<u32>,
}

fn status_str(s: runner_process::Status) -> &'static str {
    match s {
        runner_process::Status::Stopped => "stopped",
        runner_process::Status::Starting => "starting",
        runner_process::Status::Running => "running",
        runner_process::Status::Errored => "errored",
    }
}

/// Find an app's index within a session by its stable id.
fn index_of(shared: &runner_process::Shared, app_id: &str) -> Option<usize> {
    let st = shared.lock().ok()?;
    st.apps.iter().position(|a| a.id == app_id)
}

/// Background thread: reap children and stream log deltas + status changes to the
/// frontend as Tauri events until the session is torn down.
fn spawn_poller(
    project: String,
    shared: runner_process::Shared,
    dirty: runner_process::Dirty,
    alive: Arc<AtomicBool>,
    app: AppHandle,
) {
    std::thread::spawn(move || {
        // Per-app cursor: (last emitted logged_total, last emitted status).
        let mut cursors: HashMap<usize, (u64, runner_process::Status)> = HashMap::new();
        while alive.load(Ordering::Relaxed) {
            runner_process::poll(&shared, &dirty);
            if let Ok(st) = shared.lock() {
                for (i, rt) in st.runtimes.iter().enumerate() {
                    let app_id = st.apps[i].id.clone();
                    let prev = cursors.get(&i).copied();

                    // Stream any log lines pushed since the last tick.
                    let total = rt.logged_total;
                    let last_total = prev.map(|(t, _)| t).unwrap_or(0);
                    if total > last_total {
                        let new_count = (total - last_total) as usize;
                        let have = rt.logs.len();
                        let take = new_count.min(have);
                        let lines: Vec<String> =
                            rt.logs.iter().skip(have - take).cloned().collect();
                        let _ = app.emit(
                            "runner:log",
                            RunnerLogEvent {
                                project: project.clone(),
                                app_id: app_id.clone(),
                                lines,
                            },
                        );
                    }

                    // Emit status on first sight or on change.
                    let status_changed = prev.map(|(_, s)| s != rt.status).unwrap_or(true);
                    if status_changed {
                        let _ = app.emit(
                            "runner:status",
                            RunnerStatusEvent {
                                project: project.clone(),
                                app_id: app_id.clone(),
                                status: status_str(rt.status).to_string(),
                                pid: rt.pid,
                            },
                        );
                    }

                    cursors.insert(i, (total, rt.status));
                }
            }
            std::thread::sleep(Duration::from_millis(120));
        }
    });
}

/// Discover the runnable apps in a project, creating (or reusing) its runner session.
#[tauri::command]
fn cmd_runner_discover(
    project_path: String,
    state: tauri::State<'_, RunnerState>,
    app: AppHandle,
) -> Result<Vec<RunnerAppDto>, String> {
    let root = PathBuf::from(&project_path);
    let apps = runner::discover_apps(&root);

    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    if !sessions.contains_key(&project_path) {
        let runtimes = apps
            .iter()
            .map(|_| runner_process::AppRuntime::new())
            .collect();
        let shared: runner_process::Shared =
            Arc::new(StdMutex::new(runner_process::TuiState {
                apps: apps.clone(),
                runtimes,
                selected: 0,
                quitting: false,
            }));
        let dirty: runner_process::Dirty = Arc::new(AtomicBool::new(true));
        let alive = Arc::new(AtomicBool::new(true));
        spawn_poller(
            project_path.clone(),
            shared.clone(),
            dirty.clone(),
            alive.clone(),
            app,
        );
        sessions.insert(
            project_path.clone(),
            RunnerSession { shared, dirty, alive },
        );
    }

    let sess = sessions.get(&project_path).unwrap();
    let st = sess.shared.lock().map_err(|e| e.to_string())?;
    Ok(st
        .apps
        .iter()
        .enumerate()
        .map(|(i, a)| RunnerAppDto {
            id: a.id.clone(),
            label: a.label.clone(),
            dir: a.dir.display().to_string(),
            command: a.command.clone(),
            port: a.port,
            hint: a.hint.clone(),
            status: status_str(st.runtimes[i].status).to_string(),
            pid: st.runtimes[i].pid,
        })
        .collect())
}

#[tauri::command]
fn cmd_runner_start(
    project_path: String,
    app_id: String,
    state: tauri::State<'_, RunnerState>,
) -> Result<(), String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    let sess = sessions.get(&project_path).ok_or("no runner session")?;
    let idx = index_of(&sess.shared, &app_id).ok_or("unknown app")?;
    runner_process::start(&sess.shared, &sess.dirty, idx);
    Ok(())
}

#[tauri::command]
fn cmd_runner_stop(
    project_path: String,
    app_id: String,
    state: tauri::State<'_, RunnerState>,
) -> Result<(), String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    let sess = sessions.get(&project_path).ok_or("no runner session")?;
    let idx = index_of(&sess.shared, &app_id).ok_or("unknown app")?;
    runner_process::stop(&sess.shared, idx, false);
    Ok(())
}

#[tauri::command]
fn cmd_runner_restart(
    project_path: String,
    app_id: String,
    state: tauri::State<'_, RunnerState>,
) -> Result<(), String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    let sess = sessions.get(&project_path).ok_or("no runner session")?;
    let idx = index_of(&sess.shared, &app_id).ok_or("unknown app")?;
    runner_process::restart(&sess.shared, &sess.dirty, idx);
    Ok(())
}

#[tauri::command]
fn cmd_runner_free_port(
    project_path: String,
    app_id: String,
    state: tauri::State<'_, RunnerState>,
) -> Result<(), String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    let sess = sessions.get(&project_path).ok_or("no runner session")?;
    let idx = index_of(&sess.shared, &app_id).ok_or("unknown app")?;
    runner_process::free_port_for(&sess.shared, &sess.dirty, idx);
    Ok(())
}

#[tauri::command]
fn cmd_runner_start_all(
    project_path: String,
    state: tauri::State<'_, RunnerState>,
) -> Result<(), String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    let sess = sessions.get(&project_path).ok_or("no runner session")?;
    runner_process::start_all(&sess.shared, &sess.dirty);
    Ok(())
}

#[tauri::command]
fn cmd_runner_stop_all(
    project_path: String,
    state: tauri::State<'_, RunnerState>,
) -> Result<(), String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    let sess = sessions.get(&project_path).ok_or("no runner session")?;
    runner_process::stop_all(&sess.shared);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(TerminalState {
            sessions: StdMutex::new(HashMap::new()),
        })
        .manage(RunnerState::default())
        .invoke_handler(tauri::generate_handler![
            cmd_scan_directory,
            cmd_check_environment,
            cmd_set_base,
            cmd_get_editors,
            cmd_get_config,
            cmd_classify_project,
            cmd_spawn_terminal,
            cmd_write_terminal,
            cmd_kill_terminal,
            cmd_list_projects,
            cmd_set_categories,
            cmd_get_rules_raw,
            cmd_save_rules_raw,
            cmd_get_blueprints,
            cmd_add_blueprint,
            cmd_update_blueprint,
            cmd_delete_blueprint,
            cmd_get_agents,
            cmd_add_agent,
            cmd_update_agent,
            cmd_delete_agent,
            cmd_launch_agent,
            cmd_read_dir,
            cmd_runner_discover,
            cmd_runner_start,
            cmd_runner_stop,
            cmd_runner_restart,
            cmd_runner_free_port,
            cmd_runner_start_all,
            cmd_runner_stop_all,
        ])
        .build(tauri::generate_context!())
        .expect("error while building projm tauri application")
        .run(|app_handle, event| {
            // On exit, stop every runner poller and reap all child process groups
            // so closing the window never leaves orphaned dev servers behind.
            if let tauri::RunEvent::ExitRequested { .. } = &event {
                let state = app_handle.state::<RunnerState>();
                let guard = state.sessions.lock();
                if let Ok(sessions) = guard {
                    for sess in sessions.values() {
                        sess.alive.store(false, Ordering::Relaxed);
                        runner_process::shutdown(&sess.shared);
                    }
                }
            }
        });
}
