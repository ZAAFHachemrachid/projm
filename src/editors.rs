use std::path::PathBuf;

pub const KNOWN_EDITORS: &[(&str, &str)] = &[
    ("nvim", "Neovim"),
    ("zed", "Zed"),
    ("code", "VS Code"),
    ("kiro", "Kiro"),
    ("hx", "Helix"),
    ("idea", "IntelliJ"),
    ("cursor", "Cursor"),
    ("emacs", "Emacs"),
    ("vim", "Vim"),
];

#[derive(Debug, Clone)]
pub struct InstalledEditor {
    pub binary: &'static str,
    pub name: &'static str,
    pub path: PathBuf,
}

/// Walk `KNOWN_EDITORS` and return only entries whose binary exists on `$PATH`.
pub fn detect_installed() -> Vec<InstalledEditor> {
    KNOWN_EDITORS
        .iter()
        .filter_map(|(binary, name)| {
            which(binary).map(|path| InstalledEditor { binary, name, path })
        })
        .collect()
}

/// Minimal `which` — no extra crate needed.
fn which(binary: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    std::env::split_paths(&path_var)
        .map(|dir| dir.join(binary))
        .find(|p| p.is_file())
}
