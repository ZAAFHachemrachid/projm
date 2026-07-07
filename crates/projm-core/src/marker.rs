use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Per-project marker file. Lives inside the project directory and travels
/// with the repo. Shares the filename with the `[run]` config — both are
/// top-level-tolerant TOML (unknown keys are ignored by each reader).
pub const MARKER_FILE: &str = ".projm.toml";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectMarker {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

/// True if the value is safe to use as a single directory-name component.
/// Marker files arrive with cloned repos, so a hostile value must not be able
/// to redirect a move outside the base directory.
fn safe_component(s: &str) -> bool {
    !s.is_empty() && s != "." && s != ".." && !s.contains(['/', '\\']) && !s.contains('\0')
}

/// Read and sanitize the marker in `project_dir`. Returns None when the file
/// is absent, unparseable, or contains no recognized keys after sanitizing.
pub fn read_marker(project_dir: &Path) -> Option<ProjectMarker> {
    let path = project_dir.join(MARKER_FILE);
    let content = std::fs::read_to_string(&path).ok()?;
    let mut marker: ProjectMarker = match toml::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            eprintln!(
                "  {} Ignoring {}: {}",
                "warning:".yellow().bold(),
                path.display(),
                e
            );
            return None;
        }
    };

    if let Some(cat) = &marker.category {
        if !safe_component(cat) {
            eprintln!(
                "  {} Ignoring unsafe category {:?} in {}",
                "warning:".yellow().bold(),
                cat,
                path.display()
            );
            marker.category = None;
        }
    }
    if let Some(group) = &marker.group {
        if !safe_component(group) {
            eprintln!(
                "  {} Ignoring unsafe group {:?} in {}",
                "warning:".yellow().bold(),
                group,
                path.display()
            );
            marker.group = None;
        }
    }

    if marker == ProjectMarker::default() {
        return None;
    }
    Some(marker)
}

/// Write (or update) the marker in `project_dir`, preserving any other
/// content in the file (comments, `[run]` section) via toml_edit.
pub fn write_marker(project_dir: &Path, marker: &ProjectMarker) -> Result<(), String> {
    if let Some(cat) = &marker.category {
        if !safe_component(cat) {
            return Err(format!("unsafe category value: {:?}", cat));
        }
    }
    if let Some(group) = &marker.group {
        if !safe_component(group) {
            return Err(format!("unsafe group value: {:?}", group));
        }
    }

    let path = project_dir.join(MARKER_FILE);
    let mut doc: toml_edit::DocumentMut = match std::fs::read_to_string(&path) {
        Ok(existing) => existing
            .parse()
            .map_err(|e| format!("existing {} is not valid TOML: {}", MARKER_FILE, e))?,
        Err(_) => toml_edit::DocumentMut::new(),
    };

    set_or_remove(&mut doc, "category", marker.category.as_deref());
    set_or_remove(&mut doc, "group", marker.group.as_deref());
    match marker.hidden {
        Some(h) => {
            doc["hidden"] = toml_edit::value(h);
        }
        None => {
            doc.remove("hidden");
        }
    }

    std::fs::write(&path, doc.to_string()).map_err(|e| e.to_string())
}

fn set_or_remove(doc: &mut toml_edit::DocumentMut, key: &str, val: Option<&str>) {
    match val {
        Some(v) => {
            doc[key] = toml_edit::value(v);
        }
        None => {
            doc.remove(key);
        }
    }
}
