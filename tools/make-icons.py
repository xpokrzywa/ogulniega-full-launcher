#!/usr/bin/env python3
"""Generuje ikony aplikacji (PWA + Android) bez zewnętrznych zasobów.

Motyw: blok trawy w stylu Minecrafta na ciemnym tle — czytelny także w 36 px.
Użycie:  python3 tools/make-icons.py
"""
import os
from PIL import Image, ImageDraw, ImageFilter

BG = (11, 15, 12, 255)
GLOW = (34, 197, 94, 90)
GRASS = (74, 222, 128, 255)
GRASS_DARK = (34, 197, 94, 255)
DIRT = (109, 76, 44, 255)
DIRT_DARK = (82, 56, 32, 255)
DIRT_LIGHT = (128, 92, 55, 255)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def block(size, scale=1.0, rounded=True, transparent=False):
    """Rysuje ikonę; scale < 1 zostawia margines (wariant maskowalny)."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    radius = int(size * 0.22)
    if transparent:
        pass
    elif rounded:
        draw.rounded_rectangle([0, 0, size - 1, size - 1], radius=radius, fill=BG)
    else:
        draw.rectangle([0, 0, size - 1, size - 1], fill=BG)

    # poświata
    if not transparent:
        glow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
        gd = ImageDraw.Draw(glow)
        gd.ellipse([-size * 0.15, -size * 0.55, size * 1.15, size * 0.55], fill=GLOW)
        glow = glow.filter(ImageFilter.GaussianBlur(size * 0.12))
        img.alpha_composite(glow)

    if rounded and not transparent:
        mask = Image.new("L", (size, size), 0)
        ImageDraw.Draw(mask).rounded_rectangle([0, 0, size - 1, size - 1], radius=radius, fill=255)
        img.putalpha(Image.composite(img.getchannel("A"), Image.new("L", (size, size), 0), mask))

    # blok
    side = size * 0.52 * scale
    left = (size - side) / 2
    top = (size - side) / 2
    strip = side * 0.34

    draw = ImageDraw.Draw(img)
    draw.rectangle([left, top, left + side, top + side], fill=DIRT)
    draw.rectangle([left, top, left + side, top + strip], fill=GRASS)

    # piksele: faktura
    unit = side / 8.0
    for row in range(8):
        for col in range(8):
            x0 = left + col * unit
            y0 = top + row * unit
            if row < 3:
                if (row + col) % 5 == 0:
                    draw.rectangle([x0, y0, x0 + unit, y0 + unit], fill=GRASS_DARK)
            else:
                if (row * 3 + col) % 6 == 0:
                    draw.rectangle([x0, y0, x0 + unit, y0 + unit], fill=DIRT_DARK)
                elif (row + col * 2) % 7 == 0:
                    draw.rectangle([x0, y0, x0 + unit, y0 + unit], fill=DIRT_LIGHT)

    # obramowanie bloku
    draw.rectangle([left, top, left + side, top + side], outline=(0, 0, 0, 60), width=max(1, int(size * 0.006)))
    return img


def save(img, *parts):
    path = os.path.join(ROOT, *parts)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    img.save(path)
    print("zapisano", path, img.size)


def main():
    for px in (192, 512):
        save(block(px), "www", "icons", f"icon-{px}.png")

    save(block(512, scale=0.72, rounded=False), "www", "icons", "icon-maskable-512.png")
    save(block(180), "www", "icons", "apple-touch-icon.png")
    save(block(32), "www", "icons", "favicon-32.png")
    save(block(1024), "resources", "icon.png")
    save(block(1024, scale=0.8), "resources", "splash.png")

    # mipmapy Androida (nadpisywane po wygenerowaniu projektu przez `cap add android`)
    launcher = {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}
    foreground = {"mdpi": 108, "hdpi": 162, "xhdpi": 216, "xxhdpi": 324, "xxxhdpi": 432}
    base = os.path.join(ROOT, "android", "app", "src", "main", "res")
    for density, px in launcher.items():
        if not os.path.isdir(os.path.join(base, "mipmap-" + density)):
            continue
        save(block(px), "android", "app", "src", "main", "res", "mipmap-" + density, "ic_launcher.png")
        save(block(px), "android", "app", "src", "main", "res", "mipmap-" + density, "ic_launcher_round.png")
    for density, px in foreground.items():
        if not os.path.isdir(os.path.join(base, "mipmap-" + density)):
            continue
        save(block(px, scale=0.62, rounded=False, transparent=True),
             "android", "app", "src", "main", "res", "mipmap-" + density, "ic_launcher_foreground.png")


if __name__ == "__main__":
    main()
