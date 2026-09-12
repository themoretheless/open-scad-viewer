#!/usr/bin/env python3
"""Compare captures from the unchanged real Curvex renderer qualification example.

Rasterizes position/color meshes with a top-left triangle coverage rule. This
checks geometry, gradients, alpha, zoom and cache invariance without a GPU or
window. It does not claim to reproduce GPU antialiasing or color conversion.
Requires numpy and Pillow. Retains source hashes and a visual contact sheet.
"""
import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw


def edge(a, b, x, y):
    return (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])


def raster(frame, zoom, scale):
    size = round(110 * scale)
    rgb = np.ones((size, size, 3), dtype=np.float64)
    coverage = np.zeros((size, size), dtype=bool)
    for mesh in frame["meshes"]:
        positions = np.array([v["position"] for v in mesh["vertices"]]) * scale
        colors = np.array([v["color"] for v in mesh["vertices"]]) / 255
        if not np.isfinite(positions).all() or not np.isfinite(colors).all():
            raise ValueError("Renderer emitted nonfinite vertices")
        clip = np.asarray(mesh["clip"]) / zoom * scale
        indices = mesh["indices"]
        assert len(indices) % 3 == 0
        for i in range(0, len(indices), 3):
            ids = np.asarray(indices[i:i + 3])
            p, c = positions[ids], colors[ids]
            area = edge(p[0], p[1], *p[2])
            if abs(area) < 1e-12:
                continue
            if area < 0:
                p = p[[0, 2, 1]]
                c = c[[0, 2, 1]]
                area = -area
            x0 = max(0, int(np.floor(max(p[:, 0].min(), clip[0]))))
            x1 = min(size, int(np.ceil(min(p[:, 0].max(), clip[2]))))
            y0 = max(0, int(np.floor(max(p[:, 1].min(), clip[1]))))
            y1 = min(size, int(np.ceil(min(p[:, 1].max(), clip[3]))))
            if x1 <= x0 or y1 <= y0:
                continue
            y, x = np.mgrid[y0:y1, x0:x1] + 0.5
            edges, masks = [], []
            for a, b in ((p[1], p[2]), (p[2], p[0]), (p[0], p[1])):
                e = edge(a, b, x, y)
                top_left = b[1] < a[1] or (b[1] == a[1] and b[0] > a[0])
                edges.append(e / area)
                masks.append((e > 1e-9) | ((abs(e) <= 1e-9) & top_left))
            mask = masks[0] & masks[1] & masks[2]
            mask &= (x >= clip[0]) & (x < clip[2]) & (y >= clip[1]) & (y < clip[3])
            rgba = sum(e[:, :, None] * color for e, color in zip(edges, c))
            dst = rgb[y0:y1, x0:x1]
            # egui stores premultiplied vertex colors.
            dst[mask] = (rgba[:, :, :3] + dst * (1 - rgba[:, :, 3:]))[mask]
            coverage[y0:y1, x0:x1] |= mask & (rgba[:, :, 3] > 0)
    return np.clip(np.rint(rgb * 255), 0, 255).astype(np.uint8), coverage


def gradient_oracle(capture, scale):
    """Evaluate the declared gradient at pixels, independently of triangles."""
    gradient = capture.get("gradient")
    if not gradient:
        return None
    size = round(110 * scale)
    y, x = (np.mgrid[:size, :size] + 0.5) / scale
    lo_x, lo_y, hi_x, hi_y = capture["bbox"]
    x, y = (x-lo_x)/(hi_x-lo_x), (y-lo_y)/(hi_y-lo_y)
    kind, args = next(iter(gradient["kind"].items()))
    if kind == "Linear":
        a, b = args["p1"], args["p2"]
        dx, dy = b["x"]-a["x"], b["y"]-a["y"]
        t = ((x-a["x"])*dx+(y-a["y"])*dy)/max(dx*dx+dy*dy, 1e-9)
    elif kind == "Radial":
        c = args["center"]
        t = np.hypot(x-c["x"], y-c["y"])/args["radius"]
    elif kind == "Conic":
        c = args["center"]
        t = ((np.arctan2(y-c["y"], x-c["x"])-args["angle"])/(2*np.pi)) % 1
    else:
        raise ValueError(kind)
    spread = gradient.get("spread", "Pad")
    if spread == "Repeat":
        t %= 1
    elif spread == "Reflect":
        t = 1-abs(t % 2-1)
    t = np.clip(t, 0, 1)
    stops = sorted(gradient["stops"], key=lambda stop: stop["offset"])
    rgba = np.stack([np.floor(np.interp(t, [s["offset"] for s in stops],
                           [s["color"][channel] for s in stops])+0.5)
                     for channel in ("r", "g", "b", "a")], axis=-1)
    # Fixtures have opaque color stops. Master opacity is folded exactly as in
    # Curvex gradient_color_at; preserve its integer premultiplication behavior.
    assert all(s["color"]["a"] == 255 for s in stops)
    alpha = np.floor(rgba[:, :, 3:] * capture["opacity"]+0.5)
    return np.clip(np.floor(rgba[:, :, :3] * alpha / 255) + 255-alpha, 0, 255).astype(np.uint8)


