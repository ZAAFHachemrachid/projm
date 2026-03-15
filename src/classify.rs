use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::Path;

// ── Category ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Category {
    Apps,
    Services,
    Ui,
    Embedded,
    Ml,
    Tools,
    Labs,
}

impl Category {
    pub fn dir_name(&self) -> &'static str {
        match self {
            Self::Apps     => "apps",
            Self::Services => "services",
            Self::Ui       => "ui",
            Self::Embedded => "embedded",
            Self::Ml       => "ml",
            Self::Tools    => "tools",
            Self::Labs     => "labs",
        }
    }

    /// Fixed-width colored label for aligned display
    pub fn label(&self) -> String {
        let s = format!("{:<8}", self.dir_name());
        match self {
            Self::Apps     => s.blue().bold().to_string(),
            Self::Services => s.cyan().bold().to_string(),
            Self::Ui       => s.magenta().bold().to_string(),
            Self::Embedded => s.yellow().bold().to_string(),
            Self::Ml       => s.green().bold().to_string(),
            Self::Tools    => s.white().bold().to_string(),
            Self::Labs     => s.truecolor(255, 100, 50).bold().to_string(),
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Apps,
            Self::Services,
            Self::Ui,
            Self::Embedded,
            Self::Ml,
            Self::Tools,
            Self::Labs,
        ]
    }
}

// ── Known project suffixes ────────────────────────────────────────────────────
// Recognised separators: `-` or `_`

pub const KNOWN_SUFFIXES: &[&str] = &[
    "api", "web", "mob", "mobile", "desk", "desktop",
    "mono", "cli", "fw", "lib", "core", "ui",
    // common real-world names people actually use
    "backend", "frontend", "server", "client",
    "app", "apps", "bot", "worker", "jobs",
    "admin", "dashboard", "landing", "docs",
];

/// Split `DriveTrack-Backend` -> `Some(("DriveTrack", "backend"))`.
/// Matching is case-insensitive on the suffix; the returned prefix slice
/// preserves the original casing of `name`.
/// Returns `None` if the name has no recognised suffix.
pub fn split_suffix(name: &str) -> Option<(&str, &str)> {
    for sep in ['-', '_'] {
        if let Some(pos) = name.rfind(sep) {
            let suffix_raw = &name[pos + 1..];
            let suffix_low = suffix_raw.to_lowercase();
            if KNOWN_SUFFIXES.contains(&suffix_low.as_str()) {
                return Some((&name[..pos], suffix_raw));
            }
        }
    }
    None
}

/// Normalised group key: both `DriveTrack-Api` and `drivetrack-web`
/// map to `"drivetrack"`.
pub fn prefix_key(name: &str) -> Option<String> {
    split_suffix(name).map(|(prefix, _)| prefix.to_lowercase())
}

// ── Classification logic ──────────────────────────────────────────────────────

pub fn classify(path: &Path) -> Category {
    let has = |f: &str| path.join(f).exists();

    // ── doc-lab.md is the explicit labs marker — highest priority ──────────
    if has("doc-lab.md") {
        return Category::Labs;
    }

    let has_cargo   = has("Cargo.toml");
    let has_pkg     = has("package.json");
    let has_tauri   = has("src-tauri");
    // uv: uv.lock or .python-version are definitive uv markers
    let has_uv      = has("uv.lock") || has(".python-version");
    let has_py      = has_uv || has("requirements.txt") || has("pyproject.toml") || has("setup.py");
    let has_nb      = has_ext(path, "ipynb");
    let has_mem_x   = has("memory.x");           // embedded linker script
    let has_openocd = has("openocd.cfg") || has(".probe-rs");

    // ── Suffix gives a strong hint for mixed stacks ──────────────────────────
    // Bind to a variable so the temporary String lives long enough for split_suffix
    let name_lower = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase());
    if let Some((_prefix, suffix)) = name_lower.as_deref().and_then(split_suffix) {
        // suffix is already lowercase because name_lower is lowercased
        match suffix {
            "fw"                        => return Category::Embedded,
            "mob" | "mobile"            => return Category::Ui,
            "desk" | "desktop"          => return Category::Apps,
            "web" | "ui"                => return Category::Ui,
            "api" | "core" | "backend"
            | "server"                  => {
                if !has_tauri && !(has_cargo && has_pkg) {
                    return Category::Services;
                }
            }
            "frontend" | "client"
            | "landing" | "dashboard"   => return Category::Ui,
            "mono" | "desktop" | "desk" => return Category::Apps,
            _ => {}
        }
    }

    // ── Embedded: memory.x, openocd, or Cargo cross-compile target ──────────
    if has_mem_x || has_openocd || is_embedded_cargo(path) {
        return Category::Embedded;
    }

    // ── Full-stack / Tauri desktop app ───────────────────────────────────────
    if has_tauri || (has_cargo && has_pkg) {
        return Category::Apps;
    }

    // ── Python: ML pipeline or tool ─────────────────────────────────────────
    if has_py || has_nb {
        let ml_markers = [
            "train.py", "model.py", "dataset.py",
            "notebooks", "data", "models", "checkpoints",
        ];
        if ml_markers.iter().any(|m| path.join(m).exists()) {
            return Category::Ml;
        }
        return Category::Tools;
    }

    // ── Pure JS/TS: read package.json to know the truth ─────────────────────
    if has_pkg && !has_cargo {
        match read_pkg_kind(path) {
            PkgKind::Frontend  => return Category::Ui,
            PkgKind::Backend   => return Category::Services,
            PkgKind::Fullstack => return Category::Apps,
            PkgKind::Unknown   => {
                // fall back to dir-name heuristic
                if has("server") || has("backend") || has("api") {
                    return Category::Apps;
                }
                return Category::Ui;
            }
        }
    }

    // ── Pure Rust ────────────────────────────────────────────────────────────
    if has_cargo && !has_pkg {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if name.contains("cli") || name.contains("tool") || name.contains("util") {
            return Category::Tools;
        }
        return Category::Services;
    }

    Category::Labs
}

