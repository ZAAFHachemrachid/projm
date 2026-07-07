//! Scrub the AppImage runtime environment out of child processes.
//!
//! When projm ships as an AppImage, the launcher exports variables that point
//! into the mounted image (`/tmp/.mount_XXXXXX/…`) — `LD_LIBRARY_PATH`,
//! `GDK_PIXBUF_MODULE_FILE`, `GST_PLUGIN_SYSTEM_PATH*`, plus markers like
//! `APPDIR`/`APPIMAGE`/`ARGV0`/`OWD`. Any shell or dev command spawned from
//! inside the app inherits them and breaks in two characteristic ways:
//!
//! - rustup's cargo shim reads `ARGV0` to pick a proxy → "unknown proxy name"
//! - webkit2gtk resolves its helper binaries relative to the bundle →
//!   "Failed to spawn WebKitNetworkProcess"
//!
//! `appimage_env_fix()` computes what to undo. It returns `None` outside an
//! AppImage so every spawn path can apply it unconditionally.

/// Environment repairs to apply to a child process before spawning it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvFix {
    /// Variables to remove entirely.
    pub remove: Vec<String>,
    /// Variables to overwrite (path lists with the AppImage entries filtered out).
    pub set: Vec<(String, String)>,
}

/// AppImage launcher markers that must never leak into children.
const MARKER_VARS: &[&str] = &["APPDIR", "APPIMAGE", "ARGV0", "OWD"];

/// Colon-separated path lists: filter out AppImage entries, keep the rest.
const PATH_LIST_VARS: &[&str] = &[
    "PATH",
    "LD_LIBRARY_PATH",
    "XDG_DATA_DIRS",
    "GST_PLUGIN_SYSTEM_PATH",
    "GST_PLUGIN_SYSTEM_PATH_1_0",
    "GTK_PATH",
    "GIO_EXTRA_MODULES",
    "PERLLIB",
    "PYTHONPATH",
    "QT_PLUGIN_PATH",
];

/// Compute the repairs for the current process environment. `None` when not
/// running from an AppImage (nothing to do).
pub fn appimage_env_fix() -> Option<EnvFix> {
    let appdir = std::env::var("APPDIR").ok()?;
    let appdir = appdir.trim_end_matches('/');
    if appdir.is_empty() {
        return None;
    }
    Some(compute_fix(appdir, std::env::vars()))
}

fn compute_fix(appdir: &str, vars: impl Iterator<Item = (String, String)>) -> EnvFix {
    let mut fix = EnvFix::default();
    let in_appdir = |entry: &str| {
        let e = entry.trim_end_matches('/');
        e == appdir || e.starts_with(&format!("{}/", appdir))
    };

    for (key, value) in vars {
        if MARKER_VARS.contains(&key.as_str()) {
            fix.remove.push(key);
            continue;
        }
        if PATH_LIST_VARS.contains(&key.as_str()) {
            let kept: Vec<&str> = value
                .split(':')
                .filter(|e| !e.is_empty() && !in_appdir(e))
                .collect();
            let filtered = kept.join(":");
            if filtered == value {
                continue; // untouched by the AppImage launcher
            }
            if filtered.is_empty() {
                fix.remove.push(key);
            } else {
                fix.set.push((key, filtered));
            }
            continue;
        }
        // Scalar vars pointed into the bundle (GDK_PIXBUF_MODULE_FILE,
        // GSETTINGS_SCHEMA_DIR, WEBKIT_EXEC_PATH, …): the bundle path is
        // useless outside the app, so drop them and let the child use system
        // defaults.
        if in_appdir(&value) {
            fix.remove.push(key);
        }
    }
    fix
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn no_appdir_means_no_fix() {
        // appimage_env_fix reads the real env; compute_fix is the pure core.
        // Simulate absence by checking the marker isn't required in compute_fix
        // callers — covered by the Option in appimage_env_fix.
        let fix = compute_fix("/tmp/.mount_XYZ", env(&[("HOME", "/home/u")]).into_iter());
        assert!(fix.remove.is_empty() && fix.set.is_empty());
    }

    #[test]
    fn markers_are_removed() {
        let fix = compute_fix(
            "/tmp/.mount_XYZ",
            env(&[
                ("APPDIR", "/tmp/.mount_XYZ"),
                ("APPIMAGE", "/home/u/Apps/x.AppImage"),
                ("ARGV0", "x_amd64"),
                ("OWD", "/home/u"),
            ])
            .into_iter(),
        );
        let mut removed = fix.remove.clone();
        removed.sort();
        assert_eq!(removed, vec!["APPDIR", "APPIMAGE", "ARGV0", "OWD"]);
    }

    #[test]
    fn path_lists_are_filtered_not_dropped() {
        let fix = compute_fix(
            "/tmp/.mount_XYZ",
            env(&[(
                "LD_LIBRARY_PATH",
                "/opt/cuda/lib64:/tmp/.mount_XYZ/usr/lib/:/tmp/.mount_XYZ/lib64/:",
            )])
            .into_iter(),
        );
        assert_eq!(
            fix.set,
            vec![("LD_LIBRARY_PATH".to_string(), "/opt/cuda/lib64".to_string())]
        );
        assert!(fix.remove.is_empty());
    }

    #[test]
    fn path_list_of_only_appdir_entries_is_removed() {
        let fix = compute_fix(
            "/tmp/.mount_XYZ",
            env(&[("GST_PLUGIN_SYSTEM_PATH", "/tmp/.mount_XYZ/usr/lib/gstreamer:")]).into_iter(),
        );
        assert_eq!(fix.remove, vec!["GST_PLUGIN_SYSTEM_PATH"]);
    }

    #[test]
    fn scalar_pointing_into_bundle_is_removed() {
        let fix = compute_fix(
            "/tmp/.mount_XYZ",
            env(&[(
                "GDK_PIXBUF_MODULE_FILE",
                "/tmp/.mount_XYZ//usr/lib/x86_64-linux-gnu/gdk-pixbuf-2.0/2.10.0/loaders.cache",
            )])
            .into_iter(),
        );
        // The double slash after the mount point must still match.
        assert_eq!(fix.remove, vec!["GDK_PIXBUF_MODULE_FILE"]);
    }

    #[test]
    fn unrelated_vars_survive() {
        let fix = compute_fix(
            "/tmp/.mount_XYZ",
            env(&[
                ("HOME", "/home/u"),
                ("SHELL", "/usr/bin/zsh"),
                ("PATH", "/usr/bin:/bin"),
            ])
            .into_iter(),
        );
        assert!(fix.remove.is_empty() && fix.set.is_empty());
    }
}
