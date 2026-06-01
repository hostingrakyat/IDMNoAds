//! Local IPC server. The native-messaging host (`idmnoads-host.exe`) forwards
//! capture requests from the browser extensions here over a Windows named pipe
//! (`\\.\pipe\idmnoads`), using the same 4-byte-LE-length + UTF-8-JSON framing
//! as the native messaging protocol itself.
use crate::AppState;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

#[cfg(windows)]
pub const PIPE_NAME: &str = r"\\.\pipe\idmnoads";

/// Spawn the IPC listener on the Tauri async runtime.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run(app).await {
            eprintln!("[ipc] server stopped: {e}");
        }
    });
}

#[cfg(windows)]
async fn run(app: AppHandle) -> Result<(), String> {
    use tokio::net::windows::named_pipe::ServerOptions;

    let mut server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(PIPE_NAME)
        .map_err(|e| e.to_string())?;

    loop {
        server.connect().await.map_err(|e| e.to_string())?;
        let connected = server;
        // Pre-create the next instance so concurrent clients aren't refused.
        server = ServerOptions::new()
            .create(PIPE_NAME)
            .map_err(|e| e.to_string())?;
        let app2 = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = handle(connected, app2).await {
                eprintln!("[ipc] connection error: {e}");
            }
        });
    }
}

#[cfg(windows)]
async fn handle(
    mut pipe: tokio::net::windows::named_pipe::NamedPipeServer,
    app: AppHandle,
) -> Result<(), String> {
    use tokio::io::AsyncReadExt;
    // 4-byte little-endian length prefix.
    let mut len_buf = [0u8; 4];
    pipe.read_exact(&mut len_buf).await.map_err(|e| e.to_string())?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 || len > 16 * 1024 * 1024 {
        return Err("invalid frame length".into());
    }
    let mut buf = vec![0u8; len];
    pipe.read_exact(&mut buf).await.map_err(|e| e.to_string())?;
    let msg: Value = serde_json::from_slice(&buf).map_err(|e| e.to_string())?;
    process(&app, msg).await
}

/// Turn a capture request into an aria2 download and surface the window.
pub async fn process(app: &AppHandle, msg: Value) -> Result<(), String> {
    let url = msg
        .get("url")
        .and_then(|u| u.as_str())
        .unwrap_or("")
        .to_string();
    if url.is_empty() {
        return Err("missing url".into());
    }

    let state = app.state::<AppState>();
    let (dir, connections, speed_limit) = {
        let s = state.settings.lock().unwrap();
        (s.default_dir.clone(), s.connections, s.speed_limit)
    };

    let mut headers: Vec<String> = Vec::new();
    if let Some(cookies) = msg.get("cookies").and_then(|c| c.as_str()) {
        if !cookies.is_empty() {
            headers.push(format!("Cookie: {cookies}"));
        }
    }
    if let Some(referrer) = msg.get("referrer").and_then(|r| r.as_str()) {
        if !referrer.is_empty() {
            headers.push(format!("Referer: {referrer}"));
        }
    }
    if let Some(map) = msg.get("headers").and_then(|h| h.as_object()) {
        for (k, v) in map {
            if let Some(val) = v.as_str() {
                headers.push(format!("{k}: {val}"));
            }
        }
    }

    let mut options = json!({
        "split": connections,
        "max-connection-per-server": connections,
    });
    if !dir.is_empty() {
        options["dir"] = json!(dir);
    }
    if let Some(name) = msg.get("filename").and_then(|f| f.as_str()) {
        if !name.is_empty() {
            options["out"] = json!(name);
        }
    }
    if !headers.is_empty() {
        options["header"] = json!(headers);
    }
    if speed_limit > 0 {
        options["max-download-limit"] = json!(format!("{}K", speed_limit));
    }

    state.aria2.add_uri(vec![url], options).await?;
    let _ = app.emit("download-added", ());
    surface_window(app);
    Ok(())
}

/// Bring the main window to the foreground (a download was just captured).
pub fn surface_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

#[cfg(not(windows))]
async fn run(_app: AppHandle) -> Result<(), String> {
    // The named-pipe transport is Windows-only; on other platforms the IPC
    // server is a no-op so the crate still builds.
    Ok(())
}
