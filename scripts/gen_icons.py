#!/usr/bin/env python3
"""Generate the IDM No Ads icon set (download arrow + bolt motif).

Produces:
  - src-tauri/icons/*  (Tauri desktop icons, incl. multi-size .ico)
  - chromium-extension/icons/*  (16/48/128 png)
  - firefox-extension/icons/*   (16/48/128 png)
  - assets/icon-1024.png        (master / store listing)
"""
import math
import os
from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Brand palette: deep indigo -> electric blue, white download arrow with a bolt cut.
TOP = (37, 99, 235)      # blue-600
BOTTOM = (29, 78, 216)   # blue-700
ARROW = (255, 255, 255)
BOLT = (250, 204, 21)    # amber-400 accent


def rounded_gradient(size):
    """Rounded-square background with a vertical gradient."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    grad = Image.new("RGBA", (size, size))
    for y in range(size):
        t = y / max(1, size - 1)
        r = int(TOP[0] + (BOTTOM[0] - TOP[0]) * t)
        g = int(TOP[1] + (BOTTOM[1] - TOP[1]) * t)
        b = int(TOP[2] + (BOTTOM[2] - TOP[2]) * t)
        for x in range(size):
            grad.putpixel((x, y), (r, g, b, 255))
    mask = Image.new("L", (size, size), 0)
    md = ImageDraw.Draw(mask)
    radius = int(size * 0.22)
    md.rounded_rectangle([0, 0, size - 1, size - 1], radius=radius, fill=255)
    img.paste(grad, (0, 0), mask)
    return img


def draw_arrow(img):
    """White downward arrow with a lightning-bolt notch, over a baseline tray."""
    size = img.width
    d = ImageDraw.Draw(img)
    cx = size / 2
    # Vertical shaft
    shaft_w = size * 0.16
    shaft_top = size * 0.20
    shaft_bot = size * 0.52
    d.rectangle([cx - shaft_w / 2, shaft_top, cx + shaft_w / 2, shaft_bot], fill=ARROW)
    # Arrowhead (triangle pointing down)
    head_w = size * 0.40
    head_top = size * 0.46
    head_bot = size * 0.70
    d.polygon(
        [(cx - head_w / 2, head_top), (cx + head_w / 2, head_top), (cx, head_bot)],
        fill=ARROW,
    )
    # Baseline tray (download target)
    tray_y = size * 0.78
    tray_h = size * 0.075
    d.rounded_rectangle(
        [size * 0.24, tray_y, size * 0.76, tray_y + tray_h],
        radius=int(tray_h / 2),
        fill=ARROW,
    )
    # Bolt accent cut into the shaft
    bolt = [
        (cx - size * 0.015, shaft_top + size * 0.02),
        (cx - size * 0.07, shaft_top + size * 0.16),
        (cx - size * 0.01, shaft_top + size * 0.16),
        (cx - size * 0.05, shaft_top + size * 0.30),
        (cx + size * 0.07, shaft_top + size * 0.12),
        (cx + size * 0.005, shaft_top + size * 0.12),
        (cx + size * 0.05, shaft_top + size * 0.02),
    ]
    d.polygon(bolt, fill=BOLT)
    return img


def base_icon(size):
    img = rounded_gradient(size)
    draw_arrow(img)
    return img


def save_png(img, path):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    img.save(path, "PNG")
    print("wrote", os.path.relpath(path, ROOT))


def main():
    master = base_icon(1024)
    save_png(master, os.path.join(ROOT, "assets", "icon-1024.png"))

    tauri_dir = os.path.join(ROOT, "src-tauri", "icons")
    # Tauri desktop icon set
    for name, sz in [
        ("32x32.png", 32),
        ("128x128.png", 128),
        ("128x128@2x.png", 256),
        ("icon.png", 512),
        ("Square30x30Logo.png", 30),
        ("Square44x44Logo.png", 44),
        ("Square71x71Logo.png", 71),
        ("Square89x89Logo.png", 89),
        ("Square107x107Logo.png", 107),
        ("Square142x142Logo.png", 142),
        ("Square150x150Logo.png", 150),
        ("Square284x284Logo.png", 284),
        ("Square310x310Logo.png", 310),
        ("StoreLogo.png", 50),
    ]:
        save_png(base_icon(sz), os.path.join(tauri_dir, name))

    # Multi-resolution Windows .ico
    ico = base_icon(256)
    ico_path = os.path.join(tauri_dir, "icon.ico")
    ico.save(ico_path, sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    print("wrote", os.path.relpath(ico_path, ROOT))

    # Tray icon (template-ish, reuse master scaled)
    save_png(base_icon(32), os.path.join(tauri_dir, "tray.png"))

    # Browser extension icons
    for ext in ("chromium-extension", "firefox-extension"):
        for sz in (16, 48, 128):
            save_png(base_icon(sz), os.path.join(ROOT, ext, "icons", f"icon-{sz}.png"))


if __name__ == "__main__":
    main()
