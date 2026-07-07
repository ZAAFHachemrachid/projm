use crate::classify::{split_suffix, Category};
use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::cell::OnceCell;
use std::path::{Path, PathBuf};

fn default_true() -> bool {
    true
}

fn is_true(b: &bool) -> bool {
    *b
}

/// One set of match criteria. All specified fields must match (AND logic).
/// Used both at the top level of a rule and inside `any_of` / `none_of`.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct RawMatcher {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_glob: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_regex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_glob: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_marker: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_dep: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_deps: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dep_version: Option<RawDepVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

impl RawMatcher {
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.name_contains.is_none()
            && self.name_glob.is_none()
            && self.name_regex.is_none()
            && self.suffix.is_none()
            && self.parent_dir.is_none()
            && self.path_glob.is_none()
            && self.marker.is_none()
            && self.markers.is_none()
            && self.any_marker.is_none()
            && self.has_dep.is_none()
            && self.has_deps.is_none()
            && self.dep_version.is_none()
            && self.stack.is_none()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct RawDepVersion {
    pub name: String,
    pub req: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct CustomRule {
    #[serde(flatten)]
    pub matcher: RawMatcher,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<RawMatcher>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_of: Option<Vec<RawMatcher>>,
    pub category: String,
}

impl Default for CustomRule {
    fn default() -> Self {
        Self {
            matcher: RawMatcher::default(),
            description: None,
            enabled: true,
            priority: None,
            any_of: None,
            none_of: None,
            category: String::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct RulesConfig {
    #[serde(rename = "rule", default)]
    pub rules: Vec<CustomRule>,
}

// ── Compiled layer ────────────────────────────────────────────────────────────

/// A matcher with globs/regexes/version requirements compiled once at load.
#[derive(Debug, Clone, Default)]
pub struct CompiledMatcher {
    pub name: Option<String>,
    pub name_contains: Option<String>,
    pub name_glob: Option<globset::GlobMatcher>,
    pub name_regex: Option<regex::Regex>,
    pub suffix: Option<String>,
    pub parent_dir: Option<String>,
    pub path_glob: Option<globset::GlobMatcher>,
    /// `marker` and `markers` merged — ALL must exist.
    pub markers: Vec<String>,
    /// At least one must exist (when non-empty).
    pub any_marker: Vec<String>,
    /// `has_dep` and `has_deps` merged — ALL must be present.
    pub has_deps: Vec<String>,
    pub dep_version: Option<(String, semver::VersionReq)>,
    pub stack: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ValidatedRule {
    /// 1-based position in rules.toml, for explain/test output.
    pub index: usize,
    pub description: Option<String>,
    pub priority: Option<i64>,
    pub matcher: CompiledMatcher,
    pub any_of: Vec<CompiledMatcher>,
    pub none_of: Vec<CompiledMatcher>,
    pub category: Category,
}

/// Per-path caches shared across rule evaluations for one project.
#[derive(Default)]
pub struct MatchContext {
    stack: OnceCell<&'static str>,
    deps: OnceCell<Vec<(String, Option<String>)>>,
}

impl MatchContext {
    pub fn new() -> Self {
        Self::default()
    }

    fn stack_id(&self, path: &Path) -> &'static str {
        self.stack
            .get_or_init(|| crate::run::detect_stack(path).id())
    }

    fn deps(&self, path: &Path) -> &[(String, Option<String>)] {
        self.deps.get_or_init(|| read_all_deps(path))
    }
}

/// Outcome of testing one rule against one path (dry-run/test UX).
#[derive(Debug, Clone, Serialize)]
pub struct RuleEvaluation {
    pub index: usize,
    pub description: Option<String>,
    pub category: String,
    pub matched: bool,
}

// ── File location & raw IO ────────────────────────────────────────────────────

pub fn rules_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("projm/rules.toml")
}

pub fn read_rules_raw() -> Result<String, String> {
    let path = rules_path();
    if !path.exists() {
        let _ = init_default_rules();
    }
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// Strict syntax check for a rules file, for editing surfaces.
pub fn validate_rules_content(content: &str) -> Result<(), String> {
    toml::from_str::<RulesConfig>(content)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn save_rules_raw(content: &str) -> Result<(), String> {
    let _parsed: RulesConfig =
        toml::from_str(content).map_err(|e| format!("Invalid TOML/Rules syntax: {}", e))?;
    save_rules_raw_at(&rules_path(), content)
}

pub(crate) fn save_rules_raw_at(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

// ── Loading & validation ──────────────────────────────────────────────────────

pub fn load_rules() -> Vec<ValidatedRule> {
    let path = rules_path();
    if !path.exists() {
        return Vec::new();
    }

    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "  {} Failed to read rules.toml: {}",
                "warning:".yellow().bold(),
                e
            );
            return Vec::new();
        }
    };

    parse_and_validate(&contents)
}

fn warn_rule(index: usize, msg: &str) {
    eprintln!(
        "  {} Ignoring custom rule #{} in rules.toml: {}",
        "warning:".yellow().bold(),
        index,
        msg
    );
}

fn compile_matcher(raw: &RawMatcher) -> Result<CompiledMatcher, String> {
    let name_glob = match &raw.name_glob {
        Some(g) => Some(
            globset::Glob::new(g)
                .map_err(|e| format!("invalid name_glob '{}': {}", g, e))?
                .compile_matcher(),
        ),
        None => None,
    };
    let path_glob = match &raw.path_glob {
        Some(g) => Some(
            globset::Glob::new(g)
                .map_err(|e| format!("invalid path_glob '{}': {}", g, e))?
                .compile_matcher(),
        ),
        None => None,
    };
    let name_regex = match &raw.name_regex {
        Some(r) => Some(
            regex::Regex::new(r).map_err(|e| format!("invalid name_regex '{}': {}", r, e))?,
        ),
        None => None,
    };
    let dep_version = match &raw.dep_version {
        Some(dv) => Some((
            dv.name.clone(),
            semver::VersionReq::parse(&dv.req)
                .map_err(|e| format!("invalid dep_version req '{}': {}", dv.req, e))?,
        )),
        None => None,
    };
    let stack = match &raw.stack {
        Some(s) => {
            let lower = s.to_lowercase();
            match crate::run::KNOWN_STACK_IDS.iter().find(|id| **id == lower) {
                Some(id) => Some(*id),
                None => {
                    return Err(format!(
                        "unknown stack '{}' (expected one of: {})",
                        s,
                        crate::run::KNOWN_STACK_IDS.join(", ")
                    ))
                }
            }
        }
        None => None,
    };

    let mut markers: Vec<String> = raw.marker.iter().cloned().collect();
    markers.extend(raw.markers.iter().flatten().cloned());
    let any_marker: Vec<String> = raw.any_marker.iter().flatten().cloned().collect();
    let mut has_deps: Vec<String> = raw.has_dep.iter().cloned().collect();
    has_deps.extend(raw.has_deps.iter().flatten().cloned());

    Ok(CompiledMatcher {
        name: raw.name.clone(),
        name_contains: raw.name_contains.clone(),
        name_glob,
        name_regex,
        suffix: raw.suffix.clone(),
        parent_dir: raw.parent_dir.clone(),
        path_glob,
        markers,
        any_marker,
        has_deps,
        dep_version,
        stack,
    })
}

/// Check a single rule for structural problems (bad glob/regex/semver/stack,
/// no matcher at all, empty any_of/none_of tables). Used by editing surfaces
/// to reject invalid rules up front instead of warn-skipping at load time.
pub fn validate_rule(rule: &CustomRule) -> Result<(), String> {
    if rule.category.trim().is_empty() {
        return Err("category is required".to_string());
    }
    let has_any_of = rule.any_of.as_ref().is_some_and(|v| !v.is_empty());
    if rule.matcher.is_empty() && !has_any_of {
        return Err("at least one matcher field is required".to_string());
    }
    compile_matcher(&rule.matcher)?;
    for m in rule.any_of.iter().flatten() {
        if m.is_empty() {
            return Err("empty matcher table in any_of".to_string());
        }
        compile_matcher(m).map_err(|e| format!("in any_of: {}", e))?;
    }
    for m in rule.none_of.iter().flatten() {
        if m.is_empty() {
            return Err("empty matcher table in none_of".to_string());
        }
        compile_matcher(m).map_err(|e| format!("in none_of: {}", e))?;
    }
    Ok(())
}

/// Parse rules TOML and compile into validated rules.
/// Invalid rules are warned about and skipped; `enabled = false` rules are
/// dropped; rules are stable-sorted by `priority` (lower first, unprioritized
/// rules keep file order after all prioritized ones).
pub fn parse_and_validate(contents: &str) -> Vec<ValidatedRule> {
    let raw_config: RulesConfig = match toml::from_str(contents) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!(
                "  {} Failed to parse rules.toml: {}",
                "warning:".yellow().bold(),
                e
            );
            return Vec::new();
        }
    };

    let mut validated = Vec::new();
    'rules: for (i, rule) in raw_config.rules.into_iter().enumerate() {
        let index = i + 1;
        if !rule.enabled {
            continue;
        }

        let matcher = match compile_matcher(&rule.matcher) {
            Ok(m) => m,
            Err(e) => {
                warn_rule(index, &e);
                continue;
            }
        };

        let mut any_of = Vec::new();
        for m in rule.any_of.iter().flatten() {
            if m.is_empty() {
                warn_rule(index, "empty matcher table in any_of (it would match everything)");
                continue 'rules;
            }
            match compile_matcher(m) {
                Ok(c) => any_of.push(c),
                Err(e) => {
                    warn_rule(index, &format!("in any_of: {}", e));
                    continue 'rules;
                }
            }
        }

        let mut none_of = Vec::new();
        for m in rule.none_of.iter().flatten() {
            if m.is_empty() {
                warn_rule(index, "empty matcher table in none_of (it would match everything)");
                continue 'rules;
            }
            match compile_matcher(m) {
                Ok(c) => none_of.push(c),
                Err(e) => {
                    warn_rule(index, &format!("in none_of: {}", e));
                    continue 'rules;
                }
            }
        }

        validated.push(ValidatedRule {
            index,
            description: rule.description,
            priority: rule.priority,
            matcher,
            any_of,
            none_of,
            category: rule.category.into(),
        });
    }

    // Stable sort: explicit priority (lower first) before file order.
    validated.sort_by_key(|r| r.priority.unwrap_or(i64::MAX));
    validated
}

// ── Evaluation ────────────────────────────────────────────────────────────────

/// Returns the first matching rule, if any.
pub fn evaluate_rules<'a>(path: &Path, rules: &'a [ValidatedRule]) -> Option<&'a ValidatedRule> {
    let ctx = MatchContext::new();
    rules.iter().find(|r| r.matches_with(path, &ctx))
}

