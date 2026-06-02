//! IDM No Ads — Tauri 2 backend.
//!
//! Responsibilities:
//!   * launch the bundled `aria2c.exe` sidecar with a private RPC secret,
//!   * expose download-management commands to the web frontend,
//!   * host a named-pipe IPC server for the browser native-messaging host,
//!   * provide a system tray + minimize-to-tray behaviour.
mod aria2;
mod commands;
mod ipc;
mod settings;

use aria2::Aria2;
use settings::Settings;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

pub(crate) const RPC_PORT: u16 = 6800;

pub struct AppState {
    pub aria2: Aria2,
    pub settings: Mutex<Settings>,
    pub config_path: PathBuf,
    pub aria2_child: Mutex<Option<std::process::Child>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Single-instance must be registered first; focus the window if a
        // second instance is launched (e.g. via the extension or a file assoc).
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            ipc::surface_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            setup(app)?;
            Ok(())
        })
        .on_window_event(on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::add_download,
            commands::add_torrent,
            commands::list_downloads,
            commands::global_stat,
            commands::pause_download,
            commands::resume_download,
            commands::remove_download,
            commands::restart_download,
            commands::pause_all,
            commands::resume_all,
            commands::clear_completed,
            commands::get_settings,
            commands::save_settings,
            commands::pick_folder,
            commands::read_clipboard,
            commands::write_clipboard,
            commands::open_url,
            commands::open_file,
            commands::app_version,
        ])
        .build(tauri::generate_context!())
        .expect("error while building IDM No Ads")
        .run(|app_handle, event| {
            // Clean up the aria2 child process when the app exits.
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    if let Some(mut child) = state.aria2_child.lock().unwrap().take() {
                        let _ = child.kill();
                    }
                }
            }
        });
}

fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();

    // Resolve config path and load settings.
    let config_dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("idmnoads"));
    std::fs::create_dir_all(&config_dir).ok();
    let config_path = config_dir.join("settings.json");
    let mut settings = Settings::load(&config_path);

    // Ensure a stable RPC secret exists (shared with the browser extension).
    if settings.rpc_secret.is_empty() {
        settings.rpc_secret = random_secret();
        let _ = settings.save(&config_path);
    }

    // Launch the aria2c sidecar.
    let aria2_child = spawn_aria2(&settings).ok();
    let aria2 = Aria2::new(RPC_PORT, settings.rpc_secret.clone());

    app.manage(AppState {
        aria2: aria2.clone(),
        settings: Mutex::new(settings.clone()),
        config_path,
        aria2_child: Mutex::new(aria2_child),
    });

    // Apply the autostart preference on launch.
    {
        use tauri_plugin_autostart::ManagerExt;
        let mgr = app.autolaunch();
        let _ = if settings.autostart {
            mgr.enable()
        } else {
            mgr.disable()
        };
    }

    build_tray(app)?;
    ipc::start(handle.clone());
    #[cfg(windows)]
    register_native_host(app).unwrap_or_else(|e| eprintln!("[native-host] registration failed: {e}"));
    start_clipboard_watch(handle.clone());
    start_scheduler(handle);

    Ok(())
}

