//! Dynamic, polyglot discovery of the runnable apps inside a repo.
//!
//! Each ecosystem enumerator is best-effort and swallows its own errors (missing tool or
//! manifest ⇒ contributes nothing). Where possible the enumerators reuse the proven dev-command
//! resolution already living in [`crate::run`] rather than re-deriving it.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::RunnableApp;

/// Discover every runnable app under `root`, deduped and stably ordered.
pub fn discover_apps(root: &Path) -> Vec<RunnableApp> {
    let mut out = Vec::new();
    js_apps(root, &mut out);
    cargo_apps(root, &mut out);
    python_apps(root, &mut out);
    cmake_apps(root, &mut out);
    make_apps(root, &mut out);
    platformio_apps(root, &mut out);
    arduino_apps(root, &mut out);

    out.sort_by(|a, b| a.id.cmp(&b.id));
    out.dedup_by(|a, b| a.dir == b.dir && a.command == b.command);
    dedupe_ids(&mut out);
    out
}

/// Parse a dev port out of a command / script string. Best-effort — never gates startup.
pub fn extract_port(cmd: &str) -> Option<u16> {
    let toks: Vec<&str> = cmd.split_whitespace().collect();
    for (i, t) in toks.iter().enumerate() {
        for pfx in ["--port=", "-p=", "--port ", "-p "] {
            if let Some(rest) = t.strip_prefix(pfx.trim_end()) {
                if let Some(rest) = rest.strip_prefix('=') {
                    if let Ok(p) = rest.parse() {
                        return Some(p);
                    }
                }
            }
        }
        if *t == "--port" || *t == "-p" {
            if let Some(n) = toks.get(i + 1) {
                if let Ok(p) = n.parse() {
                    return Some(p);
                }
            }
        }
    }
    let l = cmd.to_lowercase();
    if l.contains("vite") {
        return Some(5173);
    }
    if l.contains("astro") {
        return Some(4321);
    }
    if l.contains("next") {
        return Some(3000);
    }
    None
}

// ── JS/TS ────────────────────────────────────────────────────────────────────────

fn js_apps(root: &Path, out: &mut Vec<RunnableApp>) {
    let pkg = root.join("package.json");
    if !pkg.exists() {
        return;
    }
    let scripts = read_scripts(&pkg);

    // Root-level `dev:*` scripts (e.g. dev:web, dev:server) — the medlink/drive-track pattern.
    let pm = pm_run_prefix(root);
    let dev_scripts: Vec<&(String, String)> = scripts
        .iter()
        .filter(|(k, _)| k.starts_with("dev:"))
        .collect();
    if !dev_scripts.is_empty() {
        for (k, v) in dev_scripts {
            let label = k.trim_start_matches("dev:").to_string();
            let command = format!("{pm} {k}");
            let port = extract_port(v).or_else(|| extract_port(&command));
            out.push(RunnableApp {
                id: label.clone(),
                label,
                dir: root.to_path_buf(),
                command,
                port,
                hint: "js".to_string(),
            });
        }
        return;
    }

    // Monorepo workspaces → one app per package that has a dev/start script.
    if let Some(tool) = crate::run::detect_monorepo(root) {
        let mut pkgs = crate::run::resolve_workspace_packages(root, &tool);
        if pkgs.is_empty() {
            // turbo/nx: resolver returns nothing — fall back to conventional layout.
            for g in ["apps/*", "packages/*"] {
                crate::run::resolve_glob(root, g, &mut pkgs);
            }
        }
        pkgs.sort();
        pkgs.dedup();
        for rel in pkgs {
            let dir = root.join(&rel);
            let pj = dir.join("package.json");
            if !pj.exists() || !has_dev_script(&pj) {
                continue;
            }
            let command = match crate::run::resolve_dev_command(&dir) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let label = dir
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| rel.clone());
            // Port comes from the script body (e.g. `vite --port 5200`), not the
            // `bun run dev` wrapper resolve_dev_command returns.
            let port = dev_script_port(&pj).or_else(|| extract_port(&command));
            out.push(RunnableApp {
                id: label.clone(),
                label,
                dir,
                command,
                port,
                hint: "js pkg".to_string(),
            });
        }
        return;
    }

    // Single JS/TS app.
    if has_dev_script(&pkg) {
        if let Ok(command) = crate::run::resolve_dev_command(root) {
            let label = root
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "app".to_string());
            let port = extract_port(&command);
            out.push(RunnableApp {
                id: label.clone(),
                label,
                dir: root.to_path_buf(),
                command,
                port,
                hint: "js".to_string(),
            });
        }
    }
}

