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


def _lerp(a: int, b: int, t: float) -> int:
    return int(a * (1 - t) + b * t)


def _lerp_rgba(a: RGBA, b: RGBA, t: float) -> RGBA:
    return tuple(_lerp(a[i], b[i], t) for i in range(4))  # type: ignore[return-value]


# ---------------------------------------------------------------------------
# In-game UI textures (aracdia_menu mod)
#
# Smooth SDF-based assets at high resolution. Low-res pixel 9-slices looked
# awful once scaled by the engine — these are anti-aliased and noise-free.
# ---------------------------------------------------------------------------

def _sdf_rounded_rect(px: float, py: float, w: float, h: float, r: float) -> float:
    """Signed distance to a rounded-rect border (negative = inside)."""
    r = min(r, w / 2, h / 2)
    cx = max(r, min(px, w - r))
    cy = max(r, min(py, h - r))
    dx = px - cx
    dy = py - cy
    dist_corner = (dx * dx + dy * dy) ** 0.5 - r
    if r <= px <= w - r and py < r:
        return py - r
    if r <= px <= w - r and py > h - r:
        return h - r - py
    if r <= py <= h - r and px < r:
        return px - r
    if r <= py <= h - r and px > w - r:
        return w - r - px
    return dist_corner


def _aa_alpha(dist: float) -> int:
    """Convert SDF distance to 8-bit alpha (1px soft edge)."""
    if dist >= 1.0:
        return 0
    if dist <= -1.0:
        return 255
    return int(max(0, min(255, (-dist + 0.5) * 255)))


def _vertical_gradient(y: int, h: int, top: RGBA, bottom: RGBA) -> RGBA:
    t = y / max(1, h - 1)
    return _lerp_rgba(top, bottom, t)


def ui_sdf_fill(
    *,
    radius: float,
    fill_top: RGBA,
    fill_bottom: RGBA,
    border: RGBA | None = None,
    border_width: float = 1.0,
    glow_top: RGBA | None = None,
) -> PixelFn:
    """Anti-aliased rounded rectangle."""

    def fn(x: int, y: int, w: int, h: int) -> RGBA:
        px = x + 0.5
        py = y + 0.5
        dist = _sdf_rounded_rect(px, py, w, h, radius)
        if dist >= 1.0:
            return (0, 0, 0, 0)

        base = _vertical_gradient(y, h, fill_top, fill_bottom)
        if glow_top and py < h * 0.35:
            t = 1.0 - (py / max(1.0, h * 0.35))
            base = _lerp_rgba(base, glow_top, t * 0.35)

        if border and 0.0 <= dist < border_width:
            mix = 1.0 - (dist / max(0.001, border_width))
            base = _lerp_rgba(base, border, mix)

        alpha = _aa_alpha(dist)
        return (base[0], base[1], base[2], alpha)

    return fn