/// Register the native-messaging host at app first-run.
///
/// NSIS/MSI installs do this at install-time via `nsis/hooks.nsi`.  For MSIX
/// (Microsoft Store) packages, the NSIS post-install hook never runs, so we
/// perform the same registry writes here at runtime.  A sentinel flag file
/// prevents redundant re-registration on every subsequent launch.
#[cfg(windows)]
fn register_native_host(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("idmnoads"));
    std::fs::create_dir_all(&config_dir)?;

    // Only register once — skip if the sentinel is present.
    let sentinel = config_dir.join("native_host_registered");
    if sentinel.exists() {
        return Ok(());
    }

    // Resolve the host executable path at runtime (works for MSIX + NSIS).
    let exe_dir = std::env::current_exe()?
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let host_exe = ["idmnoads-host.exe", "idmnoads-host-x86_64-pc-windows-msvc.exe"]
        .iter()
        .map(|name| exe_dir.join(name))
        .find(|p| p.exists())
        .unwrap_or_else(|| exe_dir.join("idmnoads-host.exe"));

    // Write the two native-messaging manifest JSON files.
    let chrome_manifest = config_dir.join("com.idmnoads.host.chrome.json");
    let firefox_manifest = config_dir.join("com.idmnoads.host.firefox.json");
    let host_path = host_exe.to_string_lossy().replace('\\', "\\\\");

    std::fs::write(
        &chrome_manifest,
        format!(
            r#"{{
  "name": "com.idmnoads.host",
  "description": "IDM No Ads native messaging host",
  "path": "{host_path}",
  "type": "stdio",
  "allowed_origins": [ "chrome-extension://ikbamigoaahjngjceemkppoimlphgmii/", "chrome-extension://hbommjcibnllhahdgcjkkbjikbifmenk/" ]
}}"#
        ),
    )?;
    std::fs::write(
        &firefox_manifest,
        format!(
            r#"{{
  "name": "com.idmnoads.host",
  "description": "IDM No Ads native messaging host",
  "path": "{host_path}",
  "type": "stdio",
  "allowed_extensions": [ "idmnoads@hostingrakyat" ]
}}"#
        ),
    )?;

    // Write HKCU registry keys for each browser (no elevation required).
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let chrome_path = chrome_manifest.to_string_lossy().to_string();
    let firefox_path = firefox_manifest.to_string_lossy().to_string();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    // Use explicit &[(&str, &str)] with destructure-by-ref to keep types unambiguous.
    let entries: &[(&str, &str)] = &[
        (r"Software\Google\Chrome\NativeMessagingHosts\com.idmnoads.host", &chrome_path),
        (r"Software\Chromium\NativeMessagingHosts\com.idmnoads.host", &chrome_path),
        (r"Software\Microsoft\Edge\NativeMessagingHosts\com.idmnoads.host", &chrome_path),
        (r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\com.idmnoads.host", &chrome_path),
        (r"Software\Mozilla\NativeMessagingHosts\com.idmnoads.host", &firefox_path),
    ];
    for &(key_path, manifest_path) in entries {
        let (key, _) = hkcu.create_subkey(key_path)?;
        key.set_value("", &manifest_path)?;
    }

    // Write sentinel so we skip this on subsequent launches.
    std::fs::write(&sentinel, b"")?;
    Ok(())
}

/// System tray with Open / Quit, left-click opens the window.
fn build_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItem::with_id(app, "show", "Open IDM No Ads", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let click_handle = app.handle().clone();
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("IDM No Ads")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => ipc::surface_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(move |_tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                ipc::surface_window(&click_handle);
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

/// Minimize-to-tray: hide instead of closing when the user clicks X.
fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        let to_tray = window
            .app_handle()
            .try_state::<AppState>()
            .map(|s| s.settings.lock().unwrap().minimize_to_tray)
            .unwrap_or(true);
        if to_tray {
            api.prevent_close();
            let _ = window.hide();
        }
    }
}

/// Optional clipboard URL auto-capture. Polls every ~1.5s when enabled.
fn start_clipboard_watch(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        use tauri_plugin_clipboard_manager::ClipboardExt;
        let mut last = String::new();
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            let enabled = app
                .try_state::<AppState>()
                .map(|s| s.settings.lock().unwrap().clipboard_watch)
                .unwrap_or(false);
            if !enabled {
                continue;
            }
            if let Ok(text) = app.clipboard().read_text() {
                let trimmed = text.trim().to_string();
                if trimmed != last
                    && is_downloadable_url(&trimmed)
                {
                    last = trimmed.clone();
                    let msg = serde_json::json!({ "url": trimmed });
                    let _ = ipc::handle_message(&app, msg).await;
                }
            }
        }
    });
}

