//! Thin async client for the aria2 JSON-RPC interface.
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Clone)]
pub struct Aria2 {
    url: String,
    secret: String,
    http: reqwest::Client,
}

/// A normalized view of an aria2 download, ready for the frontend.
#[derive(Debug, Serialize)]
pub struct Download {
    pub gid: String,
    pub name: String,
    pub url: String,
    pub total: u64,
    pub completed: u64,
    pub speed: u64,
    pub status: String,
    pub dir: String,
}

impl Aria2 {
    pub fn new(port: u16, secret: impl Into<String>) -> Self {
        Aria2 {
            url: format!("http://127.0.0.1:{port}/jsonrpc"),
            secret: secret.into(),
            http: reqwest::Client::new(),
        }
    }

    fn token(&self) -> String {
        format!("token:{}", self.secret)
    }

    async fn call(&self, method: &str, mut params: Vec<Value>) -> Result<Value, String> {
        // Every authenticated call takes the secret token as the first param.
        let mut full = vec![json!(self.token())];
        full.append(&mut params);
        let body = json!({
            "jsonrpc": "2.0",
            "id": "idmnoads",
            "method": method,
            "params": full,
        });
        let resp = self
            .http
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let v: Value = resp.json().await.map_err(|e| e.to_string())?;
        if let Some(err) = v.get("error") {
            return Err(err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("aria2 error")
                .to_string());
        }
        Ok(v.get("result").cloned().unwrap_or(Value::Null))
    }

    pub async fn ping(&self) -> Result<Value, String> {
        self.call("aria2.getVersion", vec![]).await
    }

    pub async fn add_uri(&self, uris: Vec<String>, options: Value) -> Result<String, String> {
        let r = self
            .call("aria2.addUri", vec![json!(uris), options])
            .await?;
        Ok(r.as_str().unwrap_or_default().to_string())
    }

    pub async fn add_torrent(&self, base64: String, options: Value) -> Result<String, String> {
        let r = self
            .call("aria2.addTorrent", vec![json!(base64), json!([]), options])
            .await?;
        Ok(r.as_str().unwrap_or_default().to_string())
    }

    pub async fn pause(&self, gid: &str) -> Result<(), String> {
        self.call("aria2.pause", vec![json!(gid)]).await.map(|_| ())
    }
    pub async fn unpause(&self, gid: &str) -> Result<(), String> {
        self.call("aria2.unpause", vec![json!(gid)])
            .await
            .map(|_| ())
    }
    pub async fn pause_all(&self) -> Result<(), String> {
        self.call("aria2.pauseAll", vec![]).await.map(|_| ())
    }
    pub async fn unpause_all(&self) -> Result<(), String> {
        self.call("aria2.unpauseAll", vec![]).await.map(|_| ())
    }
    pub async fn remove(&self, gid: &str) -> Result<(), String> {
        // forceRemove for active; remove for stopped — try both gracefully.
        if self.call("aria2.remove", vec![json!(gid)]).await.is_err() {
            let _ = self.call("aria2.forceRemove", vec![json!(gid)]).await;
        }
        let _ = self
            .call("aria2.removeDownloadResult", vec![json!(gid)])
            .await;
        Ok(())
    }
    pub async fn purge(&self) -> Result<(), String> {
        self.call("aria2.purgeDownloadResult", vec![])
            .await
            .map(|_| ())
    }

    pub async fn change_global_option(&self, options: Value) -> Result<(), String> {
        self.call("aria2.changeGlobalOption", vec![options])
            .await
            .map(|_| ())
    }

    pub async fn global_stat(&self) -> Result<Value, String> {
        self.call("aria2.getGlobalStat", vec![]).await
    }

    pub async fn status(&self, gid: &str) -> Result<Value, String> {
        self.call("aria2.tellStatus", vec![json!(gid)]).await
    }

    pub async fn list(&self) -> Result<Vec<Download>, String> {
        let keys = json!([
            "gid", "status", "totalLength", "completedLength", "downloadSpeed", "dir",
            "files", "bittorrent", "errorMessage"
        ]);
        let mut out = Vec::new();
        let active = self.call("aria2.tellActive", vec![keys.clone()]).await?;
        push_items(&active, &mut out);
        let waiting = self
            .call("aria2.tellWaiting", vec![json!(0), json!(1000), keys.clone()])
            .await?;
        push_items(&waiting, &mut out);
        let stopped = self
            .call("aria2.tellStopped", vec![json!(0), json!(1000), keys])
            .await?;
        push_items(&stopped, &mut out);
        Ok(out)
    }
}

fn push_items(arr: &Value, out: &mut Vec<Download>) {
    if let Some(list) = arr.as_array() {
        for item in list {
            out.push(to_download(item));
        }
    }
}

fn to_download(v: &Value) -> Download {
    let gid = str_field(v, "gid");
    let status = str_field(v, "status");
    let total = num_field(v, "totalLength");
    let completed = num_field(v, "completedLength");
    let speed = num_field(v, "downloadSpeed");
    let dir = str_field(v, "dir");

    // Prefer the torrent name, then the on-disk file name, then the URL tail.
    let mut name = v
        .get("bittorrent")
        .and_then(|b| b.get("info"))
        .and_then(|i| i.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
    let mut url = String::new();
    if let Some(files) = v.get("files").and_then(|f| f.as_array()) {
        if let Some(first) = files.first() {
            if let Some(path) = first.get("path").and_then(|p| p.as_str()) {
                if name.is_empty() && !path.is_empty() {
                    name = basename(path);
                }
            }
            if let Some(uris) = first.get("uris").and_then(|u| u.as_array()) {
                if let Some(u0) = uris.first().and_then(|u| u.get("uri")).and_then(|u| u.as_str()) {
                    url = u0.to_string();
                }
            }
        }
    }
    if name.is_empty() {
        name = if url.is_empty() {
            format!("download-{gid}")
        } else {
            basename(&url)
        };
    }

    Download {
        gid,
        name,
        url,
        total,
        completed,
        speed,
        status,
        dir,
    }
}

fn basename(s: &str) -> String {
    let s = s.split(['?', '#']).next().unwrap_or(s);
    s.replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or(s)
        .to_string()
}
fn str_field(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}
fn num_field(v: &Value, k: &str) -> u64 {
    v.get(k)
        .and_then(|x| x.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| v.get(k).and_then(|x| x.as_u64()))
        .unwrap_or(0)
}
