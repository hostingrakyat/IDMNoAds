/* IDM No Ads — content script (Firefox). Detects page media for "grab all". */
(function () {
  function collectMedia() {
    const urls = new Set();
    document.querySelectorAll("video, audio").forEach((el) => {
      if (el.src) urls.add(el.src);
      el.querySelectorAll("source").forEach((s) => {
        if (s.src) urls.add(s.src);
      });
    });
    document.querySelectorAll("a[href]").forEach((a) => {
      if (/\.(mp4|mkv|webm|mp3|flac|m4a|mov|avi|pdf|zip|rar|7z|exe|iso|apk)(\?|#|$)/i.test(a.href)) {
        urls.add(a.href);
      }
    });
    return [...urls].filter((u) => /^https?:/i.test(u));
  }

  browser.runtime.onMessage.addListener((msg) => {
    if (msg && msg.type === "grab-media") {
      const urls = collectMedia();
      if (urls.length) browser.runtime.sendMessage({ type: "media-list", urls });
    }
  });
})();