def ui_logo_mark() -> PixelFn:
    """Launcher-style gradient tile with a white A."""

    def fn(x: int, y: int, w: int, h: int) -> RGBA:
        px = x + 0.5
        py = y + 0.5
        cx, cy = w / 2, h / 2
        radius = min(w, h) * 0.42
        dist = ((px - cx) ** 2 + (py - cy) ** 2) ** 0.5 - radius
        if dist >= 1.5:
            return (0, 0, 0, 0)
        t = py / max(1, h - 1)
        fill = _lerp_rgba((99, 102, 241, 255), (124, 58, 237, 255), t)
        alpha = _aa_alpha(dist)
        # Simple A glyph in the centre.
        bitmap = _bitmap_char("A")
        gh, gw = 7, 5
        scale = max(2, min(w, h) // 10)
        gx0 = int(cx - (gw * scale) / 2)
        gy0 = int(cy - (gh * scale) / 2)
        if gx0 <= x < gx0 + gw * scale and gy0 <= y < gy0 + gh * scale:
            bx = (x - gx0) // scale
            by = (y - gy0) // scale
            if 0 <= by < gh and 0 <= bx < gw and bitmap[by] & (1 << (gw - 1 - bx)):
                return (255, 255, 255, alpha)
        return (fill[0], fill[1], fill[2], alpha)

    return fn


def ui_pill(active: bool) -> PixelFn:
    color = (16, 185, 129, 255) if active else (74, 74, 90, 255)
    border = (52, 211, 153, 255) if active else (106, 106, 120, 255)

    return ui_sdf_fill(
        radius=6,
        fill_top=color,
        fill_bottom=color,
        border=border,
        border_width=0.8,
    )


# Dark glass shell — matches launcher `--color-bg-surface`.
UI_SHELL = ui_sdf_fill(
    radius=18,
    fill_top=(26, 26, 38, 248),
    fill_bottom=(14, 14, 20, 252),
    border=(58, 58, 74, 180),
    border_width=1.2,
    glow_top=(99, 102, 241, 48),
)

UI_BTN_PRIMARY = ui_sdf_fill(
    radius=10,
    fill_top=(129, 140, 248, 255),
    fill_bottom=(79, 70, 229, 255),
    border=(165, 180, 252, 120),
    border_width=0.8,
)
UI_BTN_PRIMARY_H = ui_sdf_fill(
    radius=10,
    fill_top=(165, 180, 252, 255),
    fill_bottom=(99, 102, 241, 255),
    border=(199, 210, 254, 160),
    border_width=0.8,
)
UI_BTN_PRIMARY_P = ui_sdf_fill(
    radius=10,
    fill_top=(79, 70, 229, 255),
    fill_bottom=(67, 56, 202, 255),
    border=(99, 102, 241, 140),
    border_width=0.8,
)

UI_BTN_GHOST = ui_sdf_fill(
    radius=10,
    fill_top=(34, 34, 48, 230),
    fill_bottom=(24, 24, 34, 240),
    border=(52, 52, 68, 200),
    border_width=1.0,
)
UI_BTN_GHOST_H = ui_sdf_fill(
    radius=10,
    fill_top=(44, 44, 62, 240),
    fill_bottom=(30, 30, 44, 250),
    border=(99, 102, 241, 160),
    border_width=1.0,
)

UI_BTN_ACTIVE = ui_sdf_fill(
    radius=10,
    fill_top=(22, 64, 52, 240),
    fill_bottom=(14, 42, 34, 250),
    border=(16, 185, 129, 200),
    border_width=1.0,
)
UI_BTN_ACTIVE_H = ui_sdf_fill(
    radius=10,
    fill_top=(28, 82, 64, 250),
    fill_bottom=(18, 52, 42, 255),
    border=(52, 211, 153, 220),
    border_width=1.0,
)

UI_BTN_DISABLED = ui_sdf_fill(
    radius=10,
    fill_top=(22, 22, 30, 180),
    fill_bottom=(16, 16, 22, 200),
    border=(40, 40, 52, 140),
    border_width=0.8,
)

UI_SLOT = ui_sdf_fill(
    radius=6,
    fill_top=(34, 34, 48, 235),
    fill_bottom=(22, 22, 32, 245),
    border=(58, 58, 74, 150),
    border_width=0.9,
)

def _with_noise(base_fn: PixelFn, jitter: int = 10, seed: int = 7) -> PixelFn:
    """Subtle parchment grain on top of an SDF fill."""

    def fn(x: int, y: int, w: int, h: int) -> RGBA:
        c = base_fn(x, y, w, h)
        if c[3] < 8:
            return c
        n = ((x * 92837111) ^ (y * 689287499) ^ (seed * 283923481)) & 0xFFFF
        bias = ((n / 0xFFFF) * 2 - 1) * jitter
        return (
            max(0, min(255, int(c[0] + bias))),
            max(0, min(255, int(c[1] + bias * 0.8))),
            max(0, min(255, int(c[2] + bias * 0.6))),
            c[3],
        )

    return fn


def ui_inv_portrait() -> PixelFn:
    """Circular portrait frame — Albion-style character medallion."""

    def fn(x: int, y: int, w: int, h: int) -> RGBA:
        px = x + 0.5
        py = y + 0.5
        cx, cy = w / 2, h / 2
        outer = min(w, h) * 0.46
        inner = outer - 5.0
        dist_outer = ((px - cx) ** 2 + (py - cy) ** 2) ** 0.5 - outer
        dist_inner = ((px - cx) ** 2 + (py - cy) ** 2) ** 0.5 - inner

        if dist_outer >= 1.0:
            return (0, 0, 0, 0)

        ring = dist_inner >= 0.0
        if ring:
            t = py / max(1, h - 1)
            fill = _lerp_rgba((120, 92, 58, 255), (74, 56, 36, 255), t)
            alpha = _aa_alpha(dist_outer)
            if dist_inner < 1.0 and dist_inner >= 0.0:
                mix = 1.0 - dist_inner
                hi = (196, 165, 116, 255)
                fill = _lerp_rgba(fill, hi, mix * 0.55)
            return (fill[0], fill[1], fill[2], alpha)

        t = py / max(1, h - 1)
        fill = _lerp_rgba((72, 62, 52, 255), (48, 42, 36, 255), t)
        alpha = _aa_alpha(dist_inner)
        return (fill[0], fill[1], fill[2], alpha)

    return fn


UI_INV_PANEL = _with_noise(
    ui_sdf_fill(
        radius=10,
        fill_top=(196, 165, 116, 255),
        fill_bottom=(139, 115, 85, 255),
        border=(61, 47, 31, 255),
        border_width=2.2,
    ),
    jitter=12,
    seed=11,
)

UI_INV_SLOT = ui_sdf_fill(
    radius=4,
    fill_top=(48, 44, 40, 255),
    fill_bottom=(28, 25, 22, 255),
    border=(95, 84, 68, 200),
    border_width=1.2,
)

UI_INV_BTN = ui_sdf_fill(
    radius=6,
    fill_top=(139, 115, 85, 255),
    fill_bottom=(107, 87, 64, 255),
    border=(61, 47, 31, 200),
    border_width=1.0,
)

UI_INV_BTN_H = ui_sdf_fill(
    radius=6,
    fill_top=(168, 140, 104, 255),
    fill_bottom=(124, 102, 74, 255),
    border=(61, 47, 31, 220),
    border_width=1.0,
)

UI_INV_BAR = ui_sdf_fill(
    radius=3,
    fill_top=(201, 149, 43, 255),
    fill_bottom=(160, 110, 28, 255),
    border=(120, 86, 20, 180),
    border_width=0.6,
)


UI_TEXTURES: dict[str, tuple[int, int, PixelFn]] = {
    "aracdia_ui_shell.png": (128, 128, UI_SHELL),
    "aracdia_ui_logo.png": (64, 64, ui_logo_mark()),
    "aracdia_ui_slot.png": (64, 64, UI_SLOT),
    "aracdia_ui_btn_primary.png": (128, 48, UI_BTN_PRIMARY),
    "aracdia_ui_btn_primary_h.png": (128, 48, UI_BTN_PRIMARY_H),
    "aracdia_ui_btn_primary_p.png": (128, 48, UI_BTN_PRIMARY_P),
    "aracdia_ui_btn_ghost.png": (128, 48, UI_BTN_GHOST),
    "aracdia_ui_btn_ghost_h.png": (128, 48, UI_BTN_GHOST_H),
    "aracdia_ui_btn_active.png": (128, 48, UI_BTN_ACTIVE),
    "aracdia_ui_btn_active_h.png": (128, 48, UI_BTN_ACTIVE_H),
    "aracdia_ui_btn_disabled.png": (128, 48, UI_BTN_DISABLED),
    "aracdia_ui_pill_on.png": (48, 20, ui_pill(True)),
    "aracdia_ui_pill_off.png": (48, 20, ui_pill(False)),
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
