use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub base: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("projects"),
        }
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("projm/config.json")
}

pub fn load() -> Config {
    let p = config_path();
    if !p.exists() {
        return Config::default();
    }
    fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn set_base(path: &PathBuf) -> Result<()> {
    let p = config_path();
    fs::create_dir_all(p.parent().expect("config path has parent"))?;
    let cfg = Config { base: path.clone() };
    fs::write(&p, serde_json::to_string_pretty(&cfg)?)?;
    eprintln!("  projm base → {}", path.display());
    Ok(())
}