/// Evaluate every rule against `path`, reporting per-rule match state.
pub fn evaluate_rules_verbose(path: &Path, rules: &[ValidatedRule]) -> Vec<RuleEvaluation> {
    let ctx = MatchContext::new();
    rules
        .iter()
        .map(|r| RuleEvaluation {
            index: r.index,
            description: r.description.clone(),
            category: r.category.dir_name().to_string(),
            matched: r.matches_with(path, &ctx),
        })
        .collect()
}

impl ValidatedRule {
    pub fn matches(&self, path: &Path) -> bool {
        self.matches_with(path, &MatchContext::new())
    }

    pub fn matches_with(&self, path: &Path, ctx: &MatchContext) -> bool {
        if !self.matcher.matches(path, ctx) {
            return false;
        }
        if !self.any_of.is_empty() && !self.any_of.iter().any(|m| m.matches(path, ctx)) {
            return false;
        }
        if self.none_of.iter().any(|m| m.matches(path, ctx)) {
            return false;
        }
        true
    }
}

impl CompiledMatcher {
    fn matches(&self, path: &Path, ctx: &MatchContext) -> bool {
        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => return false,
        };

        if let Some(ref r_name) = self.name {
            if name != *r_name {
                return false;
            }
        }

        if let Some(ref r_contains) = self.name_contains {
            if !name.contains(r_contains) {
                return false;
            }
        }

