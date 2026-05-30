use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub base: PathBuf,
    #[serde(default)]
    pub categories: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("projects"),
            categories: Some(vec![
                "apps".to_string(),
                "services".to_string(),
                "ui".to_string(),
                "embedded".to_string(),
                "ml".to_string(),
                "tools".to_string(),
                "labs".to_string(),
                "content".to_string(),
            ]),
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

pub fn save_config(cfg: &Config) -> Result<()> {
    let p = config_path();
    fs::create_dir_all(p.parent().expect("config path has parent"))?;
    fs::write(&p, serde_json::to_string_pretty(cfg)?)?;
    Ok(())
}

pub fn set_base(path: &PathBuf) -> Result<()> {
    let mut cfg = load();
    cfg.base = path.clone();
    save_config(&cfg)?;
    eprintln!("  projm base → {}", path.display());
    Ok(())
}

pub fn set_categories(cats: Vec<String>) -> Result<()> {
    let mut cfg = load();
    cfg.categories = Some(cats);
    save_config(&cfg)?;
    Ok(())
}