// ── package.json classifier ──────────────────────────────────────────────────

#[derive(Debug)]
enum PkgKind { Frontend, Backend, Fullstack, Unknown }

/// Known frontend deps/devDeps (frameworks, bundlers, ui libs)
const FRONTEND_DEPS: &[&str] = &[
    "react", "react-dom", "vue", "svelte", "solid-js",
    "next", "nuxt", "sveltekit", "@sveltejs/kit",
    "vite", "webpack", "parcel", "rollup", "esbuild",
    "astro", "remix", "@remix-run/react",
    "gatsby", "angular", "@angular/core",
    "tailwindcss", "@shadcn/ui", "radix-ui",
    "react-router", "react-router-dom", "wouter",
];

/// Known backend deps
const BACKEND_DEPS: &[&str] = &[
    "hono", "express", "fastify", "koa", "restify",
    "nestjs", "@nestjs/core", "@nestjs/common",
    "elysia", "h3", "nitro",
    "better-sqlite3", "pg", "mysql2", "mongoose", "prisma",
    "@prisma/client", "drizzle-orm",
    "jsonwebtoken", "passport", "bcrypt", "bcryptjs",
    "ws", "socket.io",
];

fn read_pkg_kind(dir: &Path) -> PkgKind {
    let raw = match std::fs::read_to_string(dir.join("package.json")) {
        Ok(s)  => s,
        Err(_) => return PkgKind::Unknown,
    };

    // Collect all dependency keys (deps + devDeps) without a full JSON parser
    let all_deps = extract_dep_keys(&raw);

    let has_fe = all_deps.iter().any(|d| FRONTEND_DEPS.contains(&d.as_str()));
    let has_be = all_deps.iter().any(|d| BACKEND_DEPS.contains(&d.as_str()));

    match (has_fe, has_be) {
        (true,  true)  => PkgKind::Fullstack,
        (true,  false) => PkgKind::Frontend,
        (false, true)  => PkgKind::Backend,
        (false, false) => PkgKind::Unknown,
    }
}

/// Extract the string keys from `"dependencies"` and `"devDependencies"`
/// blocks using a simple scan — no serde/json crate needed.
fn extract_dep_keys(json: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for section in ["dependencies", "devDependencies"] {
        let start = match json.find(section) {
            Some(i) => i,
            None    => continue,
        };
        // Find the opening `{` of the section
        let brace = match json[start..].find('{') {
            Some(i) => start + i + 1,
            None    => continue,
        };
        // Walk until the matching closing `}`
        let mut depth = 1usize;
        let mut pos   = brace;
        let bytes = json.as_bytes();
        while pos < bytes.len() && depth > 0 {
            match bytes[pos] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            pos += 1;
        }
        let block = &json[brace..pos.saturating_sub(1)];
        // Each key is a quoted string before a `:`
        let mut remaining = block;
        while let Some(q1) = remaining.find('"') {
            remaining = &remaining[q1 + 1..];
            let q2 = match remaining.find('"') {
                Some(i) => i,
                None    => break,
            };
            let key = &remaining[..q2];
            remaining = &remaining[q2 + 1..];
            // Only keep it if it's followed by `:` (i.e. it's a key not a value)
            let after = remaining.trim_start();
            if after.starts_with(':') {
                keys.push(key.to_string());
            }
        }
    }
    keys
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns true if Cargo.toml is present AND .cargo/config.toml names an
/// embedded target (thumbv*, riscv*, xtensa*)
fn is_embedded_cargo(path: &Path) -> bool {
    if !path.join("Cargo.toml").exists() {
        return false;
    }
    let cfg = path.join(".cargo/config.toml");
    if !cfg.exists() {
        return false;
    }
    std::fs::read_to_string(cfg)
        .map(|s| s.contains("thumbv") || s.contains("riscv") || s.contains("xtensa"))
        .unwrap_or(false)
}

/// Check whether any file directly inside `dir` has the given extension
fn has_ext(dir: &Path, ext: &str) -> bool {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .any(|e| e.path().extension().map_or(false, |x| x == ext))
        })
        .unwrap_or(false)
}