fn read_scripts(pkg: &Path) -> Vec<(String, String)> {
    let content = match fs::read_to_string(pkg) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    match json.get("scripts").and_then(|s| s.as_object()) {
        Some(obj) => obj
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect(),
        None => Vec::new(),
    }
}

fn has_dev_script(pkg: &Path) -> bool {
    read_scripts(pkg)
        .iter()
        .any(|(k, _)| k == "dev" || k == "start")
}

/// Extract a port from a package's `dev` (then `start`) script body.
fn dev_script_port(pkg: &Path) -> Option<u16> {
    let scripts = read_scripts(pkg);
    for key in ["dev", "start"] {
        if let Some((_, v)) = scripts.iter().find(|(k, _)| k == key) {
            if let Some(p) = extract_port(v) {
                return Some(p);
            }
        }
    }
    None
}

fn pm_run_prefix(root: &Path) -> &'static str {
    if root.join("bun.lock").exists() || root.join("bun.lockb").exists() {
        "bun run"
    } else if root.join("pnpm-lock.yaml").exists() {
        "pnpm"
    } else if root.join("yarn.lock").exists() {
        "yarn"
    } else {
        "npm run"
    }
}

// ── Rust (Cargo workspace members) ─────────────────────────────────────────────────

fn cargo_apps(root: &Path, out: &mut Vec<RunnableApp>) {
    let cargo = root.join("Cargo.toml");
    let content = match fs::read_to_string(&cargo) {
        Ok(c) => c,
        Err(_) => return,
    };
    let val: toml::Value = match content.parse() {
        Ok(v) => v,
        Err(_) => return,
    };
    let members = match val
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
    {
        Some(m) => m,
        None => return,
    };

    let mut rels: Vec<String> = Vec::new();
    for m in members {
        if let Some(s) = m.as_str() {
            if s.contains('*') {
                crate::run::resolve_glob(root, s, &mut rels);
            } else {
                rels.push(s.to_string());
            }
        }
    }
    rels.sort();
    rels.dedup();

    for rel in rels {
        let dir = root.join(&rel);
        let ct = dir.join("Cargo.toml");
        let ct_content = match fs::read_to_string(&ct) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let has_bin = dir.join("src/main.rs").exists() || ct_content.contains("[[bin]]");
        if !has_bin {
            continue;
        }
        let name = ct_content
            .parse::<toml::Value>()
            .ok()
            .and_then(|v| {
                v.get("package")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| {
                dir.file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| rel.clone())
            });
        out.push(RunnableApp {
            id: name.clone(),
            label: name.clone(),
            // `cargo run -p <name>` resolves the member from the workspace root.
            dir: root.to_path_buf(),
            command: format!("cargo run -p {name}"),
            port: None,
            hint: "rust bin".to_string(),
        });
    }
}

// ── Python ─────────────────────────────────────────────────────────────────────────

fn python_apps(root: &Path, out: &mut Vec<RunnableApp>) {
    let rd = match fs::read_dir(root) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    for e in rd.flatten() {
        let dir = e.path();
        if !dir.is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if !(dir.join("pyproject.toml").exists() || dir.join("requirements.txt").exists()) {
            continue;
        }
        if let Ok(command) = crate::run::resolve_dev_command(&dir) {
            out.push(RunnableApp {
                id: name.clone(),
                label: name,
                dir,
                command,
                port: None,
                hint: "python".to_string(),
            });
        }
    }
}

// ── C/C++ (CMake) ──────────────────────────────────────────────────────────────────

fn cmake_apps(root: &Path, out: &mut Vec<RunnableApp>) {
    let f = root.join("CMakeLists.txt");
    let content = match fs::read_to_string(&f) {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut targets = Vec::new();
    for (idx, _) in content.match_indices("add_executable(") {
        let rest = &content[idx + "add_executable(".len()..];
        let name: String = rest
            .trim_start()
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != ')' && *c != '$')
            .collect();
        if !name.is_empty() {
            targets.push(name);
        }
    }
    targets.sort();
    targets.dedup();

    if targets.is_empty() {
        out.push(RunnableApp {
            id: "cmake".to_string(),
            label: "cmake".to_string(),
            dir: root.to_path_buf(),
            command: "cmake --build build".to_string(),
            port: None,
            hint: "cmake".to_string(),
        });
        return;
    }
    for t in targets {
        out.push(RunnableApp {
            id: t.clone(),
            label: t.clone(),
            dir: root.to_path_buf(),
            command: format!("cmake --build build --target {t}"),
            port: None,
            hint: "cmake target".to_string(),
        });
    }
}