        if let Some(ref g) = self.name_glob {
            if !g.is_match(&name) {
                return false;
            }
        }

        if let Some(ref r) = self.name_regex {
            if !r.is_match(&name) {
                return false;
            }
        }

        if let Some(ref r_suffix) = self.suffix {
            if let Some((_, suf)) = split_suffix(&name) {
                if suf.to_lowercase() != r_suffix.to_lowercase() {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref r_parent) = self.parent_dir {
            let parent_name = path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string());
            if parent_name.as_deref() != Some(r_parent.as_str()) {
                return false;
            }
        }

        if let Some(ref g) = self.path_glob {
            if !g.is_match(path.to_string_lossy().as_ref()) {
                return false;
            }
        }

        if !self.markers.iter().all(|m| path.join(m).exists()) {
            return false;
        }

        if !self.any_marker.is_empty() && !self.any_marker.iter().any(|m| path.join(m).exists()) {
            return false;
        }

        if !self.has_deps.is_empty() {
            let deps = ctx.deps(path);
            for wanted in &self.has_deps {
                if !deps.iter().any(|(n, _)| n.eq_ignore_ascii_case(wanted)) {
                    return false;
                }
            }
        }

        if let Some((ref dep_name, ref req)) = self.dep_version {
            let deps = ctx.deps(path);
            let found = deps
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(dep_name))
                .and_then(|(_, v)| v.as_deref())
                .and_then(extract_base_version);
            match found {
                Some(v) if req.matches(&v) => {}
                _ => return false,
            }
        }

