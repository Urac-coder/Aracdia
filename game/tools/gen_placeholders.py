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
    write_png_rect(path, size, size, pixels)


def write_png_rect(path: Path, width: int, height: int, pixels: PixelFn) -> None:
    """Write a `width`x`height` RGBA PNG. `pixels(x, y, w, h)` returns (r,g,b,a)."""
    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(
        ">IIBBBBB",
        width,
        height,
        8,  # bit depth
        6,  # colour type: RGBA
        0,
        0,
        0,
    )
    raw = bytearray()
    for y in range(height):
        raw.append(0)  # filter type: None
        for x in range(width):
            r, g, b, a = pixels(x, y, width, height)
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
# In-game UI textures (aracdia_menu mod) — launcher-aligned indigo palette
# ---------------------------------------------------------------------------

def _inside_rounded_rect(x: int, y: int, w: int, h: int, radius: int) -> bool:
    if x < 0 or y < 0 or x >= w or y >= h:
        return False
    r = min(radius, w // 2, h // 2)
    if r <= 0:
        return True
    if r <= x < w - r and r <= y < h - r:
        return True
    if x < r and r <= y < h - r:
        return True
    if w - r <= x and r <= y < h - r:
        return True
    if r <= x < w - r and y < r:
        return True
    if r <= x < w - r and y >= h - r:
        return True
    for cx, cy in ((r, r), (w - r - 1, r), (r, h - r - 1), (w - r - 1, h - r - 1)):
        if (x - cx) ** 2 + (y - cy) ** 2 <= r * r:
            return True
    return False


def _edge_distance(x: int, y: int, w: int, h: int, radius: int) -> int:
    """Chebyshev-ish distance to the rounded-rect border (0 = on edge)."""
    if not _inside_rounded_rect(x, y, w, h, radius):
        return -1
    r = min(radius, w // 2, h // 2)
    return min(x, y, w - 1 - x, h - 1 - y)


def _lerp(a: int, b: int, t: float) -> int:
    return int(a * (1 - t) + b * t)


def _lerp_rgba(a: RGBA, b: RGBA, t: float) -> RGBA:
    return tuple(_lerp(a[i], b[i], t) for i in range(4))  # type: ignore[return-value]


def ui_rounded(
    fill: RGBA,
    *,
    radius: int = 6,
    border: RGBA | None = None,
    border_px: int = 1,
    top_tint: RGBA | None = None,
    bottom_tint: RGBA | None = None,
    jitter: int = 0,
    seed: int = 0,
) -> PixelFn:
    """Rounded rectangle for 9-slice UI panels and buttons."""

    def fn(x: int, y: int, w: int, h: int) -> RGBA:
        if not _inside_rounded_rect(x, y, w, h, radius):
            return (0, 0, 0, 0)
        t = y / max(1, h - 1)
        base = fill
        if top_tint and bottom_tint:
            base = _lerp_rgba(top_tint, bottom_tint, t)
        elif top_tint and t < 0.45:
            base = _lerp_rgba(top_tint, fill, t / 0.45)
        if jitter:
            n = ((x * 92837111) ^ (y * 689287499) ^ (seed * 283923481)) & 0xFFFF
            bias = int(((n / 0xFFFF) * 2 - 1) * jitter)
            base = (
                max(0, min(255, base[0] + bias)),
                max(0, min(255, base[1] + bias)),
                max(0, min(255, base[2] + bias)),
                base[3],
            )
        if border and border_px > 0:
            dist = _edge_distance(x, y, w, h, radius)
            if 0 <= dist < border_px:
                mix = 1 - (dist / max(1, border_px))
                return _lerp_rgba(base, border, mix)
        return base

    return fn


# Launcher tokens: bg-surface #12121a, accent #6366f1, success #10b981, danger #ef4444
UI_PANEL = ui_rounded(
    (18, 18, 26, 255),
    radius=8,
    border=(99, 102, 241, 90),
    border_px=2,
    jitter=6,
    seed=40,
)
UI_HEADER = lambda x, y, w, h: (  # noqa: E731
    (99, 102, 241, 220)
    if y < h // 2
    else (79, 70, 229, 180)
    if _inside_rounded_rect(x, y, w, h, 3)
    else (0, 0, 0, 0)
)
UI_DIVIDER = lambda x, y, w, h: (42, 42, 56, 255) if y == h // 2 else (0, 0, 0, 0)  # noqa: E731

UI_BTN_PRIMARY = ui_rounded(
    (99, 102, 241, 255),
    radius=5,
    top_tint=(129, 140, 248, 255),
    bottom_tint=(79, 70, 229, 255),
)
UI_BTN_PRIMARY_H = ui_rounded(
    (129, 140, 248, 255),
    radius=5,
    top_tint=(165, 180, 252, 255),
    bottom_tint=(99, 102, 241, 255),
)
UI_BTN_PRIMARY_P = ui_rounded(
    (67, 56, 202, 255),
    radius=5,
    top_tint=(79, 70, 229, 255),
    bottom_tint=(55, 48, 163, 255),
)

UI_BTN_SECONDARY = ui_rounded(
    (26, 26, 37, 255),
    radius=5,
    border=(42, 42, 56, 255),
    border_px=1,
    jitter=4,
    seed=41,
)
UI_BTN_SECONDARY_H = ui_rounded(
    (34, 34, 46, 255),
    radius=5,
    border=(99, 102, 241, 120),
    border_px=1,
)

UI_BTN_SUCCESS = ui_rounded(
    (16, 46, 36, 255),
    radius=5,
    border=(16, 185, 129, 180),
    border_px=1,
    top_tint=(22, 78, 58, 255),
    bottom_tint=(12, 36, 28, 255),
)
UI_BTN_SUCCESS_H = ui_rounded(
    (20, 58, 44, 255),
    radius=5,
    border=(52, 211, 153, 200),
    border_px=1,
)

UI_BTN_DANGER = ui_rounded(
    (36, 18, 22, 255),
    radius=5,
    border=(239, 68, 68, 140),
    border_px=1,
)
UI_BTN_DANGER_H = ui_rounded(
    (52, 22, 28, 255),
    radius=5,
    border=(248, 113, 113, 180),
    border_px=1,
)

UI_BTN_DISABLED = ui_rounded(
    (16, 16, 22, 255),
    radius=5,
    border=(34, 34, 44, 255),
    border_px=1,
)

UI_TEXTURES: dict[str, tuple[int, int, PixelFn]] = {
    "aracdia_ui_panel.png": (48, 48, UI_PANEL),
    "aracdia_ui_header.png": (64, 10, UI_HEADER),
    "aracdia_ui_divider.png": (32, 4, UI_DIVIDER),
    "aracdia_ui_btn_primary.png": (32, 32, UI_BTN_PRIMARY),
    "aracdia_ui_btn_primary_h.png": (32, 32, UI_BTN_PRIMARY_H),
    "aracdia_ui_btn_primary_p.png": (32, 32, UI_BTN_PRIMARY_P),
    "aracdia_ui_btn_secondary.png": (32, 32, UI_BTN_SECONDARY),
    "aracdia_ui_btn_secondary_h.png": (32, 32, UI_BTN_SECONDARY_H),
    "aracdia_ui_btn_success.png": (32, 32, UI_BTN_SUCCESS),
    "aracdia_ui_btn_success_h.png": (32, 32, UI_BTN_SUCCESS_H),
    "aracdia_ui_btn_danger.png": (32, 32, UI_BTN_DANGER),
    "aracdia_ui_btn_danger_h.png": (32, 32, UI_BTN_DANGER_H),
    "aracdia_ui_btn_disabled.png": (32, 32, UI_BTN_DISABLED),
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

    ui_dir = game_root / "mods" / "aracdia_menu" / "textures"
    for name, (width, height, fn) in UI_TEXTURES.items():
        write_png_rect(ui_dir / name, width, height, fn)
        print(f"  ui:      {ui_dir / name}")

    print(f"Done. Wrote {len(TEXTURES) + 3 + len(UI_TEXTURES)} placeholder PNGs.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
