#!/usr/bin/env python3
"""Generate transparent tray variants from the WorkBuddyBar favicon."""
from io import BytesIO
from pathlib import Path

from PIL import Image, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "src-tauri" / "icons" / "app-source.png"
OUTPUT = ROOT / "src-tauri" / "icons"


def make_icon(accent: tuple[int, int, int], filename: str) -> None:
    image = Image.open(SOURCE).convert("RGBA")
    pixels = image.load()
    width, height = image.size

    foreground = Image.new("L", image.size, 0)
    foreground_pixels = foreground.load()
    accent_mask = Image.new("L", image.size, 0)
    accent_pixels = accent_mask.load()

    for y in range(height):
        for x in range(width):
            r, g, b, a = pixels[x, y]
            if a == 0:
                continue
            is_background = r < 55 and g < 55 and b < 55
            if not is_background:
                foreground_pixels[x, y] = 255
                if r > 180 and 55 < g < 180 and b < 110:
                    accent_pixels[x, y] = 255

    bbox = foreground.getbbox()
    if bbox is None:
        raise SystemExit("no foreground found")

    pad = 10
    bbox = (
        max(0, bbox[0] - pad),
        max(0, bbox[1] - pad),
        min(width, bbox[2] + pad),
        min(height, bbox[3] + pad),
    )
    foreground = foreground.crop(bbox)
    accent_mask = accent_mask.crop(bbox)

    # The source has a white robot on a near-black rounded square. Use the
    # silhouette as a mask so the menu bar background remains visible.
    outline_mask = foreground.filter(ImageFilter.MaxFilter(7))
    layered = Image.new("RGBA", foreground.size, (0, 0, 0, 0))
    outline_layer = Image.new("RGBA", foreground.size, (24, 24, 27, 255))
    body_layer = Image.new("RGBA", foreground.size, (246, 246, 248, 255))
    accent_layer = Image.new("RGBA", foreground.size, (*accent, 255))
    layered = Image.composite(outline_layer, layered, outline_mask)
    layered = Image.composite(body_layer, layered, foreground)
    layered = Image.composite(accent_layer, layered, accent_mask)

    canvas = Image.new("RGBA", (32, 32), (0, 0, 0, 0))
    scale = min(28 / layered.width, 28 / layered.height)
    resized = layered.resize(
        (max(1, round(layered.width * scale)), max(1, round(layered.height * scale))),
        Image.Resampling.LANCZOS,
    )
    canvas.alpha_composite(resized, ((32 - resized.width) // 2, (32 - resized.height) // 2))
    canvas.save(OUTPUT / filename)


make_icon((245, 158, 11), "tray-normal.png")
make_icon((250, 204, 21), "tray-warn.png")
make_icon((239, 68, 68), "tray-error.png")
