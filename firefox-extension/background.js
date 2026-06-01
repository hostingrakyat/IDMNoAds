/* IDM No Ads — Firefox background script (MV3).
 *
 * Uses the native `browser.*` promise API. Firefox lacks
 * downloads.onDeterminingFilename, so we intercept via downloads.onCreated:
 * cancel + erase the browser's own download, then forward it to the desktop
 * app over the persistent native-messaging port (com.idmnoads.host). */
const HOST = "com.idmnoads.host";

const DEFAULTS = {
  enabled: true,
  captureTypes: [
    "zip", "rar", "7z", "tar", "gz", "exe", "msi", "apk", "iso", "dmg",
    "mp4", "mkv", "avi", "mov", "webm", "flv", "mp3", "flac", "wav", "m4a",
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
  ],
  minSize: 0,
  rpcPort: 6800,
  rpcSecret: "",
};

let port = null;

function connect() {
  if (port) return port;
  try {
    port = browser.runtime.connectNative(HOST);
    port.onMessage.addListener((msg) => {
      if (msg && msg.notify) notify(msg.title || "IDM No Ads", msg.notify);
    });
    port.onDisconnect.addListener((p) => {
      console.warn("[idmnoads] native host disconnected", p.error && p.error.message);
      port = null;
    });
  } catch (e) {
    console.error("[idmnoads] connectNative failed", e);
    port = null;
  }
  return port;
}

function sendToHost(payload) {
  const p = connect();
  if (!p) {
    notify("IDM No Ads", "Desktop app not found. Is IDM No Ads installed and running?");
    return;
  }
  try {
    p.postMessage(payload);
  } catch (e) {
    port = null;
    const p2 = connect();
    if (p2) p2.postMessage(payload);
  }
}

function notify(title, message) {
  browser.notifications.create({ type: "basic", iconUrl: "icons/icon-128.png", title, message });
}

async function getOptions() {
  const stored = await browser.storage.local.get(DEFAULTS);
  return { ...DEFAULTS, ...stored };
}

function extOf(urlOrName) {
  const clean = (urlOrName || "").split(/[?#]/)[0];
  const base = clean.split("/").pop() || "";
  const dot = base.lastIndexOf(".");
  return dot >= 0 ? base.slice(dot + 1).toLowerCase() : "";
}

async function cookieHeaderFor(url) {
  try {
    const cookies = await browser.cookies.getAll({ url });
    return cookies.map((c) => `${c.name}=${c.value}`).join("; ");
  } catch (_) {
    return "";
  }
}

async function capture(url, opts = {}) {
  const cookies = await cookieHeaderFor(url);
  sendToHost({
    url,
    cookies,
    referrer: opts.referrer || "",
    filename: opts.filename || "",
    filesize: opts.filesize || 0,
    headers: opts.headers || {},
    userAgent: navigator.userAgent,
  });
  notify("IDM No Ads", "Sent to download manager:\n" + (opts.filename || url));
}

/* ---------- Intercept browser downloads (Firefox: onCreated) ---------- */
browser.downloads.onCreated.addListener(async (item) => {
  const o = await getOptions();
  if (!o.enabled) return;
  const url = item.finalUrl || item.url;
  const ext = extOf(item.filename) || extOf(url);
  const sizeOk = !o.minSize || !item.fileSize || item.fileSize < 0 || item.fileSize >= o.minSize;
  if (!o.captureTypes.includes(ext) || !sizeOk) return;

  try {
    await browser.downloads.cancel(item.id);
    await browser.downloads.erase({ id: item.id });
  } catch (_) {}
  capture(url, {
    filename: (item.filename || "").split(/[\\/]/).pop(),
    filesize: item.fileSize > 0 ? item.fileSize : 0,
    referrer: item.referrer || "",
  });
});

/* ---------- Context menus ---------- */
function buildMenus() {
  browser.contextMenus.removeAll().then(() => {
    browser.contextMenus.create({
      id: "idm-link",
      title: "Download with IDM No Ads",
      contexts: ["link", "video", "audio", "image"],
    });
    browser.contextMenus.create({
      id: "idm-page",
      title: "Download all media on this page with IDM No Ads",
      contexts: ["page"],
    });
  });
}
browser.runtime.onInstalled.addListener(buildMenus);
browser.runtime.onStartup.addListener(buildMenus);

browser.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === "idm-link") {
    const target = info.linkUrl || info.srcUrl;
    if (target) capture(target, { referrer: info.pageUrl || (tab && tab.url) || "" });
  } else if (info.menuItemId === "idm-page" && tab && tab.id != null) {
    browser.tabs.sendMessage(tab.id, { type: "grab-media" });
  }
});

/* ---------- Messages from content script / popup ---------- */
browser.runtime.onMessage.addListener(async (msg, sender) => {
  if (msg.type === "media-list" && Array.isArray(msg.urls)) {
    for (const u of msg.urls) {
      await capture(u, { referrer: (sender.tab && sender.tab.url) || "" });
    }
  } else if (msg.type === "capture") {
    capture(msg.url, msg.opts || {});
  } else if (msg.type === "ping-host") {
    return { connected: !!connect() };
  }
});
