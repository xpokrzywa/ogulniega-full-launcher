#!/usr/bin/env python3
"""Generuje banner do README (docs/banner.png) w stylu blokowym.

Wymaga: Pillow oraz plików www/fonts/PressStart2P-Regular.ttf (SIL OFL).
Użycie: python3 tools/make-banner.py
"""
import os
from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PIXEL = os.path.join(ROOT, "www", "fonts", "PressStart2P-Regular.ttf")
SANS = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
BOLD = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"

W, H = 1280, 500


def load(path, size):
    try:
        return ImageFont.truetype(path, size)
    except OSError:
        return ImageFont.load_default()


def stone_background(img):
    d = ImageDraw.Draw(img)
    d.rectangle([0, 0, W, H], fill=(28, 28, 28))
    # pas zieleni u góry (trawa) i ziemi
    d.rectangle([0, 0, W, 96], fill=(47, 127, 40))
    d.rectangle([0, 96, W, 104], fill=(88, 194, 74))
    d.rectangle([0, 104, W, H], fill=(43, 43, 43))
    # pikselowa faktura
    for y in range(104, H, 8):
        for x in range(0, W, 8):
            if (x // 8 + y // 8) % 7 == 0:
                d.rectangle([x, y, x + 8, y + 8], fill=(48, 48, 48))
    for y in range(0, 96, 8):
        for x in range(0, W, 8):
            if (x // 8 * 3 + y // 8) % 9 == 0:
                d.rectangle([x, y, x + 8, y + 8], fill=(38, 105, 32))


def grass_block(img, x, y, size):
    d = ImageDraw.Draw(img)
    side = size
    unit = side // 8
    d.rectangle([x, y, x + side, y + side], fill=(122, 90, 58))
    d.rectangle([x, y, x + side, y + unit * 3], fill=(88, 194, 74))
    for row in range(8):
        for col in range(8):
            px, py = x + col * unit, y + row * unit
            if row < 3 and (row + col) % 5 == 0:
                d.rectangle([px, py, px + unit, py + unit], fill=(47, 127, 40))
            elif row >= 3 and (row * 3 + col) % 6 == 0:
                d.rectangle([px, py, px + unit, py + unit], fill=(92, 66, 40))
    d.rectangle([x, y, x + side, y + side], outline=(10, 10, 10), width=6)


def main():
    img = Image.new("RGB", (W, H), (28, 28, 28))
    stone_background(img)
    d = ImageDraw.Draw(img)

    grass_block(img, W - 300, H // 2 - 130, 240)

    title = load(PIXEL, 46)
    sub = load(SANS, 30)
    small = load(BOLD, 22)
    tiny = load(SANS, 20)

    d.text((70, 170), "OGULNIEGA", font=title, fill=(244, 197, 66))
    d.text((70, 236), "MOBILE", font=title, fill=(242, 242, 242))
    d.text((72, 320), "Wersje klienta, mody i Modrinth na Androidzie", font=sub, fill=(220, 220, 220))
    d.text((72, 366), "PWA  •  APK (GitHub Actions)  •  działa offline", font=small, fill=(88, 194, 74))
    d.text((72, 424), "Nieoficjalna aplikacja społecznościowa — nie zawiera plików gry.", font=tiny, fill=(160, 160, 160))

    out = os.path.join(ROOT, "docs", "banner.png")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    img.save(out)
    print("zapisano", out, img.size)


if __name__ == "__main__":
    main()
