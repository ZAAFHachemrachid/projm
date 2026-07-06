// ── AI Agents: configurable coding-agent CLIs launchable inside a project shell ──

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Built-in agent presets seeded into a fresh agents.json: (name, command).
/// The first whitespace-separated token of `command` is the binary probed on $PATH.
pub const DEFAULT_AGENTS: &[(&str, &str)] = &[
    ("Claude Code", "claude"),
    ("Codex", "codex"),
    ("Gemini CLI", "gemini"),
    ("OpenCode", "opencode"),
    ("Aider", "aider"),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Agent {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentsStore {
    pub agents: Vec<Agent>,
}

impl Default for AgentsStore {
    fn default() -> Self {
        Self {
            agents: DEFAULT_AGENTS
                .iter()
                .map(|(name, command)| Agent {
                    name: name.to_string(),
                    command: command.to_string(),
                })
                .collect(),
        }
    }
}

impl AgentsStore {
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
        .join("projm/agents.json"))
}

/// The binary an agent command resolves to: its first whitespace-separated token.
pub fn agent_binary(command: &str) -> &str {
    command.split_whitespace().next().unwrap_or("")
}

/// Probe $PATH for the agent's binary; None when not installed.
pub fn detect_path(command: &str) -> Option<PathBuf> {
    let binary = agent_binary(command);
    if binary.is_empty() {
        return None;
    }
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(binary);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            for ext in ["exe", "bat", "cmd"] {
                let with_ext = candidate.with_extension(ext);
                if with_ext.is_file() {
                    return Some(with_ext);
                }
            }
        }
    }
    None
}
