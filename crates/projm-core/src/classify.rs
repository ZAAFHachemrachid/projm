use colored::Colorize;
use std::path::Path;

// ── Category ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Category {
    Apps,
    Services,
    Ui,
    Embedded,
    Ml,
    Tools,
    Labs,
    Content,
    Custom(String),
}

impl serde::Serialize for Category {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.dir_name())
    }
}

impl<'de> serde::Deserialize<'de> for Category {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(s.into())
    }
}

impl From<String> for Category {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "apps" => Category::Apps,
            "services" => Category::Services,
            "ui" => Category::Ui,
            "embedded" => Category::Embedded,
            "ml" => Category::Ml,
            "tools" => Category::Tools,
            "labs" => Category::Labs,
            "content" => Category::Content,
            other => Category::Custom(other.to_string()),
        }
    }
}

impl From<&str> for Category {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

impl Category {
    pub fn dir_name(&self) -> &str {
        match self {
            Self::Apps => "apps",
            Self::Services => "services",
            Self::Ui => "ui",
            Self::Embedded => "embedded",
            Self::Ml => "ml",
            Self::Tools => "tools",
            Self::Labs => "labs",
            Self::Content => "content",
            Self::Custom(s) => s,
        }
    }

    /// Fixed-width colored label for aligned display
    pub fn label(&self) -> String {
        let s = format!("{:<8}", self.dir_name());
        match self {
            Self::Apps => s.blue().bold().to_string(),
            Self::Services => s.cyan().bold().to_string(),
            Self::Ui => s.magenta().bold().to_string(),
            Self::Embedded => s.yellow().bold().to_string(),
            Self::Ml => s.green().bold().to_string(),
            Self::Tools => s.white().bold().to_string(),
            Self::Labs => s.truecolor(255, 100, 50).bold().to_string(),
            Self::Content => s.truecolor(255, 105, 180).bold().to_string(),
            Self::Custom(_) => s.truecolor(150, 150, 150).bold().to_string(),
        }
    }

    pub fn all() -> Vec<Self> {
        let cfg = crate::config::load();
        cfg.categories
            .unwrap_or_else(|| {
                vec![
                    "apps".to_string(),
                    "services".to_string(),
                    "ui".to_string(),
                    "embedded".to_string(),
                    "ml".to_string(),
                    "tools".to_string(),
                    "labs".to_string(),
                    "content".to_string(),
                ]
            })
            .into_iter()
            .map(|s| s.into())
            .collect()
    }

