//! Tauri commands invoked from the web frontend.
use crate::aria2::Download;
use crate::settings::Settings;
use crate::AppState;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn add_download(
    state: State<'_, AppState>,
    url: String,
    dir: Option<String>,
    connections: Option<u32>,
) -> Result<String, String> {
    let (def_dir, def_conn, speed_limit) = {
        let s = state.settings.lock().unwrap();
        (s.default_dir.clone(), s.connections, s.speed_limit)
    };
    let conn = connections.unwrap_or(def_conn).clamp(1, 32);
    let target_dir = dir.filter(|d| !d.is_empty()).unwrap_or(def_dir);

    let mut options = json!({
        "split": conn,
        "max-connection-per-server": conn,
        // Re-downloading a URL whose file already exists must produce a fresh
        // numbered copy (file.1.ext), like IDM/browsers do. With aria2's global
        // --continue=true, a re-download instead sees the finished file and
        // silently marks itself "complete" without downloading anything. The
        // per-download override below fixes that; aria2 still resumes its own
        // interrupted downloads because the .aria2 control file takes priority.
        "continue": "false",
        "auto-file-renaming": "true",
    });
    if !target_dir.is_empty() {
        options["dir"] = json!(target_dir);
    }
    if speed_limit > 0 {
        options["max-download-limit"] = json!(format!("{}K", speed_limit));
    }
    state.aria2.add_uri(vec![url], options).await
}

#[tauri::command]
pub async fn add_torrent(state: State<'_, AppState>, base64: String) -> Result<String, String> {
    let dir = {
        let s = state.settings.lock().unwrap();
        s.default_dir.clone()
    };
    let mut options = json!({});
    if !dir.is_empty() {
        options["dir"] = json!(dir);
    }
    state.aria2.add_torrent(base64, options).await
}

#[tauri::command]
pub async fn list_downloads(state: State<'_, AppState>) -> Result<Vec<Download>, String> {
    state.aria2.list().await
}

#[tauri::command]
pub async fn global_stat(state: State<'_, AppState>) -> Result<Value, String> {
    state.aria2.global_stat().await
}

#[tauri::command]
pub async fn pause_download(state: State<'_, AppState>, gid: String) -> Result<(), String> {
    state.aria2.pause(&gid).await
}

#[tauri::command]
pub async fn resume_download(state: State<'_, AppState>, gid: String) -> Result<(), String> {
    state.aria2.unpause(&gid).await
}

#[tauri::command]
pub async fn remove_download(state: State<'_, AppState>, gid: String) -> Result<(), String> {
    state.aria2.remove(&gid).await
}

#[tauri::command]
pub async fn restart_download(
    state: State<'_, AppState>,
    gid: String,
    url: String,
) -> Result<String, String> {
    let _ = state.aria2.remove(&gid).await;
    add_download(state, url, None, None).await
}

#[tauri::command]
pub async fn pause_all(state: State<'_, AppState>) -> Result<(), String> {
    state.aria2.pause_all().await
}

#[tauri::command]
pub async fn resume_all(state: State<'_, AppState>) -> Result<(), String> {
    state.aria2.unpause_all().await
}

#[tauri::command]
pub async fn clear_completed(state: State<'_, AppState>) -> Result<(), String> {
    state.aria2.purge().await
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    // Persist and update the in-memory copy.
    {
        let mut guard = state.settings.lock().unwrap();
        *guard = settings.clone();
        guard
            .save(&state.config_path)
            .map_err(|e| e.to_string())?;
    }

    // Push relevant options into the running aria2 daemon.
    let mut opts = json!({
        "max-concurrent-downloads": settings.max_concurrent.to_string(),
    });
    if settings.speed_limit > 0 {
        opts["max-overall-download-limit"] = json!(format!("{}K", settings.speed_limit));
    } else {
        opts["max-overall-download-limit"] = json!("0");
    }
    // Always set all-proxy (empty string clears a previously configured proxy).
    opts["all-proxy"] = json!(settings.proxy);
    let _ = state.aria2.change_global_option(opts).await;

    // Apply Windows startup preference.
    apply_autostart(&app, settings.autostart);
    Ok(())
}

