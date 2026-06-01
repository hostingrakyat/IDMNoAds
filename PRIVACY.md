# Privacy Policy — IDM No Ads

_Last updated: 2026-06-01_

IDM No Ads is a free, no-ads download manager for Windows, with companion
browser extensions. We respect your privacy completely.

## What we collect

**Nothing.** IDM No Ads has no servers, no accounts, no analytics, no telemetry,
and no advertising. We never see your data.

## What stays on your device

- **Download URLs, files, and queue** — handled locally by the bundled aria2
  engine and saved only to folders you choose.
- **Settings** — stored in a local JSON file in your Windows user profile.
- **Browser extension data** — the extension reads the URL, file name, size,
  referrer and cookies of a download **only at the moment you start it**, and
  sends them **only to the IDM No Ads app running on your own computer** (via a
  local named pipe). This information never leaves your machine and is never
  transmitted to us or any third party.

## Permissions the browser extension requests, and why

| Permission | Why |
| ---------- | --- |
| `nativeMessaging` | Talk to the local IDM No Ads desktop app. |
| `downloads` | Detect and take over downloads you start. |
| `contextMenus` | Add the "Download with IDM No Ads" right-click item. |
| `cookies` | Pass your session cookies to the engine so authenticated downloads (e.g. Google Drive) work. |
| `tabs` / `webRequest` | Detect media on the current page. |
| `storage` | Remember your extension options locally. |
| `notifications` | Tell you when a download was sent to the app. |
| `<all_urls>` | The above must work on any site you download from. |

All processing is local. Cookies and headers are used solely to complete the
download you requested and are not stored or shared.

## Children's privacy

The app is not directed at children and collects no personal data from anyone.

## Changes

Any updates to this policy will be published in this file in the repository.

## Contact

Ir. Riovan Styx Roring — riovan.roring@lecturer.itk.ac.id
