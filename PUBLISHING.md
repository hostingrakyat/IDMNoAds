# Publishing the IDM No Ads browser extensions

> ## ⚠️ Read this about extension IDs first
>
> **Chrome Web Store rejects the `key` field** ("The 'key' field is not allowed
> in the manifest"). The CI-built `IDMNoAds-chrome-extension.zip` already has the
> `key` **stripped**, so upload it as-is. The `key` stays in the repo only so the
> **unpacked / developer** build keeps the ID
> `ikbamigoaahjngjceemkppoimlphgmii` (which matches the native-host
> `allowed_origins`) while you test locally.
>
> **The store assigns its own ID** — you cannot choose it for store-hosted
> extensions. Chrome and Edge each assign a *different* permanent ID the moment
> you create the item (visible in the dashboard URL, even before approval).
>
> **So the flow is:**
> 1. Upload the zip → copy the assigned ID from the dashboard.
> 2. Add that ID to the native host's `allowed_origins` (see
>    [After publishing](#after-publishing-update-the-native-host-ids) below) and
>    cut a new release so store-installed users get working native messaging.
>
> Firefox is the exception: the `gecko.id` (`idmnoads@hostingrakyat`) you set
> **is** the permanent ID, so no post-publish change is needed for Firefox.

## Assets you need for every store

- Icons: 128×128 PNG (also 48×48, 16×16) — in `*/icons/`.
- At least one screenshot (1280×800 or 640×400).
- Short description (≤132 chars), e.g.
  *"Free, no-ads download manager integration. Capture downloads with multi-connection speed."*
- Detailed description (reuse the README).
- Privacy policy URL — host [`PRIVACY.md`](PRIVACY.md) on GitHub Pages or
  `bisdig.xyz`.
- Category: **Productivity** or **Tools**.

Use the zips produced by CI: `IDMNoAds-chrome-extension.zip` and
`IDMNoAds-firefox-extension.zip`.

## Chrome Web Store — $5 one-time fee

1. <https://chrome.google.com/webstore/devconsole> → pay the $5 registration.
2. **New Item** → upload `IDMNoAds-chrome-extension.zip`.
3. Fill name, description, category, screenshots, icon.
4. Add the privacy-policy URL.
5. Distribution: *All regions*.
6. **Submit for review** (1–7 business days).
7. Copy the **Item ID** from the dashboard URL (e.g.
   `chrome.google.com/webstore/devconsole/.../<THIS-IS-THE-ID>`) — you need it
   for the [native-host update](#after-publishing-update-the-native-host-ids).

## Microsoft Edge Add-ons — free

1. <https://partner.microsoft.com/dashboard/microsoftedge> → sign in.
2. **Create new extension** → upload the **same** Chromium zip.
3. Fill details → **Submit** (1–3 business days).
4. Users install from <https://microsoftedge.microsoft.com/addons>.

## Firefox Add-ons (AMO) — free

1. <https://addons.mozilla.org/developers/> → sign in.
2. **Submit a New Add-on** → *On this site* → upload
   `IDMNoAds-firefox-extension.zip`.
3. AMO auto-signs it in seconds (the `gecko.id` becomes the permanent ID —
   it must stay `idmnoads@hostingrakyat`).
4. Fill listing details. Listing review takes 1–10 days, but it's installable
   immediately after signing.

## Opera Add-ons — free

1. <https://addons.opera.com/developer/> → sign in.
2. Upload the **same** Chromium zip → **Submit** (3–7 days).

## Brave

No separate store — Brave users install from the Chrome Web Store. Nothing to do.

## After publishing: update the native-host IDs

The Chrome and Edge stores assign their own IDs. Once you have them, add them to
the native-messaging host's `allowed_origins` so store-installed extensions are
authorised to talk to the desktop app. Edit **two** places (both list the same
IDs) and cut a new release:

**1. `src-tauri/nsis/hooks.nsi`** — the JSON written for Chrome/Edge/Brave.
Change the single ID in the `allowed_origins` line to a list, e.g.:

```nsi
FileWrite $0 '  "allowed_origins": [ "chrome-extension://ikbamigoaahjngjceemkppoimlphgmii/", "chrome-extension://<CHROME_STORE_ID>/", "chrome-extension://<EDGE_STORE_ID>/" ]$\r$\n'
```

**2. `src-tauri/src/lib.rs`** — `register_native_host()` writes the same JSON at
runtime (for MSIX installs). Update its `allowed_origins` array the same way.

Keep `ikbamigoaahjngjceemkppoimlphgmii` in the list so unpacked/dev builds keep
working. Firefox needs no change — the `gecko.id` you set **is** its permanent
ID.

> Tip: the store IDs are visible in the dashboard URL as soon as you create the
> item — you don't have to wait for approval to grab them.
