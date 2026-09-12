# planar-geometry

Planar 2D shape kernel for CAD / vector work: cubic Bézier paths, closed rings
(boolean / offset), Pathfinder z-order ops, stroke outlines, scissors, and fill
tessellation.

Coordinates are **millimeters**; algorithms use **binary64**. No triangle-mesh,
NURBS, SDF, or renderer dependency.

## Stable Rust

This crate builds on **stable** Rust (no `#![feature]`). Workspace CI may still
use a dated nightly toolchain for other crates.

## Install

```toml
[dependencies]
planar-geometry = "0.1"
```

Requires published [`osv-math`](https://crates.io/crates/osv-math) `0.1`
(publish `osv-math` first, then this crate). Workspace directory for that
dependency is still `crates/math-core`.

```sh
cargo publish -p osv-math --manifest-path crates/Cargo.toml
cargo publish -p planar-geometry --manifest-path crates/Cargo.toml
```

## Modules

| Module | Role |
| --- | --- |
| `path` | `BezierPath`, flatten, node edit |
| `rings` | Closed polyline boolean / offset |
| `pathfinder` | Divide / crop / trim / minus / merge |
| `stroke` | Caps, joins, dashes, arrow markers |
| `path_offset` | Closed-region offset (stroke-ring + boolean, kurbo parity) |
| `tessellation` | Ear-clip fill mesh |
| `scissors` | Knife hit / cut / split |
| `edit` | Align, distribute, measure |
| `effects` | Hatch, stipple, distortions |
| `corners` | Fillet / chamfer helpers |

## License

MIT — see repository root `LICENSE`.