/// Scheduler: every 60 s check whether we are inside the configured time window
/// and pause / unpause aria2 accordingly.  Outside the window all downloads are
/// paused; inside they are unpaused (respecting the user's per-download pauses).
fn start_scheduler(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let state = match app.try_state::<AppState>() {
                Some(s) => s,
                None => continue,
            };
            let (enabled, start, stop, aria2) = {
                let s = state.settings.lock().unwrap();
                (
                    s.scheduler_enabled,
                    s.schedule_start.clone(),
                    s.schedule_stop.clone(),
                    state.aria2.clone(),
                )
            };
            if !enabled {
                continue;
            }
            if time_in_window(&start, &stop) {
                let _ = aria2.unpause_all().await;
            } else {
                let _ = aria2.pause_all().await;
            }
        }
    });
}

/// Returns true when the current local time is within [start, stop) (HH:MM).
/// Handles overnight windows where stop < start (e.g. 22:00–06:00).
fn time_in_window(start: &str, stop: &str) -> bool {
    use chrono::Timelike;
    let now = chrono::Local::now();
    let now_m = now.hour() * 60 + now.minute();

    let parse = |s: &str| -> u32 {
        let mut it = s.splitn(2, ':');
        let h: u32 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        let m: u32 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        h * 60 + m
    };
    let s = parse(start);
    let e = parse(stop);
    if s <= e {
        now_m >= s && now_m < e
    } else {
        // overnight: active from start until midnight, then midnight until stop
        now_m >= s || now_m < e
    }
}

fn is_downloadable_url(s: &str) -> bool {
    if !(s.starts_with("http://") || s.starts_with("https://") || s.starts_with("ftp://")) {
        return false;
    }
    // crude heuristic: ends with a file-ish extension
    let tail = s.split(['?', '#']).next().unwrap_or(s);
    let exts = [
        ".zip", ".rar", ".7z", ".exe", ".msi", ".mp4", ".mkv", ".mp3", ".flac", ".pdf",
        ".iso", ".apk", ".dmg", ".tar", ".gz", ".doc", ".docx", ".xls", ".xlsx",
    ];
    exts.iter().any(|e| tail.to_lowercase().ends_with(e))
}

fn spawn_aria2(settings: &Settings) -> std::io::Result<std::process::Child> {
    let exe = aria2_path();
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--enable-rpc")
        .arg("--rpc-listen-all=false")
        .arg(format!("--rpc-listen-port={RPC_PORT}"))
        .arg(format!("--rpc-secret={}", settings.rpc_secret))
        .arg("--rpc-allow-origin-all")
        .arg("--continue=true")
        .arg("--file-allocation=none")
        .arg("--min-split-size=1M")
        .arg("--split=16")
        .arg("--max-connection-per-server=16")
        .arg(format!(
            "--max-concurrent-downloads={}",
            settings.max_concurrent
        ));
    if !settings.default_dir.is_empty() {
        cmd.arg(format!("--dir={}", settings.default_dir));
    }
    if settings.speed_limit > 0 {
        cmd.arg(format!("--max-overall-download-limit={}K", settings.speed_limit));
    }
    if !settings.proxy.is_empty() {
        cmd.arg(format!("--all-proxy={}", settings.proxy));
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.spawn()
}

/// Find the aria2c binary next to our own executable (installed as a sidecar).
fn aria2_path() -> PathBuf {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let candidates = if cfg!(windows) {
        vec!["aria2c.exe", "aria2c-x86_64-pc-windows-msvc.exe"]
    } else {
        vec!["aria2c", "aria2c-x86_64-unknown-linux-gnu"]
    };
    for c in &candidates {
        let p = dir.join(c);
        if p.exists() {
            return p;
        }
    }
    dir.join(candidates[0])
}

fn random_secret() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