def compare(baseline_path, migrated_path, output, scale):
    baseline, migrated = [json.loads(p.read_text()) for p in (baseline_path, migrated_path)]
    assert [(c["case"], c["zoom"]) for c in baseline] == [(c["case"], c["zoom"]) for c in migrated]
    output.mkdir(parents=True, exist_ok=True)
    metrics = []
    rows = []
    for original, replacement in zip(baseline, migrated):
        rendered = [[raster(f, capture["zoom"], scale) for f in capture["frames"]]
                    for capture in (original, replacement)]
        a, b = rendered[0][0][0], rendered[1][0][0]
        ma, mb = rendered[0][0][1], rendered[1][0][1]
        delta = abs(a.astype(np.int16) - b.astype(np.int16))
        active = ma | mb
        cache_identical = [all(np.array_equal(frames[0][0], f[0]) and
                               np.array_equal(frames[0][1], f[1]) for f in frames[1:])
                           for frames in rendered]
        item = {"case": original["case"], "zoom": original["zoom"],
                "baseline_triangles": sum(len(m["indices"]) // 3 for m in original["frames"][0]["meshes"]),
                "migrated_triangles": sum(len(m["indices"]) // 3 for m in replacement["frames"][0]["meshes"]),
                "coverage_iou": float((ma & mb).sum() / max(1, active.sum())),
                "coverage_disagreement_pixels": int((ma ^ mb).sum()),
                "mean_channel_difference_active": float(delta[active].mean()) if active.any() else 0,
                "changed_fraction_active_gt3": float((delta.max(axis=2)[active] > 3).mean()) if active.any() else 0,
                "cache_identical": dict(zip(("baseline", "migrated"), cache_identical))}
        metrics.append(item)
        oracle = gradient_oracle(original, scale)
        if oracle is not None:
            item["analytic_gradient_error"] = {}
            for label, pixels, mask in (("baseline", a, ma), ("migrated", b, mb)):
                error = abs(pixels.astype(np.int16)-oracle.astype(np.int16))[mask]
                item["analytic_gradient_error"][label] = {
                    "mean_channel_error": float(error.mean()) if len(error) else None,
                    "p99_channel_error": float(np.percentile(error, 99)) if len(error) else None,
                    "max_channel_error": int(error.max()) if len(error) else None,
                    "fraction_pixels_gt3": float((error.max(axis=1)>3).mean()) if len(error) else None}
        prefix = original["case"].replace(" ", "-") + "-zoom" + str(original["zoom"])
        diff = np.clip(delta * 4, 0, 255).astype(np.uint8)
        for suffix, pixels in (("baseline", a), ("migrated", b), ("difference-x4", diff)):
            Image.fromarray(pixels).save(output / f"{prefix}-{suffix}.png")
        if oracle is not None:
            oracle[~mb] = 255
            Image.fromarray(oracle).save(output / f"{prefix}-analytic-gradient.png")
        thumb_size = 220
        row = Image.new("RGB", (thumb_size * 3, thumb_size + 42), "#f1f3f5")
        draw = ImageDraw.Draw(row)
        draw.text((8, 5), f'{original["case"]} | zoom {original["zoom"]} | coverage IoU {item["coverage_iou"]:.6f}', fill="black")
        for column, (label, pixels) in enumerate((("Original", a), ("Replacement", b), ("Difference x4", diff))):
            draw.text((column * thumb_size + 8, 23), label, fill="black")
            row.paste(Image.fromarray(pixels).resize((thumb_size, thumb_size)), (column * thumb_size, 42))
        rows.append(row)
    sheet = Image.new("RGB", (rows[0].width * 3, rows[0].height * ((len(rows) + 2) // 3)), "white")
    # Keep the three zoom levels of each case adjacent.
    for index, row in enumerate(rows):
        sheet.paste(row, ((index % 3) * row.width, (index // 3) * row.height))
    sheet.save(output / "contact-sheet.png")
    report = {"method": "Software barycentric premultiplied color rasterization, top-left rule; no GPU antialiasing",
              "pixels_per_document_mm": scale,
              "inputs": {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in (baseline_path, migrated_path)},
              "cases": metrics}
    (output / "comparison.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(metrics, indent=2))
    if not all(all(m["cache_identical"].values()) and m["migrated_triangles"] > 0
               and m["coverage_iou"] >= 0.99 for m in metrics):
        raise SystemExit("Missing geometry, changed cached output or coverage regression")
    if not all(m.get("analytic_gradient_error", {}).get("migrated", {}).get("fraction_pixels_gt3", 0) <= 0.005
               for m in metrics):
        raise SystemExit("Gradient sampling differs from analytic color at more than 0.5% of covered pixels")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("migrated", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--scale", type=float, default=4)
    args = parser.parse_args()
    compare(args.baseline, args.migrated, args.output, args.scale)
