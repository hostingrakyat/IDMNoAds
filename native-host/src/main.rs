//! IDM No Ads — native messaging host (`idmnoads-host.exe`).
//!
//! The browser extension launches this process and speaks the standard Chrome/
//! Firefox native-messaging protocol over stdin/stdout: each message is a
//! 4-byte little-endian length prefix followed by a UTF-8 JSON body.
//!
//! Each message (a download capture request) is forwarded to the running
//! desktop app through the Windows named pipe `\\.\pipe\idmnoads`, using the
//! same framing. A small JSON acknowledgement is written back to the browser.
use std::io::{self, Read, Write};

#[cfg_attr(not(windows), allow(dead_code))]
const PIPE_NAME: &str = r"\\.\pipe\idmnoads";
const MAX_MESSAGE: usize = 64 * 1024 * 1024;

fn read_message<R: Read>(r: &mut R) -> Option<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    if r.read_exact(&mut len_buf).is_err() {
        return None; // stdin closed -> browser disconnected
    }
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 || len > MAX_MESSAGE {
        return None;
    }
    let mut buf = vec![0u8; len];
    if r.read_exact(&mut buf).is_err() {
        return None;
    }
    Some(buf)
}

fn write_message<W: Write>(w: &mut W, payload: &[u8]) -> io::Result<()> {
    w.write_all(&(payload.len() as u32).to_le_bytes())?;
    w.write_all(payload)?;
    w.flush()
}

#[cfg(windows)]
fn forward_to_app(payload: &[u8]) -> io::Result<()> {
    use std::fs::OpenOptions;
    let mut pipe = OpenOptions::new()
        .read(true)
        .write(true)
        .open(PIPE_NAME)?;
    pipe.write_all(&(payload.len() as u32).to_le_bytes())?;
    pipe.write_all(payload)?;
    pipe.flush()
}

#[cfg(not(windows))]
fn forward_to_app(_payload: &[u8]) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "named pipe IPC is only available on Windows",
    ))
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();

    while let Some(msg) = read_message(&mut stdin.lock()) {
        let reply = match forward_to_app(&msg) {
            Ok(()) => serde_json::json!({ "ok": true }),
            Err(e) => serde_json::json!({
                "ok": false,
                "title": "IDM No Ads",
                "notify": format!(
                    "IDM No Ads desktop app isn't running ({e}). Open it and try the download again."
                )
            }),
        };
        let bytes = serde_json::to_vec(&reply).unwrap_or_else(|_| b"{}".to_vec());
        if write_message(&mut stdout.lock(), &bytes).is_err() {
            break;
        }
    }
}
