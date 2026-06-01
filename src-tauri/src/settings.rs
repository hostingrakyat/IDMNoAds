//! Persistent user settings, stored as JSON in the app config directory.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub max_concurrent: u32,
    pub connections: u32,
    pub speed_limit: u64, // KB/s, 0 = unlimited
    pub default_dir: String,
    pub proxy: String,
    pub clipboard_watch: bool,
    pub autostart: bool,
    pub minimize_to_tray: bool,
    pub scheduler_enabled: bool,
    pub schedule_start: String,
    pub schedule_stop: String,
    pub filetypes: Vec<String>,
    pub language: String,
    pub theme: String,
    /// aria2 RPC secret — generated once and reused so the browser
    /// extension can keep talking to the same daemon.
    pub rpc_secret: String,
    /// Whether the first-run "add the extension to your browser" prompt has
    /// already been shown.
    pub browser_prompt_shown: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            max_concurrent: 5,
            connections: 16,
            speed_limit: 0,
            default_dir: default_download_dir(),
            proxy: String::new(),
            clipboard_watch: false,
            autostart: false,
            minimize_to_tray: true,
            scheduler_enabled: false,
            schedule_start: "01:00".into(),
            schedule_stop: "08:00".into(),
            filetypes: [
                "mp4", "mkv", "mp3", "flac", "pdf", "zip", "rar", "7z", "exe", "msi", "apk",
                "iso", "doc", "docx", "xls", "xlsx",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            language: "en".into(),
            theme: "dark".into(),
            rpc_secret: String::new(),
            browser_prompt_shown: false,
        }
    }
}

fn default_download_dir() -> String {
    if let Some(dir) = dirs_download() {
        dir.to_string_lossy().to_string()
    } else {
        String::new()
    }
}

fn dirs_download() -> Option<PathBuf> {
    // %USERPROFILE%\Downloads on Windows.
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(|p| Path::new(&p).join("Downloads"))
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(|p| Path::new(&p).join("Downloads"))
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self).unwrap_or_default();
        std::fs::write(path, data)
    }
}
