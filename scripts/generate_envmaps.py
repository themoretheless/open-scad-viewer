#!/usr/bin/env python3
"""Renders the built-in equirectangular environment maps into public/env/.

Each preset paints a 2:1 equirect panorama (u = azimuth around +Y, v = 0 at
the +Y pole), matching how mesh_pbr.wgsl samples the texture:
u = atan2(z, x) / tau + 0.5, v = acos(y) / pi. Kept small (512x256) and
smooth — the shader's diffuse cone average and single-sample specular assume
low-frequency content. Colors are written in linear-ish space (the shader
tonemaps after accumulation), so values may exceed the display range mildly.

Usage: python3 scripts/generate_envmaps.py
"""
import math
from pathlib import Path

from PIL import Image

WIDTH = 512
HEIGHT = 256
OUT = Path(__file__).resolve().parent.parent / "public" / "env"


def clamp(value, lo=0.0, hi=1.0):
    return lo if value < lo else hi if value > hi else value


def each_direction():
    """Yields (x, y, z) unit directions, equirect scan order (top row = +Y)."""
    for row in range(HEIGHT):
        v = (row + 0.5) / HEIGHT
        y = math.cos(v * math.pi)
        ring = math.sin(v * math.pi)
        for column in range(WIDTH):
            u = (column + 0.5) / WIDTH
            phi = (u - 0.5) * 2.0 * math.pi
            yield (math.cos(phi) * ring, y, math.sin(phi) * ring)


def shade_studio_softbox(d):
    """Studio: gray gradient room, one big overhead softbox, warm side card."""
    x, y, z = d
    base = 0.18 + 0.30 * clamp(0.5 + 0.5 * y)
    r, g, b = base, base, base * 1.04
    # Softbox: a wide soft rectangle above, slightly toward -x.
    box = math.exp(-(((x + 0.25) ** 2) * 6.0 + (z ** 2) * 6.0 + ((y - 1.0) ** 2) * 3.0))
    r += box * 1.5; g += box * 1.5; b += box * 1.45
    # Warm bounce card on +z side.
    card = math.exp(-(((x - 0.6) ** 2) * 2.0 + ((z - 0.8) ** 2) * 2.0 + ((y + 0.1) ** 2) * 2.0))
    r += card * 0.35; g += card * 0.22; b += card * 0.12
    # Dark floor.
    floor = clamp(-y) ** 1.5 * 0.10
    return (clamp(r - floor), clamp(g - floor), clamp(b - floor))


def shade_outdoor_sky(d):
    """Outdoor: blue-sky gradient to warm horizon, sun disk, dim ground."""
    x, y, z = d
    h = clamp(y * 0.5 + 0.5)
    # Sky gradient: warm horizon into cool zenith.
    r = 0.75 - 0.45 * h
    g = 0.80 - 0.35 * h
    b = 0.85 - 0.15 * h
    # Sun: bright disk toward (+0.5, 0.55, +0.66).
    sun = clamp((x * 0.5 + y * 0.55 + z * 0.66 - 0.965) / 0.035)
    r += sun * 2.2; g += sun * 1.9; b += sun * 1.4
    # Ground: dark desaturated green-brown below the horizon.
    if y < 0.0:
        k = clamp(-y * 2.5)
        r = r * (1 - k) + 0.16 * k
        g = g * (1 - k) + 0.15 * k
        b = b * (1 - k) + 0.11 * k
    return (clamp(r, 0.0, 3.0), clamp(g, 0.0, 3.0), clamp(b, 0.0, 3.0))


def shade_workshop(d):
    """Workshop: dim warm room with a row of cool ceiling strip lights."""
    x, y, z = d
    base = 0.12 + 0.10 * clamp(0.5 + 0.5 * y)
    r, g, b = base * 1.15, base, base * 0.9
    # Strip lights: two long cool bands across the ceiling (periodic in x).
    if y > 0.35:
        for zc in (0.45, -0.45):
            band = math.exp(-(((z - zc) ** 2) * 30.0 + ((y - 0.9) ** 2) * 4.0))
            r += band * 0.9; g += band * 1.0; b += band * 1.25
    # Faint warm window on one wall.
    win = math.exp(-(((x + 0.9) ** 2) * 4.0 + (z ** 2) * 2.0 + ((y - 0.2) ** 2) * 3.0))
    r += win * 0.4; g += win * 0.28; b += win * 0.16
    return (clamp(r), clamp(g), clamp(b))


PRESETS = {
    "studio-softbox": shade_studio_softbox,
    "outdoor-sky": shade_outdoor_sky,
    "workshop": shade_workshop,
}


def to_srgb_byte(linear):
    """Linear → sRGB byte, so the rgba8unorm texture reads back linear values."""
    c = clamp(linear, 0.0, 1.0)
    srgb = 12.92 * c if c <= 0.0031308 else 1.055 * (c ** (1.0 / 2.4)) - 0.055
    return round(clamp(srgb) * 255)


def render(name, shade):
    image = Image.new("RGB", (WIDTH, HEIGHT))
    pixels = image.load()
    column = 0
    row = 0
    for direction in each_direction():
        color = shade(direction)
        # Values above 1 (the sun) clip to sRGB white: acceptable for these
        # low-dynamic-range preview maps; tonemapping happens in the shader.
        pixels[column, row] = tuple(to_srgb_byte(c) for c in color)
        column += 1
        if column == WIDTH:
            column = 0
            row += 1
    path = OUT / f"{name}.png"
    image.save(path, optimize=True)
    print(f"env {name}: {path} ({path.stat().st_size} bytes)")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name, shade in PRESETS.items():
        render(name, shade)


if __name__ == "__main__":
    main()
