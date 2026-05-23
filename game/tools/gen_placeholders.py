#!/usr/bin/env python3
"""Regenerate placeholder PNG textures for the Aracdia core mod and menu.

This script depends ONLY on the Python 3 standard library (no Pillow / PIL).
It writes minimal RGBA PNGs so the game has visible blocks while we wait for
real art. Run from the repository root:

    python3 game/tools/gen_placeholders.py

All files under `game/mods/aracdia_core/textures/` and `game/menu/` are
overwritten in-place — the script is the source of truth, the PNGs are
checked in for convenience so a clean clone can run the game right away.
"""

from __future__ import annotations

import os
import struct
import sys
import zlib
from pathlib import Path
from typing import Callable, Tuple

RGBA = Tuple[int, int, int, int]
PixelFn = Callable[[int, int, int, int], RGBA]


# ---------------------------------------------------------------------------
# Tiny PNG writer (RGBA, 8-bit). Hand-rolled to avoid third-party deps.
# ---------------------------------------------------------------------------

def _png_chunk(tag: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + tag
        + data
        + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    )


def write_png(path: Path, size: int, pixels: PixelFn) -> None:
    """Write a `size`x`size` RGBA PNG. `pixels(x, y, w, h)` returns (r,g,b,a)."""
    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(
        ">IIBBBBB",
        size,
        size,
        8,  # bit depth
        6,  # colour type: RGBA
        0,
        0,
        0,
    )
    raw = bytearray()
    for y in range(size):
        raw.append(0)  # filter type: None
        for x in range(size):
            r, g, b, a = pixels(x, y, size, size)
            raw.extend((r & 0xFF, g & 0xFF, b & 0xFF, a & 0xFF))
    idat = zlib.compress(bytes(raw), level=9)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as fp:
        fp.write(sig)
        fp.write(_png_chunk(b"IHDR", ihdr))
        fp.write(_png_chunk(b"IDAT", idat))
        fp.write(_png_chunk(b"IEND", b""))


# ---------------------------------------------------------------------------
# Pixel generators
# ---------------------------------------------------------------------------

def solid(rgba: RGBA) -> PixelFn:
    return lambda x, y, w, h: rgba


def noisy(base: RGBA, jitter: int = 18, seed: int = 1) -> PixelFn:
    """Stable per-pixel pseudo-noise so the texture is not perfectly flat.
    `seed` makes each material reproducibly different."""

    def fn(x: int, y: int, w: int, h: int) -> RGBA:
        # cheap deterministic hash
        n = ((x * 92837111) ^ (y * 689287499) ^ (seed * 283923481)) & 0xFFFF
        bias = ((n / 0xFFFF) * 2 - 1) * jitter
        r, g, b, a = base
        return (
            max(0, min(255, int(r + bias))),
            max(0, min(255, int(g + bias))),
            max(0, min(255, int(b + bias))),
            a,
        )

    return fn


def grass_side(grass: RGBA, dirt: RGBA, seed: int = 7) -> PixelFn:
    """Top 4 rows are grass, bottom rows are dirt — for the side faces of grass blocks."""
    grass_fn = noisy(grass, jitter=18, seed=seed)
    dirt_fn = noisy(dirt, jitter=18, seed=seed + 1)

    def fn(x: int, y: int, w: int, h: int) -> RGBA:
        return grass_fn(x, y, w, h) if y < h // 4 else dirt_fn(x, y, w, h)

    return fn


def menu_gradient(
    top: RGBA, bottom: RGBA, glyph: str = "A", glyph_color: RGBA = (255, 255, 255, 255)
) -> PixelFn:
    """Vertical gradient with a single capital letter centered (5x7 bitmap font)."""
    bitmap = _bitmap_char(glyph)

    def fn(x: int, y: int, w: int, h: int) -> RGBA:
        t = y / max(1, h - 1)
        bg = tuple(  # noqa: C400 -- legibility
            int(top[i] * (1 - t) + bottom[i] * t) for i in range(4)
        )
        # draw the letter centered
        gh = 7
        gw = 5
        scale = max(1, min(w, h) // 12)
        gx0 = (w - gw * scale) // 2
        gy0 = (h - gh * scale) // 2
        if 0 <= x - gx0 < gw * scale and 0 <= y - gy0 < gh * scale:
            bx = (x - gx0) // scale
            by = (y - gy0) // scale
            if bitmap[by] & (1 << (gw - 1 - bx)):
                return glyph_color
        return bg  # type: ignore[return-value]

    return fn


# A tiny 5x7 bitmap for the letter "A". MSB = leftmost pixel.
_FONT_5X7: dict[str, list[int]] = {
    "A": [
        0b01110,
        0b10001,
        0b10001,
        0b11111,
        0b10001,
        0b10001,
        0b10001,
    ],
}


def _bitmap_char(c: str) -> list[int]:
    return _FONT_5X7.get(c.upper(), _FONT_5X7["A"])


# ---------------------------------------------------------------------------
# Texture catalog
# ---------------------------------------------------------------------------

DIRT = (139, 90, 43, 255)
GRASS = (76, 175, 80, 255)
GRASS_DARK = (56, 142, 60, 255)
STONE = (140, 140, 140, 255)
SAND = (245, 222, 179, 255)
WATER = (33, 150, 243, 200)  # semi-transparent
WOOD = (109, 76, 65, 255)
LEAVES = (46, 125, 50, 230)


TEXTURES: dict[str, PixelFn] = {
    "aracdia_dirt.png": noisy(DIRT, jitter=22, seed=1),
    "aracdia_grass_top.png": noisy(GRASS, jitter=14, seed=2),
    "aracdia_grass_side.png": grass_side(GRASS, DIRT, seed=3),
    "aracdia_stone.png": noisy(STONE, jitter=24, seed=4),
    "aracdia_sand.png": noisy(SAND, jitter=12, seed=5),
    "aracdia_water.png": noisy(WATER, jitter=10, seed=6),
    "aracdia_wood.png": noisy(WOOD, jitter=18, seed=7),
    "aracdia_wood_top.png": noisy(WOOD, jitter=22, seed=8),
    "aracdia_leaves.png": noisy(LEAVES, jitter=26, seed=9),
}


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> int:
    here = Path(__file__).resolve()
    game_root = here.parent.parent  # game/
    tex_dir = game_root / "mods" / "aracdia_core" / "textures"
    menu_dir = game_root / "menu"

    for name, fn in TEXTURES.items():
        write_png(tex_dir / name, 16, fn)
        print(f"  texture: {tex_dir / name}")

    # Menu assets (sized to what Luanti expects)
    write_png(
        menu_dir / "icon.png", 192, menu_gradient((90, 60, 200, 255), (40, 20, 80, 255))
    )
    print(f"  menu:    {menu_dir / 'icon.png'}")
    write_png(
        menu_dir / "header.png", 256, menu_gradient((50, 30, 110, 255), (90, 60, 200, 255))
    )
    print(f"  menu:    {menu_dir / 'header.png'}")
    write_png(
        menu_dir / "background.png",
        512,
        menu_gradient((20, 12, 50, 255), (10, 6, 25, 255), glyph_color=(255, 255, 255, 32)),
    )
    print(f"  menu:    {menu_dir / 'background.png'}")

    print(f"Done. Wrote {len(TEXTURES) + 3} placeholder PNGs.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