// ── Make ───────────────────────────────────────────────────────────────────────────

const MAKE_TARGETS: &[&str] = &[
    "run", "dev", "serve", "start", "flash", "upload", "monitor", "watch",
];

fn make_apps(root: &Path, out: &mut Vec<RunnableApp>) {
    let mut content = String::new();
    for f in ["Makefile", "makefile", "GNUmakefile"] {
        if let Ok(c) = fs::read_to_string(root.join(f)) {
            content = c;
            break;
        }
    }
    if content.is_empty() {
        return;
    }
    for line in content.lines() {
        // Top-level `target:` rule (not indented, not a variable assignment).
        if line.starts_with([' ', '\t', '#', '.']) {
            continue;
        }
        let Some(colon) = line.find(':') else {
            continue;
        };
        if line[colon..].starts_with(":=") || line.contains('=') && line.find('=') < Some(colon) {
            continue;
        }
        let target = line[..colon].trim();
        if MAKE_TARGETS.contains(&target) {
            out.push(RunnableApp {
                id: target.to_string(),
                label: target.to_string(),
                dir: root.to_path_buf(),
                command: format!("make {target}"),
                port: None,
                hint: "make".to_string(),
            });
        }
    }
}

// ── Arduino / ESP32 (PlatformIO) ───────────────────────────────────────────────────

fn platformio_apps(root: &Path, out: &mut Vec<RunnableApp>) {
    let content = match fs::read_to_string(root.join("platformio.ini")) {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut boards: HashMap<String, String> = HashMap::new();
    let mut envs: Vec<String> = Vec::new();
    let mut cur: Option<String> = None;
    for line in content.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("[env:") {
            if let Some(name) = rest.strip_suffix(']') {
                cur = Some(name.to_string());
                envs.push(name.to_string());
            }
        } else if t.starts_with('[') {
            cur = None;
        } else if let (Some(env), Some(board)) = (&cur, t.strip_prefix("board")) {
            if let Some(v) = board.trim_start().strip_prefix('=') {
                boards.insert(env.clone(), v.trim().to_string());
            }
        }
    }
    for env in envs {
        let hint = boards
            .get(&env)
            .map(|b| format!("board={b}"))
            .unwrap_or_else(|| "platformio".to_string());
        out.push(RunnableApp {
            id: env.clone(),
            label: env.clone(),
            dir: root.to_path_buf(),
            command: format!("pio run -e {env} -t upload"),
            port: None,
            hint,
        });
    }
}

fn arduino_apps(root: &Path, out: &mut Vec<RunnableApp>) {
    if root.join("platformio.ini").exists() {
        return; // PlatformIO governs; don't double-list.
    }
    let has_ino = fs::read_dir(root)
        .map(|rd| {
            rd.flatten().any(|e| {
                e.path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("ino"))
            })
        })
        .unwrap_or(false);
    if !has_ino {
        return;
    }
    let label = root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "sketch".to_string());
    out.push(RunnableApp {
        id: label.clone(),
        label,
        dir: root.to_path_buf(),
        command: "arduino-cli compile --upload".to_string(),
        port: None,
        hint: "arduino (set FQBN)".to_string(),
    });
}

// ── Helpers ────────────────────────────────────────────────────────────────────────

