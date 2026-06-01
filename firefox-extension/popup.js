/* IDM No Ads popup (Firefox) — polls aria2 JSON-RPC on localhost. */
const DEFAULTS = { rpcPort: 6800, rpcSecret: "" };
let cfg = DEFAULTS;

function fmtBytes(n) {
  n = Number(n) || 0;
  if (n < 1024) return n + " B";
  const u = ["KB", "MB", "GB", "TB"];
  let i = -1;
  do { n /= 1024; i++; } while (n >= 1024 && i < u.length - 1);
  return n.toFixed(n < 10 ? 1 : 0) + " " + u[i];
}

async function rpc(method, params = []) {
  const body = {
    jsonrpc: "2.0",
    id: "popup",
    method,
    params: [`token:${cfg.rpcSecret}`, ...params],
  };
  const res = await fetch(`http://127.0.0.1:${cfg.rpcPort}/jsonrpc`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const j = await res.json();
  if (j.error) throw new Error(j.error.message);
  return j.result;
}

function nameOf(d) {
  if (d.bittorrent && d.bittorrent.info && d.bittorrent.info.name) return d.bittorrent.info.name;
  const f = d.files && d.files[0];
  if (f && f.path) return f.path.split(/[\\/]/).pop();
  if (f && f.uris && f.uris[0]) return f.uris[0].uri.split(/[?#]/)[0].split("/").pop();
  return d.gid;
}

function render(items, speed) {
  document.getElementById("stat").textContent = "↓ " + fmtBytes(speed) + "/s";
  const list = document.getElementById("list");
  if (!items.length) {
    list.innerHTML = '<div class="empty">No active downloads.</div>';
    return;
  }
  list.innerHTML = "";
  for (const d of items) {
    const total = Number(d.totalLength) || 0;
    const done = Number(d.completedLength) || 0;
    const pct = total ? Math.min(100, (done / total) * 100) : 0;
    const sp = Number(d.downloadSpeed) || 0;
    const el = document.createElement("div");
    el.className = "item";
    el.innerHTML = `
      <div class="name" title="${nameOf(d)}">${nameOf(d)}</div>
      <div class="bar"><span style="width:${pct.toFixed(1)}%"></span></div>
      <div class="meta"><span>${pct.toFixed(1)}% · ${fmtBytes(done)} / ${fmtBytes(total)}</span>
      <span>${fmtBytes(sp)}/s</span></div>`;
    list.appendChild(el);
  }
}

async function tick() {
  if (!cfg.rpcSecret) {
    document.getElementById("list").innerHTML =
      '<div class="warn">Set the aria2 RPC secret in <a id="go">Options</a> to see your downloads here.<br>(Desktop app → Settings → Advanced → copy secret.)</div>';
    const go = document.getElementById("go");
    if (go) go.onclick = () => browser.runtime.openOptionsPage();
    return;
  }
  try {
    const [active, stat] = await Promise.all([
      rpc("aria2.tellActive", [["gid", "status", "totalLength", "completedLength", "downloadSpeed", "files", "bittorrent"]]),
      rpc("aria2.getGlobalStat"),
    ]);
    render(active, Number(stat.downloadSpeed) || 0);
  } catch (e) {
    document.getElementById("list").innerHTML =
      `<div class="warn">Can't reach the download engine.<br>Make sure IDM No Ads is running.<br><small>${e.message}</small></div>`;
  }
}

document.getElementById("opts").onclick = () => browser.runtime.openOptionsPage();
document.getElementById("pauseAll").onclick = () => rpc("aria2.pauseAll").then(tick).catch(() => {});
document.getElementById("resumeAll").onclick = () => rpc("aria2.unpauseAll").then(tick).catch(() => {});

// Ask the background to fetch the RPC secret from the app if we don't have it.
browser.runtime.sendMessage({ type: "ensure-config" });
// Re-render immediately when the freshly-fetched config is stored.
browser.storage.onChanged.addListener((changes, area) => {
  if (area === "local" && (changes.rpcSecret || changes.rpcPort)) {
    if (changes.rpcSecret) cfg.rpcSecret = changes.rpcSecret.newValue;
    if (changes.rpcPort) cfg.rpcPort = changes.rpcPort.newValue;
    tick();
  }
});

browser.storage.local.get(DEFAULTS).then((stored) => {
  cfg = { ...DEFAULTS, ...stored };
  tick();
  setInterval(tick, 1000);
});