    pub fn coerce_to_active(self) -> Self {
        let active = Self::all();
        if active.contains(&self) {
            self
        } else {
            Self::Custom("undefined".to_string())
        }
    }
}

// ── Known project suffixes ────────────────────────────────────────────────────
// Recognised separators: `-` or `_`

pub const KNOWN_SUFFIXES: &[&str] = &[
    "api",
    "web",
    "mob",
    "mobile",
    "desk",
    "desktop",
    "mono",
    "cli",
    "fw",
    "lib",
    "core",
    "ui",
    "website",
    // common real-world names people actually use
    "backend",
    "frontend",
    "server",
    "client",
    "app",
    "apps",
    "bot",
    "worker",
    "jobs",
    "admin",
    "dashboard",
    "landing",
    "docs",
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
#[allow(dead_code)]
pub fn prefix_key(name: &str) -> Option<String> {
    split_suffix(name).map(|(prefix, _)| prefix.to_lowercase())
}

// ── Classification logic ──────────────────────────────────────────────────────

/// Why a project ended up in its category.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClassificationSource {
    /// Pinned by a `.projm.toml` marker inside the project.
    ProjectMarker,
    /// Matched a custom rule from rules.toml (1-based index).
    Rule {
        index: usize,
        description: Option<String>,
    },
    /// Decided by a built-in heuristic.
    Heuristic(&'static str),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Classification {
    pub category: Category,
    pub source: ClassificationSource,
}

impl Classification {
    fn heuristic(category: Category, label: &'static str) -> Self {
        Self {
            category,
            source: ClassificationSource::Heuristic(label),
        }
    }

    /// Human-readable reason, e.g. for `projm rules test` output.
    pub fn reason(&self) -> String {
        match &self.source {
            ClassificationSource::ProjectMarker => {
                format!("pinned by {}", crate::marker::MARKER_FILE)
            }
            ClassificationSource::Rule { index, description } => match description {
                Some(d) => format!("matched rule #{} ({})", index, d),
                None => format!("matched rule #{}", index),
            },
            ClassificationSource::Heuristic(h) => format!("no rule matched; heuristic: {}", h),
        }
    }
}

pub fn classify(path: &Path, rules: &[crate::rules::ValidatedRule]) -> Category {
    classify_explained(path, rules).category
}

pub fn classify_explained(
    path: &Path,
    rules: &[crate::rules::ValidatedRule],
) -> Classification {
    use Classification as C;

    // --- 0. Per-project marker pin (travels with the repo, beats everything) ---
    if let Some(marker) = crate::marker::read_marker(path) {
        if let Some(cat) = marker.category {
            return C {
                category: cat.into(),
                source: ClassificationSource::ProjectMarker,
            };
        }
    }

    // --- 1. Custom rules.toml (First match wins) ---
    if let Some(rule) = crate::rules::evaluate_rules(path, rules) {
        return C {
            category: rule.category.clone(),
            source: ClassificationSource::Rule {
                index: rule.index,
                description: rule.description.clone(),
            },
        };
    }

    let has = |f: &str| path.join(f).exists();

    // ── doc-lab.md is the explicit labs marker — highest priority ──────────
    if has("doc-lab.md") {
        return C::heuristic(Category::Labs, "doc-lab.md marker");
    }

    // ── Monorepos (Turborepo, pnpm workspaces, etc.) ──────────────────────────
    if is_monorepo(path) {
        return C::heuristic(Category::Apps, "monorepo");
    }

    let has_cargo = has("Cargo.toml");
    let has_pkg = has("package.json");
    let has_tauri = has("src-tauri");
    // uv: uv.lock or .python-version are definitive uv markers
    let has_uv = has("uv.lock") || has(".python-version");
    let has_py = has_uv || has("requirements.txt") || has("pyproject.toml") || has("setup.py");
    let has_nb = has_ext(path, "ipynb");
    let has_mem_x = has("memory.x"); // embedded linker script
    let has_openocd = has("openocd.cfg") || has(".probe-rs");

    // ── Suffix gives a strong hint for mixed stacks ──────────────────────────
    // Bind to a variable so the temporary String lives long enough for split_suffix
    let name_lower = path.file_name().map(|n| n.to_string_lossy().to_lowercase());
    if let Some((_prefix, suffix)) = name_lower.as_deref().and_then(split_suffix) {
        // suffix is already lowercase because name_lower is lowercased
        match suffix {
            "fw" => return C::heuristic(Category::Embedded, "suffix: fw"),
            "mob" | "mobile" => return C::heuristic(Category::Ui, "suffix: mobile"),
            "web" | "ui" => return C::heuristic(Category::Ui, "suffix: web/ui"),
            "api" | "core" | "backend" | "server" => {
                if !(has_tauri || (has_cargo && has_pkg)) {
                    return C::heuristic(Category::Services, "suffix: api/core/backend/server");
                }
            }
            "frontend" | "client" | "landing" | "dashboard" => {
                return C::heuristic(Category::Ui, "suffix: frontend/client/landing/dashboard")
            }
            "mono" | "desktop" | "desk" => {
                return C::heuristic(Category::Apps, "suffix: mono/desktop")
            }
            _ => {}
        }
    }

    // ── Embedded: memory.x, openocd, Cargo cross-compile target, or C/C++ embedded ──────────
    if has_mem_x || has_openocd || is_embedded_cargo(path) || is_c_cpp_embedded(path) {
        return C::heuristic(Category::Embedded, "embedded markers");
    }

    // ── Full-stack / Tauri desktop app ───────────────────────────────────────
    if has_tauri || (has_cargo && has_pkg) {
        return C::heuristic(Category::Apps, "stack: tauri / rust+js full-stack");
    }

    // ── Flutter / Dart ───────────────────────────────────────────────────────
    if has("pubspec.yaml") {
        if path.join("android").exists() || path.join("ios").exists() {
            return C::heuristic(Category::Apps, "stack: flutter (mobile)");
        }
        return C::heuristic(Category::Ui, "stack: flutter");
    }

    // ── Kotlin / Android & Spring Boot ───────────────────────────────────────
    if has("build.gradle") || has("build.gradle.kts") {
        if has_android_manifest(path) {
            return C::heuristic(Category::Apps, "stack: android");
        }
        return C::heuristic(Category::Services, "stack: gradle");
    }

    // ── Java / Maven ─────────────────────────────────────────────────────────
    if has("pom.xml") {
        return C::heuristic(Category::Services, "stack: maven");
    }

    // ── Swift / iOS / macOS ──────────────────────────────────────────────────
    if has_ext(path, "xcodeproj") || has("Package.swift") {
        return C::heuristic(Category::Apps, "stack: swift");
    }

    // ── Go ───────────────────────────────────────────────────────────────────
    if has("go.mod") {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if name.contains("cli") || name.contains("tool") || name.contains("util") {
            return C::heuristic(Category::Tools, "stack: go + tool-like name");
        }
        return C::heuristic(Category::Services, "stack: go");
    }

    // ── Ruby on Rails ────────────────────────────────────────────────────────
    if has("Gemfile") && path.join("config/routes.rb").exists() {
        return C::heuristic(Category::Services, "stack: rails");
    }

    // ── Laravel / PHP ────────────────────────────────────────────────────────
    if has("composer.json") && has("artisan") {
        return C::heuristic(Category::Services, "stack: laravel");
    }

    // ── Elixir / Phoenix ─────────────────────────────────────────────────────
    if has("mix.exs") {
        return C::heuristic(Category::Services, "stack: elixir");
    }

    // ── C# / .NET ────────────────────────────────────────────────────────────
    if has_ext(path, "csproj") || has_ext(path, "sln") {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if name.contains("app")
            || name.contains("ui")
            || name.contains("desktop")
            || name.contains("mobile")
        {
            return C::heuristic(Category::Apps, "stack: dotnet + app-like name");
        }
        return C::heuristic(Category::Services, "stack: dotnet");
    }

    // ── C / C++ Native (Non-embedded) ────────────────────────────────────────
    if has("CMakeLists.txt") {
        return C::heuristic(Category::Tools, "stack: cpp");
    }

    // ── Python: ML pipeline or tool ─────────────────────────────────────────
    if has_py || has_nb {
        let ml_markers = [
            "train.py",
            "model.py",
            "dataset.py",
            "notebooks",
            "data",
            "models",
            "checkpoints",
        ];
        if ml_markers.iter().any(|m| path.join(m).exists()) {
            return C::heuristic(Category::Ml, "stack: python + ml markers");
        }
        return C::heuristic(Category::Tools, "stack: python");
    }

    // ── Pure JS/TS: read package.json to know the truth ─────────────────────
    if has_pkg && !has_cargo {
        match read_pkg_kind(path) {
            PkgKind::Frontend => return C::heuristic(Category::Ui, "package.json: frontend deps"),
            PkgKind::Backend => {
                return C::heuristic(Category::Services, "package.json: backend deps")
            }
            PkgKind::Fullstack => {
                return C::heuristic(Category::Apps, "package.json: fullstack deps")
            }
            PkgKind::Unknown => {
                // fall back to dir-name heuristic
                if has("server") || has("backend") || has("api") {
                    return C::heuristic(Category::Apps, "js + server/backend/api dir");
                }
                return C::heuristic(Category::Ui, "stack: js");
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
            return C::heuristic(Category::Tools, "stack: rust + tool-like name");
        }
        return C::heuristic(Category::Services, "stack: rust");
    }

    C::heuristic(Category::Labs, "default: labs")
}

// ── package.json classifier ──────────────────────────────────────────────────

#[derive(Debug)]
enum PkgKind {
    Frontend,
    Backend,
    Fullstack,
    Unknown,
}

/// Known frontend deps/devDeps (frameworks, bundlers, ui libs)
const FRONTEND_DEPS: &[&str] = &[
    "react",
    "react-dom",
    "vue",
    "svelte",
    "solid-js",
    "next",
    "nuxt",
    "sveltekit",
    "@sveltejs/kit",
    "vite",
    "webpack",
    "parcel",
    "rollup",
    "esbuild",
    "astro",
    "remix",
    "@remix-run/react",
    "gatsby",
    "angular",
    "@angular/core",
    "tailwindcss",
    "@shadcn/ui",
    "radix-ui",
    "react-router",
    "react-router-dom",
    "wouter",
];

/// Known backend deps
const BACKEND_DEPS: &[&str] = &[
    "hono",
    "express",
    "fastify",
    "koa",
    "restify",
    "nestjs",
    "@nestjs/core",
    "@nestjs/common",
    "elysia",
    "h3",
    "nitro",
    "better-sqlite3",
    "pg",
    "mysql2",
    "mongoose",
    "prisma",
    "@prisma/client",
    "drizzle-orm",
    "jsonwebtoken",
    "passport",
    "bcrypt",
    "bcryptjs",
    "ws",
    "socket.io",
];

fn read_pkg_kind(dir: &Path) -> PkgKind {
    let raw = match std::fs::read_to_string(dir.join("package.json")) {
        Ok(s) => s,
        Err(_) => return PkgKind::Unknown,
    };

    // Collect all dependency keys (deps + devDeps) without a full JSON parser
    let all_deps = extract_dep_keys(&raw);

    let has_fe = all_deps.iter().any(|d| FRONTEND_DEPS.contains(&d.as_str()));
    let has_be = all_deps.iter().any(|d| BACKEND_DEPS.contains(&d.as_str()));

    match (has_fe, has_be) {
        (true, true) => PkgKind::Fullstack,
        (true, false) => PkgKind::Frontend,
        (false, true) => PkgKind::Backend,
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
            None => continue,
        };
        // Find the opening `{` of the section
        let brace = match json[start..].find('{') {
            Some(i) => start + i + 1,
            None => continue,
        };
        // Walk until the matching closing `}`
        let mut depth = 1usize;
        let mut pos = brace;
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
                None => break,
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

fn is_monorepo(path: &Path) -> bool {
    if path.join("turbo.json").exists()
        || path.join("pnpm-workspace.yaml").exists()
        || path.join("lerna.json").exists()
        || path.join("nx.json").exists()
    {
        return true;
    }

    let pkg_json_path = path.join("package.json");
    if pkg_json_path.exists() {
        if let Ok(content) = std::fs::read_to_string(pkg_json_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if val.get("workspaces").is_some() {
                    return true;
                }
            }
        }
    }

    false
}

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
                .any(|e| e.path().extension().is_some_and(|x| x == ext))
        })
        .unwrap_or(false)
}