        if let Some(stack) = self.stack {
            if ctx.stack_id(path) != stack {
                return false;
            }
        }

        true
    }
}

// ── Dependency reading ────────────────────────────────────────────────────────

/// Collect `(name, raw_version)` pairs from every recognized manifest in `path`:
/// Cargo.toml, package.json, requirements.txt, pyproject.toml.
pub fn read_all_deps(path: &Path) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();

    // 1. Rust / Cargo.toml
    if let Ok(content) = std::fs::read_to_string(path.join("Cargo.toml")) {
        if let Ok(cargo_toml) = toml::from_str::<toml::Value>(&content) {
            for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(deps) = cargo_toml.get(section).and_then(|v| v.as_table()) {
                    for (name, val) in deps {
                        let version = match val {
                            toml::Value::String(s) => Some(s.clone()),
                            toml::Value::Table(t) => t
                                .get("version")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            _ => None,
                        };
                        out.push((name.clone(), version));
                    }
                }
            }
        }
    }

    // 2. Node.js / package.json
    if let Ok(content) = std::fs::read_to_string(path.join("package.json")) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            for section in ["dependencies", "devDependencies"] {
                if let Some(deps) = json.get(section).and_then(|v| v.as_object()) {
                    for (name, val) in deps {
                        out.push((name.clone(), val.as_str().map(|s| s.to_string())));
                    }
                }
            }
        }
    }

    // 3. Python / requirements.txt
    if let Ok(content) = std::fs::read_to_string(path.join("requirements.txt")) {
        for line in content.lines() {
            let cleaned = line.trim();
            if cleaned.is_empty() || cleaned.starts_with('#') {
                continue;
            }
            let pkg_name = cleaned
                .split(&['=', '<', '>', '~', ';', '['][..])
                .next()
                .unwrap_or(cleaned)
                .trim();
            if pkg_name.is_empty() {
                continue;
            }
            let version = cleaned
                .strip_prefix(pkg_name)
                .map(|rest| rest.trim().to_string())
                .filter(|v| !v.is_empty());
            out.push((pkg_name.to_string(), version));
        }
    }

    // 4. Python / pyproject.toml
    if let Ok(content) = std::fs::read_to_string(path.join("pyproject.toml")) {
        if let Ok(pyproject) = toml::from_str::<toml::Value>(&content) {
            if let Some(deps) = pyproject
                .get("project")
                .and_then(|p| p.get("dependencies"))
                .and_then(|d| d.as_array())
            {
                for dep_val in deps {
                    if let Some(dep_str) = dep_val.as_str() {
                        let pkg_name = dep_str
                            .split(&['=', '<', '>', '~', ';', '['][..])
                            .next()
                            .unwrap_or(dep_str)
                            .trim();
                        if pkg_name.is_empty() {
                            continue;
                        }
                        let version = dep_str
                            .strip_prefix(pkg_name)
                            .map(|rest| rest.trim().to_string())
                            .filter(|v| !v.is_empty());
                        out.push((pkg_name.to_string(), version));
                    }
                }
            }
            if let Some(deps) = pyproject
                .get("tool")
                .and_then(|t| t.get("poetry"))
                .and_then(|p| p.get("dependencies"))
                .and_then(|d| d.as_table())
            {
                for (name, val) in deps {
                    let version = match val {
                        toml::Value::String(s) => Some(s.clone()),
                        toml::Value::Table(t) => t
                            .get("version")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        _ => None,
                    };
                    out.push((name.clone(), version));
                }
            }
        }
    }

    out
}

