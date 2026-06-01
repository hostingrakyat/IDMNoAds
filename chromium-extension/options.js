/* IDM No Ads — options page logic (Chromium). */
const ALL_TYPES = [
  "zip", "rar", "7z", "tar", "gz", "exe", "msi", "apk", "iso", "dmg",
  "mp4", "mkv", "avi", "mov", "webm", "flv", "mp3", "flac", "wav", "m4a",
  "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
];
const DEFAULTS = {
  enabled: true,
  captureTypes: ALL_TYPES.slice(),
  minSize: 0,
  rpcPort: 6800,
  rpcSecret: "",
};

let selected = new Set();

function renderTypes() {
  const wrap = document.getElementById("types");
  wrap.innerHTML = "";
  ALL_TYPES.forEach((t) => {
    const c = document.createElement("span");
    c.className = "chip" + (selected.has(t) ? " on" : "");
    c.textContent = "." + t;
    c.onclick = () => {
      selected.has(t) ? selected.delete(t) : selected.add(t);
      renderTypes();
    };
    wrap.appendChild(c);
  });
}

async function load() {
  const o = { ...DEFAULTS, ...(await chrome.storage.local.get(DEFAULTS)) };
  document.getElementById("enabled").checked = o.enabled;
  document.getElementById("minSize").value = Math.round((o.minSize || 0) / (1024 * 1024));
  document.getElementById("rpcPort").value = o.rpcPort;
  document.getElementById("rpcSecret").value = o.rpcSecret;
  selected = new Set(o.captureTypes);
  renderTypes();
}

document.getElementById("save").onclick = async () => {
  const minMB = Number(document.getElementById("minSize").value) || 0;
  await chrome.storage.local.set({
    enabled: document.getElementById("enabled").checked,
    captureTypes: [...selected],
    minSize: minMB * 1024 * 1024,
    rpcPort: Number(document.getElementById("rpcPort").value) || 6800,
    rpcSecret: document.getElementById("rpcSecret").value.trim(),
  });
  const saved = document.getElementById("saved");
  saved.style.opacity = 1;
  setTimeout(() => (saved.style.opacity = 0), 1500);
};

load();
