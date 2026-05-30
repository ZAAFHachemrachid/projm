// ── Blueprint: saved project creation templates ──

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Blueprint {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintsStore {
    pub blueprints: Vec<Blueprint>,
}

impl BlueprintsStore {
    pub fn load() -> Result<Self> {
        Self::load_from(default_path()?)
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&default_path()?)
    }

    pub fn load_from(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let store = serde_json::from_str(&content).unwrap_or_default();
        Ok(store)
    }

    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

pub fn default_path() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve XDG config dir"))?
        .join("projm/blueprints.json"))
}