pub fn extract_dep_keys_helper(path: &Path) -> Vec<String> {
    if let Ok(raw) = std::fs::read_to_string(path.join("package.json")) {
        extract_dep_keys(&raw)
    } else {
        Vec::new()
    }
}

fn is_c_cpp_embedded(path: &Path) -> bool {
    if !path.join("CMakeLists.txt").exists() {
        return false;
    }
    has_ext(path, "ld") || path.join("openocd.cfg").exists() || path.join("openocd").exists()
}

fn has_android_manifest(path: &Path) -> bool {
    if path.join("src/main/AndroidManifest.xml").exists()
        || path.join("app/src/main/AndroidManifest.xml").exists()
        || path.join("AndroidManifest.xml").exists()
    {
        return true;
    }

    // Depth-limited search (depth <= 4)
    fn find_manifest(dir: &Path, depth: usize) -> bool {
        if depth > 4 {
            return false;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                        if name == "target"
                            || name == "node_modules"
                            || name == ".git"
                            || name == "build"
                        {
                            continue;
                        }
                    }
                    if find_manifest(&p, depth + 1) {
                        return true;
                    }
                } else if p.file_name().is_some_and(|n| n == "AndroidManifest.xml") {
                    return true;
                }
            }
        }
        false
    }
    find_manifest(path, 1)
}
