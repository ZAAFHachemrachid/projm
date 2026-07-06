use crate::classify::{split_suffix, Category};
use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct CustomRule {
    pub name: Option<String>,
    pub marker: Option<String>,
    pub name_contains: Option<String>,
    pub suffix: Option<String>,
    pub has_dep: Option<String>,
    pub category: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct RulesConfig {
    #[serde(rename = "rule", default)]
    pub rules: Vec<CustomRule>,
}

#[derive(Debug, Clone)]
pub struct ValidatedRule {
    pub name: Option<String>,
    pub marker: Option<String>,
    pub name_contains: Option<String>,
    pub suffix: Option<String>,
    pub has_dep: Option<String>,
    pub category: Category,
}

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

pub fn save_rules_raw(content: &str) -> Result<(), String> {
    let _parsed: RulesConfig =
        toml::from_str(content).map_err(|e| format!("Invalid TOML/Rules syntax: {}", e))?;
    let path = rules_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

fn parse_category(s: &str) -> Option<Category> {
    Some(s.to_string().into())
}

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

    let raw_config: RulesConfig = match toml::from_str(&contents) {
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
    for (i, rule) in raw_config.rules.into_iter().enumerate() {
        if let Some(cat) = parse_category(&rule.category) {
            validated.push(ValidatedRule {
                name: rule.name,
                marker: rule.marker,
                name_contains: rule.name_contains,
                suffix: rule.suffix,
                has_dep: rule.has_dep,
                category: cat,
            });
        } else {
            eprintln!(
                "  {} Ignoring custom rule #{} in rules.toml: unknown category '{}'",
                "warning:".yellow().bold(),
                i + 1,
                rule.category
            );
        }
    }

    validated
}

pub fn init_default_rules() -> Result<()> {
    let path = rules_path();
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let template = r#"# ==============================================================================
# Projm Custom Classification Rules Configuration (rules.toml)
# ==============================================================================
#
# Rules are evaluated from top to bottom. The first matching rule wins.
# Within a single [[rule]], all specified criteria must match (AND logic).
#
# Supported fields:
# - name          : Exact name match of the project directory (e.g. "pioneers-website")
# - name_contains : Substring match of the project directory name (e.g. "adrar")
# - marker        : File/directory presence marker in the project root (e.g. "rocket.toml")
# - suffix        : Override built-in suffix behaviour (e.g. "fw")
# - has_dep       : Check dependencies in Cargo.toml, package.json, or requirements.txt/pyproject.toml (e.g. "burn")
#
# Categories must be one of: "apps", "services", "ui", "embedded", "ml", "tools", "labs", "content"
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
# name_contains = "adrar"
# category      = "labs"
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
# has_dep  = "tensorflow"
# category = "ml"
#
# [[rule]]
# has_dep  = "remotion"
# category = "content"
"#;

    std::fs::write(&path, template)?;
    Ok(())
}

impl ValidatedRule {
    pub fn matches(&self, path: &Path) -> bool {
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

        if let Some(ref r_suffix) = self.suffix {
            if let Some((_, suf)) = split_suffix(&name) {
                if suf.to_lowercase() != r_suffix.to_lowercase() {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref r_marker) = self.marker {
            if !path.join(r_marker).exists() {
                return false;
            }
        }

        if let Some(ref r_dep) = self.has_dep {
            if !self.check_dep(path, r_dep) {
                return false;
            }
        }

        true
    }

    fn check_dep(&self, path: &Path, dep: &str) -> bool {
        // 1. Rust / Cargo.toml
        if path.join("Cargo.toml").exists() {
            if let Ok(content) = std::fs::read_to_string(path.join("Cargo.toml")) {
                if let Ok(cargo_toml) = toml::from_str::<toml::Value>(&content) {
                    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                        if let Some(deps) = cargo_toml.get(section).and_then(|v| v.as_table()) {
                            if deps.contains_key(dep) {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // 2. Node.js / package.json
        if path.join("package.json").exists() {
            let keys = crate::classify::extract_dep_keys_helper(path);
            if keys.iter().any(|k| k == dep) {
                return true;
            }
        }

        // 3. Python / requirements.txt & pyproject.toml
        if path.join("requirements.txt").exists() {
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
                    if pkg_name.to_lowercase() == dep.to_lowercase() {
                        return true;
                    }
                }
            }
        }

        if path.join("pyproject.toml").exists() {
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
                                if pkg_name.to_lowercase() == dep.to_lowercase() {
                                    return true;
                                }
                            }
                        }
                    }
                    if let Some(deps) = pyproject
                        .get("tool")
                        .and_then(|t| t.get("poetry"))
                        .and_then(|p| p.get("dependencies"))
                        .and_then(|d| d.as_table())
                    {
                        if deps.contains_key(dep) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }
}