/// Best-effort: extract the declared minimum version from a manifest version
/// string. `"^18.2.0"` → 18.2.0, `">=2.0"` → 2.0.0. Returns None for things
/// that don't declare a concrete base version (`workspace:*`, git URLs, `*`,
/// `latest`, …).
pub fn extract_base_version(raw: &str) -> Option<semver::Version> {
    // Take the first comparator of a possibly comma/`||`-separated range.
    let first = raw.split(&[',', ' '][..]).find(|s| !s.is_empty())?;
    let first = first.split("||").next()?.trim();
    let stripped = first
        .trim_start_matches(['^', '~', '=', '>', '<', 'v'])
        .trim();
    if stripped.is_empty() {
        return None;
    }
    // Reject obvious non-versions early.
    if !stripped.chars().next()?.is_ascii_digit() {
        return None;
    }
    // Keep only the leading version-ish chunk (digits, dots, pre-release tags).
    let chunk: String = stripped
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
        .collect();
    // Drop wildcard segments like `1.2.x` / `1.*`.
    let numeric_parts: Vec<&str> = chunk
        .split('.')
        .take_while(|p| p.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .collect();
    if numeric_parts.is_empty() {
        return None;
    }
    let mut parts = numeric_parts;
    while parts.len() < 3 {
        parts.push("0");
    }
    semver::Version::parse(&parts[..3].join(".")).ok()
}

// ── Default template ──────────────────────────────────────────────────────────

pub fn init_default_rules() -> Result<()> {
    let path = rules_path();
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&path, DEFAULT_RULES_TEMPLATE)?;
    Ok(())
}

pub const DEFAULT_RULES_TEMPLATE: &str = r#"# ==============================================================================
# Projm Custom Classification Rules Configuration (rules.toml)
# ==============================================================================
#
# Rules are evaluated from top to bottom. The first matching rule wins.
# Within a single [[rule]], all specified criteria must match (AND logic).
# Rules with an explicit `priority` (lower = earlier) run before the others.
#
# Tip: a `.projm.toml` file inside a project dir (with `category = "..."`)
# pins that project's category and takes precedence over all rules here.
#
# Matcher fields (all optional; combine freely):
# - name          : Exact directory-name match (e.g. "pioneers-website")
# - name_contains : Substring match on the directory name (e.g. "adrar")
# - name_glob     : Glob on the directory name (e.g. "*-api")
# - name_regex    : Regex on the directory name (e.g. "^svc-[0-9]+$")
# - suffix        : Recognised project suffix (e.g. "fw")
# - parent_dir    : Immediate parent directory name (e.g. "clients")
# - path_glob     : Glob on the full project path (e.g. "**/experiments/**")
# - marker        : A file/dir that must exist in the project root (e.g. "rocket.toml")
# - markers       : ALL of these files/dirs must exist (e.g. ["Dockerfile", "fly.toml"])
# - any_marker    : At least ONE of these must exist (e.g. ["justfile", "Makefile"])
# - has_dep       : Dependency in Cargo.toml / package.json / requirements.txt / pyproject.toml
# - has_deps      : ALL of these dependencies must be present (e.g. ["react", "vite"])
# - dep_version   : Semver check on a dependency, e.g. { name = "react", req = ">=18" }
# - stack         : Detected stack: rust, js, tauri, flutter, go, python, rails,
#                   elixir, gradle, maven, laravel, cpp, dotnet
#
# Rule options:
# - description   : Free-text note shown in listings and explain output
# - enabled       : Set to false to keep a rule but skip it (default true)
# - priority      : Lower numbers are evaluated first (unprioritized keep file order)
# - any_of        : Array of matcher tables — at least ONE table must fully match
# - none_of       : Array of matcher tables — the rule fails if ANY table matches
#
# Categories: "apps", "services", "ui", "embedded", "ml", "tools", "labs", "content"
# (plus any custom categories from your projm config)
#
# Examples:
#
# [[rule]]
# name     = "pioneers-website"
# category = "ui"
#
# [[rule]]
# marker   = "rocket.toml"
# category = "services"
#
# [[rule]]
# name_glob = "*-api"
# category  = "services"
#
# [[rule]]
# suffix   = "fw"
# category = "embedded"
#
# [[rule]]
# has_dep  = "burn"
# category = "ml"
#
# [[rule]]
# has_dep  = "remotion"
# category = "content"
#
# [[rule]]
# description = "React 18+ frontends, except legacy ones"
# dep_version = { name = "react", req = ">=18" }
# none_of     = [ { name_contains = "legacy" } ]
# category    = "ui"
#
# [[rule]]
# description = "Anything that looks like a service"
# any_of      = [ { name_glob = "*-api" }, { marker = "rocket.toml" } ]
# category    = "services"
"#;
