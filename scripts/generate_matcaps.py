#!/usr/bin/env python3
"""Renders the built-in matcap capture textures into public/matcaps/.

Each preset shades the unit sphere in "capture space" (x right, y up, z
toward the viewer) with a small procedural lighting rig, matching how
mesh_matcap.wgsl samples the texture: uv = viewNormal.xy * 0.5 + 0.5.

Usage: python3 scripts/generate_matcaps.py
"""
import math
from pathlib import Path

from PIL import Image

SIZE = 256
OUT = Path(__file__).resolve().parent.parent / "public" / "matcaps"

BACKGROUND = (10, 10, 13)


def clamp(value, lo=0.0, hi=1.0):
    return lo if value < lo else hi if value > hi else value


def each_normal():
    """Yields (x, y, z) unit normals over the disk; None outside it."""
    for row in range(SIZE):
        y = 1.0 - (row + 0.5) / SIZE * 2.0  # top row = +1 (v flips on upload)
        for column in range(SIZE):
            x = (column + 0.5) / SIZE * 2.0 - 1.0
            r2 = x * x + y * y
            if r2 > 1.0:
                yield None
            else:
                yield (x, y, math.sqrt(1.0 - r2))


def shade_studio(n):
    """Neutral studio: soft top-left key, cool fresnel rim, one spec blob."""
    x, y, z = n
    key = clamp((-0.35 * x + 0.55 * y) * 0.5 + 0.55)
    rim = (1.0 - abs(z)) ** 2.5
    blob = (x + 0.3) ** 2 + (y - 0.45) ** 2
    spec = math.exp(-blob * 18.0) * 0.6
    return tuple(clamp(0.25 + 0.75 * key + rim * 0.35 * c + spec) for c in (0.9, 0.95, 1.0))


def shade_clay(n):
    """Warm sculpting clay: wrapped diffuse, gentle top fill, muted rim."""
    x, y, z = n
    lx, ly, lz = -0.45, 0.55, 0.70
    diffuse = clamp((x * lx + y * ly + z * lz) * 0.5 + 0.5)
    fill = clamp(0.5 + 0.5 * y) * 0.12
    rim = (1.0 - abs(z)) ** 3.0 * 0.18
    base = (0.86, 0.74, 0.66)
    return tuple(clamp(base[i] * (0.35 + 0.65 * diffuse) + fill + rim) for i in range(3))


def shade_chrome(n):
    """Polished chrome: hard horizon banding and a bright key stripe."""
    x, y, z = n
    bands = 0.5 + 0.5 * math.sin(y * 9.0 + 1.2 * math.sin(x * 3.0))
    bands = bands ** 2.0
    horizon = math.exp(-((y + 0.15) ** 2) * 40.0) * 0.9
    stripe = math.exp(-(((x + 0.35) ** 2) + ((y - 0.55) ** 2)) * 10.0)
    value = clamp(0.08 + 0.55 * bands + horizon + stripe * 0.9)
    return (value * 0.96, value * 0.98, clamp(value * 1.04))


def shade_pearl(n):
    """Iridescent pearl: pastel hue drift by fresnel over a soft diffuse."""
    x, y, z = n
    fresnel = (1.0 - abs(z)) ** 1.5
    diffuse = clamp(0.55 + 0.45 * (x * -0.3 + y * 0.5 + z * 0.8))
    r = 0.72 + 0.28 * math.cos(fresnel * 4.5)
    g = 0.72 + 0.28 * math.cos(fresnel * 4.5 + 1.8)
    b = 0.78 + 0.22 * math.cos(fresnel * 4.5 + 3.4)
    return tuple(clamp(c * (0.45 + 0.55 * diffuse)) for c in (r, g, b))


PRESETS = {
    "studio": shade_studio,
    "clay": shade_clay,
    "chrome": shade_chrome,
    "pearl": shade_pearl,
}


def render(name, shade):
    image = Image.new("RGB", (SIZE, SIZE), BACKGROUND)
    pixels = image.load()
    row = 0
    column = 0
    for normal in each_normal():
        if normal is not None:
            color = shade(normal)
            pixels[column, row] = tuple(round(clamp(c) * 255) for c in color)
        column += 1
        if column == SIZE:
            column = 0
            row += 1
    path = OUT / f"{name}.png"
    image.save(path, optimize=True)
    print(f"matcap {name}: {path} ({path.stat().st_size} bytes)")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name, shade in PRESETS.items():
        render(name, shade)


if __name__ == "__main__":
    main()
