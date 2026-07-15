#!/usr/bin/env python3
"""Build deterministic Lid Awake app and menu-bar icon masters."""

from __future__ import annotations

import argparse
from io import BytesIO
from pathlib import Path

from PIL import Image, ImageDraw, ImageOps
from PIL.PngImagePlugin import PngInfo


MASTER_SIZE = 1024
TRAY_SIZE = 36
PROHIBITION_RED = (229, 57, 53, 255)


def save_srgb_png(image: Image.Image, path: Path) -> None:
    output = BytesIO()
    png_info = PngInfo()
    png_info.add(b"sRGB", b"\x00")
    image.save(output, format="PNG", pnginfo=png_info, optimize=True)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(output.getvalue())


def build_app_master(source_path: Path, output_path: Path) -> None:
    with Image.open(source_path) as source:
        master = ImageOps.fit(
            source.convert("RGBA"),
            (MASTER_SIZE, MASTER_SIZE),
            method=Image.Resampling.LANCZOS,
        )

    scale = 4
    overlay = Image.new("RGBA", (MASTER_SIZE * scale, MASTER_SIZE * scale), (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)
    ring_box = tuple(value * scale for value in (136, 136, 888, 888))
    stroke = 64 * scale
    draw.ellipse(ring_box, outline=PROHIBITION_RED, width=stroke)
    draw.line(
        tuple(value * scale for value in (229, 229, 795, 795)),
        fill=PROHIBITION_RED,
        width=stroke,
    )
    overlay = overlay.resize((MASTER_SIZE, MASTER_SIZE), Image.Resampling.LANCZOS)
    master = Image.alpha_composite(master, overlay)
    save_srgb_png(master, output_path)


def build_tray_icon(source_path: Path, output_path: Path) -> None:
    with Image.open(source_path) as source:
        rgba = source.convert("RGBA")

    alpha = Image.new("L", rgba.size)
    alpha_values: list[int] = []
    for red, green, blue, source_alpha in rgba.get_flattened_data():
        green_dominance = green - max(red, blue)
        ratio = max(0.0, min(1.0, (green_dominance - 20.0) / 160.0))
        smooth_ratio = ratio * ratio * (3.0 - 2.0 * ratio)
        alpha_values.append(round(source_alpha * (1.0 - smooth_ratio)))
    alpha.putdata(alpha_values)

    bounds = alpha.getbbox()
    if bounds is None:
        raise ValueError("Tray source has no visible pixels")

    alpha = alpha.crop(bounds)
    alpha.thumbnail((TRAY_SIZE - 2, TRAY_SIZE - 2), Image.Resampling.LANCZOS)
    tray = Image.new("RGBA", (TRAY_SIZE, TRAY_SIZE), (0, 0, 0, 0))
    glyph = Image.new("RGBA", alpha.size, (0, 0, 0, 255))
    glyph.putalpha(alpha)
    tray.alpha_composite(
        glyph,
        ((TRAY_SIZE - glyph.width) // 2, (TRAY_SIZE - glyph.height) // 2),
    )
    save_srgb_png(tray, output_path)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--app-source", type=Path, required=True)
    parser.add_argument("--tray-source", type=Path, required=True)
    parser.add_argument("--app-out", type=Path, required=True)
    parser.add_argument("--tray-out", type=Path, required=True)
    args = parser.parse_args()

    build_app_master(args.app_source, args.app_out)
    build_tray_icon(args.tray_source, args.tray_out)


if __name__ == "__main__":
    main()
