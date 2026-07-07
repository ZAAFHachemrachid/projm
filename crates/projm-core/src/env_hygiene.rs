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
//! `appimage_env_fix()` computes what to undo. It returns `None` when there is
//! nothing to scrub so every spawn path can apply it unconditionally.
//!
//! Contamination is detected two ways: entries under the current `$APPDIR`,
//! and entries under `/tmp/.mount_*` — the runtime mount prefix every AppImage
//! launcher uses. The second catches leaks the first can't: a nested launch
//! (app started from inside another AppImage's shell) inherits the *outer*
//! mount's paths, and a half-scrubbed environment may have poisoned path lists
//! with no `APPDIR` marker left at all.

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

/// Runtime mount prefix used by AppImage launchers. Any path entry under it is
/// poisoned regardless of which AppImage (current, parent, stale) mounted it.
const MOUNT_PREFIX: &str = "/tmp/.mount_";

/// Compute the repairs for the current process environment. `None` when the
/// environment is clean (nothing to do).
pub fn appimage_env_fix() -> Option<EnvFix> {
    let appdir = std::env::var("APPDIR").ok();
    let appdir = appdir
        .as_deref()
        .map(|d| d.trim_end_matches('/'))
        .filter(|d| !d.is_empty());
    let fix = compute_fix(appdir, std::env::vars());
    if fix.remove.is_empty() && fix.set.is_empty() {
        None
    } else {
        Some(fix)
    }
}

fn compute_fix(appdir: Option<&str>, vars: impl Iterator<Item = (String, String)>) -> EnvFix {
    let mut fix = EnvFix::default();
    let poisoned = |entry: &str| {
        let e = entry.trim_end_matches('/');
        if e.starts_with(MOUNT_PREFIX) {
            return true;
        }
        // APPDIR can live outside /tmp/.mount_* (e.g. an extracted image run
        // via AppRun), so check it separately.
        match appdir {
            Some(d) => e == d || e.starts_with(&format!("{}/", d)),
            None => false,
        }
    };

    for (key, value) in vars {
        // Markers are only ever set by AppImage launchers; a stale one (e.g.
        // ARGV0 inherited through a nested launch) still breaks rustup, so
        // remove them whenever present.
        if MARKER_VARS.contains(&key.as_str()) {
            fix.remove.push(key);
            continue;
        }
        if PATH_LIST_VARS.contains(&key.as_str()) {
            let kept: Vec<&str> = value
                .split(':')
                .filter(|e| !e.is_empty() && !poisoned(e))
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
        if poisoned(&value) {
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
    fn clean_env_means_no_fix() {
        let fix = compute_fix(
            Some("/tmp/.mount_XYZ"),
            env(&[("HOME", "/home/u")]).into_iter(),
        );
        assert!(fix.remove.is_empty() && fix.set.is_empty());
    }

    #[test]
    fn markers_are_removed() {
        let fix = compute_fix(
            Some("/tmp/.mount_XYZ"),
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
            Some("/tmp/.mount_XYZ"),
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
            Some("/tmp/.mount_XYZ"),
            env(&[("GST_PLUGIN_SYSTEM_PATH", "/tmp/.mount_XYZ/usr/lib/gstreamer:")]).into_iter(),
        );
        assert_eq!(fix.remove, vec!["GST_PLUGIN_SYSTEM_PATH"]);
    }

    #[test]
    fn scalar_pointing_into_bundle_is_removed() {
        let fix = compute_fix(
            Some("/tmp/.mount_XYZ"),
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
            Some("/tmp/.mount_XYZ"),
            env(&[
                ("HOME", "/home/u"),
                ("SHELL", "/usr/bin/zsh"),
                ("PATH", "/usr/bin:/bin"),
            ])
            .into_iter(),
        );
        assert!(fix.remove.is_empty() && fix.set.is_empty());
    }

    #[test]
    fn other_appimage_mounts_are_filtered_too() {
        // Nested launch: current APPDIR is one mount, but the env still
        // carries entries from the outer AppImage's mount.
        let fix = compute_fix(
            Some("/tmp/.mount_INNER"),
            env(&[(
                "LD_LIBRARY_PATH",
                "/opt/cuda/lib64:/tmp/.mount_OUTER/usr/lib/:/tmp/.mount_INNER/lib64/",
            )])
            .into_iter(),
        );
        assert_eq!(
            fix.set,
            vec![("LD_LIBRARY_PATH".to_string(), "/opt/cuda/lib64".to_string())]
        );
    }

    #[test]
    fn poison_without_appdir_is_still_scrubbed() {
        // Half-scrubbed inheritance: markers already gone, path lists not.
        let fix = compute_fix(
            None,
            env(&[
                ("LD_LIBRARY_PATH", "/tmp/.mount_XYZ/usr/lib/:/opt/cuda/lib64"),
                ("ARGV0", "app.AppImage"),
                ("GDK_PIXBUF_MODULE_FILE", "/tmp/.mount_XYZ/usr/lib/loaders.cache"),
                ("PATH", "/usr/bin:/bin"),
            ])
            .into_iter(),
        );
        let mut removed = fix.remove.clone();
        removed.sort();
        assert_eq!(removed, vec!["ARGV0", "GDK_PIXBUF_MODULE_FILE"]);
        assert_eq!(
            fix.set,
            vec![("LD_LIBRARY_PATH".to_string(), "/opt/cuda/lib64".to_string())]
        );
    }
}
