use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

/// `~/.config/projm/prefs.json`
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Prefs {
    /// canonical project path → last-used editor binary
    #[serde(default)]
    pub last_editor: HashMap<String, String>,
}

impl Prefs {
    // ── Production API (uses XDG path) ────────────────────────────────────────

    pub fn load() -> Result<Self> {
        Ok(Self::load_from(default_path()?))
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&default_path()?)
    }

    pub fn last_editor_for(&self, project: &Path) -> Option<&str> {
        self.last_editor.get(&key(project)).map(String::as_str)
    }

    pub fn set_last_editor(&mut self, project: &Path, binary: &str) -> Result<()> {
        self.set_last_editor_at(&default_path()?, project, binary)
    }

    // ── Testable variants (explicit path) ─────────────────────────────────────

    pub fn load_from(path: PathBuf) -> Self {
        if !path.exists() {
            return Self::default();
        }
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)
            .with_context(|| format!("writing {}", path.display()))
    }

    pub fn set_last_editor_at(
        &mut self,
        path: &PathBuf,
        project: &Path,
        binary: &str,
    ) -> Result<()> {
        self.last_editor.insert(key(project), binary.to_owned());
        self.save_to(path)
    }
}

fn key(project: &Path) -> String {
    project
        .canonicalize()
        .unwrap_or_else(|_| project.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

fn default_path() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .context("cannot resolve XDG config dir")?
        .join("projm/prefs.json"))
}

