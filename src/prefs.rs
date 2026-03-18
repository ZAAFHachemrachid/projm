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
    pub fn load() -> Result<Self> {
        let p = path()?;
        if !p.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", p.display()))
    }

    pub fn save(&self) -> Result<()> {
        let p = path()?;
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&p, serde_json::to_string_pretty(self)?)
            .with_context(|| format!("writing {}", p.display()))
    }

    /// Last editor binary used for this project, if any.
    pub fn last_editor_for(&self, project: &Path) -> Option<&str> {
        self.last_editor.get(&key(project)).map(String::as_str)
    }

    /// Persist the choice.
    pub fn set_last_editor(&mut self, project: &Path, binary: &str) -> Result<()> {
        self.last_editor.insert(key(project), binary.to_owned());
        self.save()
    }
}

fn key(project: &Path) -> String {
    project
        .canonicalize()
        .unwrap_or_else(|_| project.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

fn path() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .context("cannot resolve XDG config dir")?
        .join("projm/prefs.json"))
}

