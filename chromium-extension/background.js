/* IDM No Ads — Chromium background service worker (MV3).
 *
 * Maintains a persistent native-messaging port to com.idmnoads.host and hands
 * downloads off to the desktop app. MV3 service workers unload after ~30s idle,
 * so the port is lazily (re)connected and we reconnect on disconnect.
 */
const HOST = "com.idmnoads.host";

const DEFAULTS = {
  enabled: true,
  captureTypes: [
    "zip", "rar", "7z", "tar", "gz", "exe", "msi", "apk", "iso", "dmg",
    "mp4", "mkv", "avi", "mov", "webm", "flv", "mp3", "flac", "wav", "m4a",
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
  ],
  minSize: 0, // bytes; 0 = capture regardless of size
  rpcPort: 6800,
  rpcSecret: "",
};

let port = null;

function connect() {
  if (port) return port;
  try {
    port = chrome.runtime.connectNative(HOST);
    port.onMessage.addListener((msg) => {
      if (!msg) return;
      // Auto-discovered aria2 RPC config from the desktop app — store it so the
      // popup can poll the engine directly without the user copy/pasting a secret.
      if (msg.type === "config" && msg.secret) {
        chrome.storage.local.set({ rpcSecret: msg.secret, rpcPort: msg.port || 6800 });
      }
      // The host can ask us to show a notification (e.g. "queued").
      if (msg.notify) {
        notify(msg.title || "IDM No Ads", msg.notify);
      }
    });
    port.onDisconnect.addListener(() => {
      const err = chrome.runtime.lastError;
      console.warn("[idmnoads] native host disconnected", err && err.message);
      port = null; // reconnect lazily on next send
    });
    // Pull the RPC secret/port as soon as we connect.
    requestConfig();
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
    // Port may have silently died; retry once with a fresh connection.
    port = null;
    const p2 = connect();
    if (p2) p2.postMessage(payload);
  }
}

function notify(title, message) {
  chrome.notifications.create({
    type: "basic",
    iconUrl: "icons/icon-128.png",
    title,
    message,
  });
}

/** Ask the desktop app for the aria2 RPC secret + port (auto, no copy/paste). */
function requestConfig() {
  const p = connect();
  if (p) {
    try {
      p.postMessage({ type: "get-config" });
    } catch (_) {
      port = null;
    }
  }
}

async function getOptions() {
  const stored = await chrome.storage.local.get(DEFAULTS);
  return { ...DEFAULTS, ...stored };
}

function extOf(urlOrName) {
  try {
    const clean = (urlOrName || "").split(/[?#]/)[0];
    const base = clean.split("/").pop() || "";
    const dot = base.lastIndexOf(".");
    return dot >= 0 ? base.slice(dot + 1).toLowerCase() : "";
  } catch (_) {
    return "";
  }
}

async function cookieHeaderFor(url) {
  try {
    const cookies = await chrome.cookies.getAll({ url });
    return cookies.map((c) => `${c.name}=${c.value}`).join("; ");
  } catch (_) {
    return "";
  }
}

/** Build a capture request and forward it to the desktop app. */
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

/* ---------- Intercept browser downloads ---------- */
chrome.downloads.onDeterminingFilename.addListener((item, suggest) => {
  (async () => {
    const o = await getOptions();
    if (!o.enabled) return;
    const ext = extOf(item.filename) || extOf(item.finalUrl || item.url);
    const sizeOk = !o.minSize || item.fileSize < 0 || item.fileSize >= o.minSize;
    if (!o.captureTypes.includes(ext) || !sizeOk) return;

    // Take over: cancel + erase the browser's own download, then capture.
    const url = item.finalUrl || item.url;
    try {
      await chrome.downloads.cancel(item.id);
      await chrome.downloads.erase({ id: item.id });
    } catch (_) {}
    capture(url, {
      filename: item.filename,
      filesize: item.fileSize > 0 ? item.fileSize : 0,
      referrer: item.referrer || "",
    });
  })();
  // We are not suggesting a filename (download is being cancelled).
  return false;
});

/* ---------- Context menus ---------- */
function buildMenus() {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: "idm-link",
      title: "Download with IDM No Ads",
      contexts: ["link", "video", "audio", "image"],
    });
    chrome.contextMenus.create({
      id: "idm-page",
      title: "Download all media on this page with IDM No Ads",
      contexts: ["page"],
    });
  });
}
function onWake() {
  buildMenus();
  requestConfig();
}
chrome.runtime.onInstalled.addListener(onWake);
chrome.runtime.onStartup.addListener(onWake);

chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === "idm-link") {
    const target = info.linkUrl || info.srcUrl;
    if (target) capture(target, { referrer: info.pageUrl || (tab && tab.url) || "" });
  } else if (info.menuItemId === "idm-page") {
    if (tab && tab.id != null) {
      chrome.tabs.sendMessage(tab.id, { type: "grab-media" });
    }
  }
});

/* ---------- Messages from content script / popup ---------- */
chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg.type === "media-list" && Array.isArray(msg.urls)) {
    (async () => {
      for (const u of msg.urls) {
        await capture(u, { referrer: (sender.tab && sender.tab.url) || "" });
      }
    })();
  } else if (msg.type === "capture") {
    capture(msg.url, msg.opts || {});
  } else if (msg.type === "ensure-config") {
    // Popup opened — make sure we (re)fetch the RPC secret from the app.
    requestConfig();
  } else if (msg.type === "ping-host") {
    const p = connect();
    sendResponse({ connected: !!p });
    return true;
  }
});
