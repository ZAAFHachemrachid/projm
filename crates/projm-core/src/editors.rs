use std::path::PathBuf;

pub const KNOWN_EDITORS: &[(&str, &str)] = &[
    ("antigravity-ide", "Antigravity"),
    ("nvim", "Neovim"),
    ("zed", "Zed"),
    ("zeditor", "Zed"),
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
/// On Windows, also probes `.exe`, `.bat`, `.cmd` extensions.
fn which(binary: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(binary);
        if candidate.is_file() {
            return Some(candidate);
        }
        // On Windows, the binary may have an extension
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
