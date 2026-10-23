//! Small, dependency-free persistent settings.
//!
//! StatsCraft is meant to run as a portable executable. The settings file
//! therefore lives beside the executable when that directory is writable,
//! and falls back to the per-user application data directory when it is
//! not (for example when the program is installed under Program Files).
//!
//! The format is a deliberately plain `key=value` text file. It keeps the
//! build free of a serialization dependency and stays easy to inspect.

use std::io::Write;
use std::path::{Path, PathBuf};

/// The most recent files kept in the File > Recent Files menu.
pub const MAX_RECENT: usize = 10;

#[derive(Debug, Clone)]
pub struct Settings {
    /// Whether the dark theme was active when the program last closed.
    pub dark: bool,
    /// The most recently opened data files, newest first.
    pub recent_files: Vec<String>,
    /// True when the last session closed cleanly. False means a restore
    /// snapshot may be offered at start-up.
    pub session_ok: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dark: false,
            recent_files: Vec::new(),
            session_ok: true,
        }
    }
}

impl Settings {
    /// Remember a file, newest first, without duplicates.
    pub fn remember_file(&mut self, path: &Path) {
        let text = path.display().to_string();
        self.recent_files.retain(|p| !same_path(p, &text));
        self.recent_files.insert(0, text);
        self.recent_files.truncate(MAX_RECENT);
    }

    pub fn forget_files(&mut self) {
        self.recent_files.clear();
    }
}

/// Load the settings, or the defaults when no file exists yet.
pub fn load() -> Settings {
    let path = config_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Settings::default();
    };
    let mut settings = Settings {
        session_ok: true,
        ..Settings::default()
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim().to_ascii_lowercase().as_str() {
            "dark" => {
                settings.dark = matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                );
            }
            "recent" => {
                let value = value.trim();
                if !value.is_empty() && !settings.recent_files.iter().any(|p| same_path(p, value)) {
                    settings.recent_files.push(value.to_string());
                }
            }
            "session_ok" => {
                settings.session_ok = !matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "0" | "false" | "no" | "off"
                );
            }
            _ => {}
        }
    }
    settings.recent_files.truncate(MAX_RECENT);
    settings
}

/// Write the settings, ignoring failures: a read-only installation must
/// still be usable, and a missing preference is not an application error.
pub fn save(settings: &Settings) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut text = String::new();
    text.push_str("# StatsCraft settings\n");
    text.push_str(&format!("dark={}\n", if settings.dark { 1 } else { 0 }));
    text.push_str(&format!("session_ok={}\n", if settings.session_ok { 1 } else { 0 }));
    for file in &settings.recent_files {
        if !file.contains('\n') && !file.contains('\r') {
            text.push_str(&format!("recent={}\n", file));
        }
    }
    // Write a sibling file and rename it, so a crash in the middle of a
    // save cannot leave a truncated settings file.
    let tmp = path.with_extension("ini.tmp");
    let written = std::fs::File::create(&tmp).and_then(|mut f| {
        f.write_all(text.as_bytes())?;
        f.flush()
    });
    if written.is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    } else {
        let _ = std::fs::remove_file(&tmp);
    }
}

/// The settings file: beside the executable when possible, otherwise in
/// the per-user configuration directory.
pub fn config_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("statscraft.ini");
            // A read-only installation can have a stale ini beside the
            // executable; use it only when the directory can be written.
            if candidate.exists() && directory_is_writable(dir) {
                return candidate;
            }
            if !candidate.exists() && directory_is_writable(dir) {
                return candidate;
            }
        }
    }
    user_config_dir().join("StatsCraft").join("settings.ini")
}

fn directory_is_writable(dir: &Path) -> bool {
    let probe = dir.join(".statscraft-write-test");
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// The `.sav` snapshot written while the session is dirty.
pub fn recovery_sav_path() -> PathBuf {
    config_path().with_file_name("statscraft-restore.sav")
}

/// Sidecar for the recovery snapshot: original path, weight, split.
pub fn recovery_meta_path() -> PathBuf {
    config_path().with_file_name("statscraft-restore.meta")
}

fn user_config_dir() -> PathBuf {
    if let Some(base) = std::env::var_os("APPDATA") {
        return PathBuf::from(base);
    }
    if let Some(base) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(base);
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config");
    }
    std::env::temp_dir()
}

/// Path comparison that follows the platform's case rules.
fn same_path(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_files_move_to_the_front_and_do_not_duplicate() {
        let mut s = Settings::default();
        s.remember_file(Path::new("C:/data/a.csv"));
        s.remember_file(Path::new("C:/data/b.csv"));
        s.remember_file(Path::new("C:/data/a.csv"));
        assert_eq!(s.recent_files, vec!["C:/data/a.csv", "C:/data/b.csv"]);
    }

    #[test]
    fn recent_files_are_capped() {
        let mut s = Settings::default();
        for i in 0..(MAX_RECENT + 5) {
            s.remember_file(Path::new(&format!("C:/data/{}.csv", i)));
        }
        assert_eq!(s.recent_files.len(), MAX_RECENT);
    }
}