fn apply_autostart(app: &AppHandle, enable: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let mgr = app.autolaunch();
    let _ = if enable { mgr.enable() } else { mgr.disable() };
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |f| {
        let _ = tx.send(f);
    });
    let picked = rx.await.map_err(|e| e.to_string())?;
    Ok(picked.and_then(|fp| fp.into_path().ok().map(|p| p.to_string_lossy().to_string())))
}

#[tauri::command]
pub fn read_clipboard(app: AppHandle) -> Result<String, String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().read_text().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_clipboard(app: AppHandle, text: String) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    open_external(&url)
}

#[tauri::command]
pub async fn open_file(state: State<'_, AppState>, gid: String) -> Result<(), String> {
    let status = state.aria2.status(&gid).await?;
    let dir = status.get("dir").and_then(|d| d.as_str()).unwrap_or("");
    let path = status
        .get("files")
        .and_then(|f| f.as_array())
        .and_then(|a| a.first())
        .and_then(|f| f.get("path"))
        .and_then(|p| p.as_str())
        .unwrap_or("");

    // aria2 normally reports an absolute path, but if it's relative (or empty)
    // resolve it against the download dir so the file manager lands in the
    // right place instead of defaulting to Documents.
    let target = if !path.is_empty() {
        let p = std::path::Path::new(path);
        if p.is_absolute() || dir.is_empty() {
            path.to_string()
        } else {
            std::path::Path::new(dir).join(path).to_string_lossy().to_string()
        }
    } else if !dir.is_empty() {
        dir.to_string()
    } else {
        return Err("file path unknown".into());
    };
    reveal_in_folder(&target)
}

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Open the bundled (unpacked) browser-extension folder in the OS file manager.
///
/// The extension ships inside the installer under `resources/extension/<browser>`,
/// so the user never has to download or unzip anything — they only enable
/// Developer mode and point "Load unpacked" at the folder this opens.
/// `browser` is "chromium" or "firefox".
#[tauri::command]
pub fn open_extension_folder(app: AppHandle, browser: String) -> Result<String, String> {
    let sub = if browser == "firefox" { "firefox" } else { "chromium" };
    let dir = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("extension")
        .join(sub);
    if !dir.exists() {
        return Err(format!("extension folder not found: {}", dir.display()));
    }
    let path = dir.to_string_lossy().to_string();
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(path)
}

// ---- platform helpers ----
fn open_external(target: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", target])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn reveal_in_folder(path: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // aria2 can report forward slashes; Explorer needs backslashes.
        let win_path = path.replace('/', "\\");
        let is_file = std::path::Path::new(&win_path).is_file();
        // IMPORTANT: only the PATH may be quoted — not the whole "/select,..."
        // switch. Using .arg() makes std quote the entire argument
        // (`"/select,C:\dir\file"`), which Explorer can't parse, so it silently
        // falls back to the Documents folder. raw_arg passes the fragment
        // verbatim so the quotes land around the path only.
        let mut cmd = std::process::Command::new("explorer");
        if is_file {
            cmd.raw_arg(format!("/select,\"{win_path}\""));
        } else {
            // The file isn't there (renamed, moved, deleted) — open the nearest
            // existing folder instead of letting Explorer default to Documents.
            let dir = std::path::Path::new(&win_path)
                .parent()
                .filter(|d| d.exists())
                .map(|d| d.to_string_lossy().replace('/', "\\"))
                .unwrap_or_else(|| win_path.clone());
            cmd.raw_arg(format!("\"{dir}\""));
        }
        cmd.spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(not(windows))]
    {
        let parent = std::path::Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".into());
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
