// IDM No Ads — frontend controller. Talks to the Rust backend via Tauri's
// global API (app.withGlobalTauri = true), so no bundler/npm imports needed.
const TAURI = window.__TAURI__ || {};
const invoke = (cmd, args) =>
  TAURI.core ? TAURI.core.invoke(cmd, args) : Promise.reject("no tauri");
const listen = (ev, cb) => (TAURI.event ? TAURI.event.listen(ev, cb) : null);

const $ = (id) => document.getElementById(id);
const state = {
  lang: localStorage.getItem("lang") || "en",
  theme: localStorage.getItem("theme") || "dark",
  cat: "all",
  speedHistory: new Array(80).fill(0),
  settings: null,
};

const CATEGORY_EXT = {
  Videos: ["mp4", "mkv", "avi", "mov", "webm", "flv", "wmv", "m4v", "ts"],
  Music: ["mp3", "flac", "wav", "aac", "ogg", "m4a", "wma"],
  Documents: ["pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "txt", "epub"],
  Programs: ["exe", "msi", "apk", "dmg", "deb", "appimage"],
  Compressed: ["zip", "rar", "7z", "tar", "gz", "bz2", "xz"],
};
const ALL_FILETYPES = [
  "mp4", "mkv", "mp3", "flac", "pdf", "zip", "rar", "7z",
  "exe", "msi", "apk", "iso", "doc", "docx", "xls", "xlsx",
];

function categoryOf(name) {
  const ext = (name.split(".").pop() || "").toLowerCase();
  for (const [cat, exts] of Object.entries(CATEGORY_EXT)) {
    if (exts.includes(ext)) return cat;
  }
  return "General";
}

function fmtBytes(n) {
  n = Number(n) || 0;
  if (n < 1024) return n + " B";
  const u = ["KB", "MB", "GB", "TB"];
  let i = -1;
  do { n /= 1024; i++; } while (n >= 1024 && i < u.length - 1);
  return n.toFixed(n < 10 ? 2 : 1) + " " + u[i];
}
function fmtSpeed(n) { return fmtBytes(n) + "/s"; }
function fmtEta(sec) {
  if (!isFinite(sec) || sec <= 0) return "—";
  if (sec > 86400 * 7) return "—";
  const h = Math.floor(sec / 3600), m = Math.floor((sec % 3600) / 60), s = Math.floor(sec % 60);
  if (h) return `${h}h ${m}m`;
  if (m) return `${m}m ${s}s`;
  return `${s}s`;
}

// ---------- Theme / language ----------
function setTheme(t) {
  state.theme = t;
  document.body.setAttribute("data-theme", t);
  localStorage.setItem("theme", t);
}
function setLang(l) {
  state.lang = l;
  $("langSel").value = l;
  window.applyI18n(l);
  localStorage.setItem("lang", l);
  render(lastDownloads);
}

// ---------- Downloads rendering ----------
let lastDownloads = [];

function statusClass(s) {
  return { active: "active", paused: "paused", waiting: "waiting",
    complete: "complete", error: "error", removed: "error" }[s] || "waiting";
}

function matchesCat(d) {
  if (state.cat === "all") return true;
  if (state.cat === "active") return d.status === "active";
  return categoryOf(d.name) === state.cat;
}

function render(list) {
  lastDownloads = list;
  const body = $("dlBody");
  const counts = { all: 0, active: 0, General: 0, Videos: 0, Music: 0,
    Documents: 0, Programs: 0, Compressed: 0 };
  list.forEach((d) => {
    counts.all++;
    if (d.status === "active") counts.active++;
    counts[categoryOf(d.name)]++;
  });
  for (const k of Object.keys(counts)) {
    const el = $("cnt-" + k);
    if (el) el.textContent = counts[k];
  }

  const filtered = list.filter(matchesCat);
  body.innerHTML = "";
  $("emptyState").style.display = filtered.length ? "none" : "flex";

  for (const d of filtered) {
    const total = Number(d.total) || 0;
    const done = Number(d.completed) || 0;
    const pct = total ? Math.min(100, (done / total) * 100) : 0;
    const tr = document.createElement("tr");
    tr.innerHTML = `
      <td class="name-cell">
        <div class="fname" title="${escapeHtml(d.name)}">${escapeHtml(d.name)}</div>
        <div class="furl" title="${escapeHtml(d.url || "")}">${escapeHtml(d.url || "")}</div>
      </td>
      <td>${total ? fmtBytes(total) : "—"}</td>
      <td>
        <div class="bar"><span style="width:${pct.toFixed(1)}%"></span></div>
        <span class="pct">${pct.toFixed(1)}%</span>
      </td>
      <td>${d.status === "active" ? fmtSpeed(d.speed) : "—"}</td>
      <td>${d.status === "active" && d.speed > 0 ? fmtEta((total - done) / d.speed) : "—"}</td>
      <td><span class="status ${statusClass(d.status)}">${window.t("status_" + d.status) || d.status}</span></td>
      <td><div class="row-actions"></div></td>`;
    const actions = tr.querySelector(".row-actions");
    if (d.status === "active" || d.status === "waiting") {
      actions.appendChild(miniBtn("⏸", () => invoke("pause_download", { gid: d.gid })));
    } else if (d.status === "paused") {
      actions.appendChild(miniBtn("▶", () => invoke("resume_download", { gid: d.gid })));
    }
    if (d.status === "complete") {
      actions.appendChild(miniBtn("📂", () => invoke("open_file", { gid: d.gid })));
    }
    if (d.status === "error") {
      actions.appendChild(miniBtn("↻", () => invoke("restart_download", { gid: d.gid, url: d.url })));
    }
    actions.appendChild(miniBtn("✕", () => invoke("remove_download", { gid: d.gid }).then(refresh)));
    body.appendChild(tr);
  }
}

function miniBtn(label, fn) {
  const b = document.createElement("button");
  b.className = "mini";
  b.textContent = label;
  b.onclick = (e) => { e.stopPropagation(); fn(); };
  return b;
}
function escapeHtml(s) {
  return String(s || "").replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}

// ---------- Polling ----------
async function refresh() {
  try {
    const list = await invoke("list_downloads");
    render(list);
  } catch (e) { /* backend not ready */ }
}
async function pollStat() {
  try {
    const stat = await invoke("global_stat");
    const speed = Number(stat.downloadSpeed) || 0;
    $("globalStat").textContent = "↓ " + fmtSpeed(speed);
    state.speedHistory.push(speed);
    state.speedHistory.shift();
    drawGraph();
  } catch (e) { /* ignore */ }
}

function drawGraph() {
  const c = $("speedGraph");
  const w = (c.width = c.clientWidth || 600);
  const h = c.height;
  const ctx = c.getContext("2d");
  ctx.clearRect(0, 0, w, h);
  const data = state.speedHistory;
  const max = Math.max(1, ...data);
  const step = w / (data.length - 1);
  const grad = ctx.createLinearGradient(0, 0, 0, h);
  grad.addColorStop(0, "rgba(56,189,248,0.5)");
  grad.addColorStop(1, "rgba(37,99,235,0.02)");
  ctx.beginPath();
  ctx.moveTo(0, h);
  data.forEach((v, i) => ctx.lineTo(i * step, h - (v / max) * (h - 8) - 2));
  ctx.lineTo(w, h);
  ctx.closePath();
  ctx.fillStyle = grad;
  ctx.fill();
  ctx.beginPath();
  data.forEach((v, i) => {
    const y = h - (v / max) * (h - 8) - 2;
    i ? ctx.lineTo(i * step, y) : ctx.moveTo(0, y);
  });
  ctx.strokeStyle = "#38bdf8";
  ctx.lineWidth = 1.5;
  ctx.stroke();
}

// ---------- Add download ----------
function openModal(id) { $(id).hidden = false; }
function closeModal(id) { $(id).hidden = true; }

// Open a URL in the user's real browser (not inside the app webview).
function openExternal(url) {
  if (TAURI.opener) TAURI.opener.openUrl(url);
  else invoke("open_url", { url });
}

async function addUrls(text, dir, conn) {
  const urls = text.split(/[\r\n]+/).map((s) => s.trim()).filter(Boolean);
  for (const url of urls) {
    if (!/^(https?|ftp|sftp|magnet):/i.test(url)) continue;
    await invoke("add_download", { url, dir: dir || null, connections: conn || null });
  }
  refresh();
}

async function quickAdd() {
  const v = $("quickUrl").value.trim();
  if (!v) return;
  await addUrls(v);
  $("quickUrl").value = "";
}

// ---------- Settings ----------
async function loadSettings() {
  try {
    state.settings = await invoke("get_settings");
  } catch (e) {
    state.settings = defaultSettings();
  }
  applySettingsToForm();
  if (state.settings.language) setLang(state.settings.language);
  if (state.settings.theme) setTheme(state.settings.theme);
}
function defaultSettings() {
  return { max_concurrent: 5, connections: 16, speed_limit: 0, default_dir: "",
    proxy: "", clipboard_watch: false, autostart: false, minimize_to_tray: true,
    scheduler_enabled: false, schedule_start: "01:00", schedule_stop: "08:00",
    filetypes: ALL_FILETYPES.slice(), language: "en", theme: "dark", rpc_secret: "",
    browser_prompt_shown: false };
}
function applySettingsToForm() {
  const s = state.settings;
  $("s_concurrent").value = s.max_concurrent;
  $("s_conn").value = s.connections;
  $("s_speed").value = s.speed_limit;
  $("s_dir").value = s.default_dir || "";
  $("s_proxy").value = s.proxy || "";
  $("s_clipboard").checked = !!s.clipboard_watch;
  $("s_autostart").checked = !!s.autostart;
  $("s_tray").checked = !!s.minimize_to_tray;
  $("s_sched").checked = !!s.scheduler_enabled;
  $("s_sched_start").value = s.schedule_start || "01:00";
  $("s_sched_stop").value = s.schedule_stop || "08:00";
  $("s_rpc_secret").value = s.rpc_secret || "";
  renderFiletypeChips();
}
function renderFiletypeChips() {
  const wrap = $("filetypeChips");
  wrap.innerHTML = "";
  const on = new Set(state.settings.filetypes || []);
  ALL_FILETYPES.forEach((ext) => {
    const c = document.createElement("span");
    c.className = "chip" + (on.has(ext) ? " on" : "");
    c.textContent = "." + ext;
    c.onclick = () => {
      on.has(ext) ? on.delete(ext) : on.add(ext);
      state.settings.filetypes = [...on];
      renderFiletypeChips();
    };
    wrap.appendChild(c);
  });
}
async function saveSettings() {
  const s = state.settings;
  s.max_concurrent = +$("s_concurrent").value;
  s.connections = +$("s_conn").value;
  s.speed_limit = +$("s_speed").value;
  s.default_dir = $("s_dir").value;
  s.proxy = $("s_proxy").value;
  s.clipboard_watch = $("s_clipboard").checked;
  s.autostart = $("s_autostart").checked;
  s.minimize_to_tray = $("s_tray").checked;
  s.scheduler_enabled = $("s_sched").checked;
  s.schedule_start = $("s_sched_start").value;
  s.schedule_stop = $("s_sched_stop").value;
  s.language = state.lang;
  s.theme = state.theme;
  await invoke("save_settings", { settings: s });
  closeModal("settingsModal");
}

// Show the "add the extension to your browser" prompt once, on first launch.
function maybeShowFirstRunExtension() {
  if (state.settings && !state.settings.browser_prompt_shown) {
    openModal("extModal");
    state.settings.browser_prompt_shown = true;
    invoke("save_settings", { settings: state.settings }).catch(() => {});
  }
}

async function pickFolder(targetInput) {
  try {
    const dir = await invoke("pick_folder");
    if (dir) $(targetInput).value = dir;
  } catch (e) { /* dialog plugin unavailable */ }
}

// ---------- Drag & drop (URLs + .torrent) ----------
function setupDnd() {
  const overlay = $("dropOverlay");
  let depth = 0;
  window.addEventListener("dragenter", (e) => { e.preventDefault(); depth++; overlay.hidden = false; });
  window.addEventListener("dragover", (e) => e.preventDefault());
  window.addEventListener("dragleave", (e) => { depth--; if (depth <= 0) overlay.hidden = true; });
  window.addEventListener("drop", async (e) => {
    e.preventDefault();
    depth = 0; overlay.hidden = true;
    const dt = e.dataTransfer;
    if (dt.files && dt.files.length) {
      for (const f of dt.files) {
        if (f.name.toLowerCase().endsWith(".torrent")) {
          const b64 = await fileToBase64(f);
          await invoke("add_torrent", { base64: b64 });
        }
      }
      refresh();
      return;
    }
    const url = dt.getData("text/uri-list") || dt.getData("text/plain");
    if (url) addUrls(url);
  });
}
function fileToBase64(file) {
  return new Promise((res, rej) => {
    const r = new FileReader();
    r.onload = () => res(String(r.result).split(",")[1]);
    r.onerror = rej;
    r.readAsDataURL(file);
  });
}

// ---------- Wire up ----------
function wire() {
  $("addBtn").onclick = () => { $("m_url").value = $("quickUrl").value; openModal("addModal"); };
  $("quickUrl").addEventListener("keydown", (e) => { if (e.key === "Enter") quickAdd(); });
  $("grabBtn").onclick = async () => {
    try {
      const txt = await invoke("read_clipboard");
      if (txt) { $("m_url").value = txt; openModal("addModal"); }
    } catch (e) {}
  };
  $("themeBtn").onclick = () => setTheme(state.theme === "dark" ? "light" : "dark");
  $("langSel").onchange = (e) => setLang(e.target.value);

  // Categories
  $("catList").querySelectorAll("li").forEach((li) => {
    li.onclick = () => {
      $("catList").querySelector(".active")?.classList.remove("active");
      li.classList.add("active");
      state.cat = li.dataset.cat;
      render(lastDownloads);
    };
  });

  // Toolbar
  $("tbResumeAll").onclick = () => invoke("resume_all").then(refresh);
  $("tbPauseAll").onclick = () => invoke("pause_all").then(refresh);
  $("tbClear").onclick = () => invoke("clear_completed").then(refresh);

  // Add modal
  $("m_browse").onclick = () => pickFolder("m_dir");
  $("m_cancel").onclick = () => closeModal("addModal");
  $("m_ok").onclick = async () => {
    await addUrls($("m_url").value, $("m_dir").value, +$("m_conn").value);
    closeModal("addModal");
    $("m_url").value = "";
  };

  // Settings modal
  $("navSettings").onclick = () => { applySettingsToForm(); openModal("settingsModal"); };
  $("s_browse").onclick = () => pickFolder("s_dir");
  $("set_cancel").onclick = () => closeModal("settingsModal");
  $("set_save").onclick = saveSettings;
  $("s_copy_secret").onclick = () => {
    invoke("write_clipboard", { text: $("s_rpc_secret").value }).catch(() => {});
  };

  // Browser-integration modal — the extension ships bundled with the app, so
  // these buttons just reveal the folder for "Load unpacked" / "Load Temporary
  // Add-on". If the bundled copy can't be found (e.g. dev build), fall back to
  // the Releases page so the user can still grab the zip.
  $("navExtension").onclick = () => openModal("extModal");
  $("ext_close").onclick = () => closeModal("extModal");
  const openExtFolder = (browser) =>
    invoke("open_extension_folder", { browser }).catch(() =>
      openExternal("https://github.com/hostingrakyat/IDMNoAds/releases/latest")
    );
  $("extOpenChrome").onclick = () => openExtFolder("chromium");
  $("extOpenFirefox").onclick = () => openExtFolder("firefox");

  // About modal
  $("navAbout").onclick = () => openModal("aboutModal");
  $("about_close").onclick = () => closeModal("aboutModal");

  // Open social links in the default browser via the opener plugin
  document.querySelectorAll(".socials a").forEach((a) => {
    a.addEventListener("click", (e) => {
      e.preventDefault();
      openExternal(a.href);
    });
  });

  setupDnd();
}

// ---------- Backend push events ----------
function setupEvents() {
  listen("download-added", () => refresh());
  listen("focus-window", () => {});
  listen("settings-updated", () => loadSettings());
}

async function main() {
  setTheme(state.theme);
  window.applyI18n(state.lang);
  $("langSel").value = state.lang;
  wire();
  setupEvents();
  await loadSettings();
  maybeShowFirstRunExtension();
  await refresh();
  await pollStat();
  setInterval(refresh, 1000);
  setInterval(pollStat, 1000);
  // version in about
  try {
    const v = await invoke("app_version");
    if (v) $("aboutVer").textContent = "v" + v;
  } catch (e) {}
}

window.addEventListener("DOMContentLoaded", main);