/// Ensure every app has a unique `id` by suffixing collisions (`api`, `api-2`, …).
fn dedupe_ids(apps: &mut [RunnableApp]) {
    let mut seen: HashMap<String, u32> = HashMap::new();
    for a in apps.iter_mut() {
        let n = seen.entry(a.id.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            a.id = format!("{}-{}", a.id, *n);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Path, rel: &str, content: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }

    #[test]
    fn extract_port_variants() {
        assert_eq!(extract_port("vite --port 5199"), Some(5199));
        assert_eq!(extract_port("next dev -p 4000"), Some(4000));
        assert_eq!(extract_port("node server --port=3001"), Some(3001));
        assert_eq!(extract_port("vite"), Some(5173));
        assert_eq!(extract_port("next dev"), Some(3000));
        assert_eq!(extract_port("astro dev"), Some(4321));
        assert_eq!(extract_port("cargo run -p api"), None);
    }

    #[test]
    fn root_dev_scripts_become_apps() {
        let td = TempDir::new().unwrap();
        write(
            td.path(),
            "package.json",
            r#"{ "scripts": { "dev:web": "vite --port 3000", "dev:server": "bun run src/index.ts", "build": "tsc" } }"#,
        );
        write(td.path(), "bun.lock", "");
        let apps = discover_apps(td.path());
        let ids: Vec<&str> = apps.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"web"), "got {ids:?}");
        assert!(ids.contains(&"server"), "got {ids:?}");
        let web = apps.iter().find(|a| a.id == "web").unwrap();
        assert_eq!(web.command, "bun run dev:web");
        assert_eq!(web.port, Some(3000));
    }

    #[test]
    fn bun_workspace_packages() {
        let td = TempDir::new().unwrap();
        write(td.path(), "package.json", r#"{ "workspaces": ["apps/*"] }"#);
        write(td.path(), "bun.lock", "");
        write(
            td.path(),
            "apps/web/package.json",
            r#"{ "scripts": { "dev": "vite --port 5200" } }"#,
        );
        write(
            td.path(),
            "apps/api/package.json",
            r#"{ "scripts": { "dev": "bun run index.ts" } }"#,
        );
        let apps = discover_apps(td.path());
        let ids: Vec<&str> = apps.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"web"), "got {ids:?}");
        assert!(ids.contains(&"api"), "got {ids:?}");
        assert_eq!(
            apps.iter().find(|a| a.id == "web").unwrap().port,
            Some(5200)
        );
    }

    #[test]
    fn cargo_workspace_members() {
        let td = TempDir::new().unwrap();
        write(
            td.path(),
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\"]\n",
        );
        write(
            td.path(),
            "crates/cli/Cargo.toml",
            "[package]\nname = \"my-cli\"\n",
        );
        write(td.path(), "crates/cli/src/main.rs", "fn main(){}");
        write(
            td.path(),
            "crates/lib/Cargo.toml",
            "[package]\nname = \"my-lib\"\n",
        );
        write(td.path(), "crates/lib/src/lib.rs", "");
        let apps = discover_apps(td.path());
        let ids: Vec<&str> = apps.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"my-cli"), "got {ids:?}");
        assert!(
            !ids.contains(&"my-lib"),
            "lib-only member must be skipped: {ids:?}"
        );
        assert_eq!(
            apps.iter().find(|a| a.id == "my-cli").unwrap().command,
            "cargo run -p my-cli"
        );
    }

    #[test]
    fn platformio_envs() {
        let td = TempDir::new().unwrap();
        write(
            td.path(),
            "platformio.ini",
            "[env:esp32dev]\nboard = esp32dev\nframework = arduino\n\n[env:nodemcu]\nboard = nodemcuv2\n",
        );
        let apps = discover_apps(td.path());
        let ids: Vec<&str> = apps.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"esp32dev"), "got {ids:?}");
        assert!(ids.contains(&"nodemcu"), "got {ids:?}");
        let e = apps.iter().find(|a| a.id == "esp32dev").unwrap();
        assert_eq!(e.command, "pio run -e esp32dev -t upload");
        assert_eq!(e.hint, "board=esp32dev");
    }

    #[test]
    fn cmake_targets() {
        let td = TempDir::new().unwrap();
        write(
            td.path(),
            "CMakeLists.txt",
            "cmake_minimum_required(VERSION 3.10)\nadd_executable(app main.cpp)\nadd_executable(tool tool.cpp)\n",
        );
        let apps = discover_apps(td.path());
        let ids: Vec<&str> = apps.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"app"), "got {ids:?}");
        assert!(ids.contains(&"tool"), "got {ids:?}");
        assert_eq!(
            apps.iter().find(|a| a.id == "app").unwrap().command,
            "cmake --build build --target app"
        );
    }

    #[test]
    fn make_run_targets() {
        let td = TempDir::new().unwrap();
        write(
            td.path(),
            "Makefile",
            "CC = gcc\nall: build\nrun:\n\t./a.out\nclean:\n\trm -f a.out\n",
        );
        let apps = discover_apps(td.path());
        let ids: Vec<&str> = apps.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"run"), "got {ids:?}");
        assert!(
            !ids.contains(&"clean"),
            "clean must be filtered out: {ids:?}"
        );
        assert!(!ids.contains(&"all"), "all must be filtered out: {ids:?}");
    }
}
