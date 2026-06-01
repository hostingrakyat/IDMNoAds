# Publishing the IDM No Ads browser extensions

> **Pin the IDs first.** The native-messaging host manifest lists
> `allowed_origins` (Chromium) and `allowed_extensions` (Firefox). They must
> match the **final published** extension IDs. They are already pinned in this
> repo and must never change:
>
> - Chromium ID: `ikbamigoaahjngjceemkppoimlphgmii` — pinned via the `key`
>   field in `chromium-extension/manifest.json` (the private key is
>   `chromium-extension/key.pem`; **keep it secret, never publish it**).
> - Firefox ID: `idmnoads@hostingrakyat` — pinned via
>   `browser_specific_settings.gecko.id`.

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
7. After approval the permanent ID will match the pinned `key` —
   `ikbamigoaahjngjceemkppoimlphgmii`. No installer change needed.

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

## After publishing

Verify the Chrome dashboard ID equals `ikbamigoaahjngjceemkppoimlphgmii`. If you
pinned the key correctly it matches and the installer needs no changes. For
Firefox the `gecko.id` you set **is** the ID — no surprises.
