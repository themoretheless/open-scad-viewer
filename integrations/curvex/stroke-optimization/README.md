# Curvex stroke optimization — 2026-09-13

The 2D kernel still has one runtime dependency: our `osv-math` crate. No external
geometry dependency was added. Lyon is used only by the comparison executable.
Curvex's GUI/SVG stack still has its separate transitive Kurbo dependency.

The previous stroke renderer expanded each edge and join into overlapping
polygons, solved their union, and swept the resulting boundaries into a mesh.
Ordinary open curves paid for this general arrangement even when no overlap
existed. The new route constructs offset boundaries and a strip of disjoint
triangles directly. Spatial and along-path gradient stroke APIs use the same
mesh route through `stroke::tessellate_stroke`.

Local inward trims must leave both incident edges intact. A bounded segment
sweep rejects nonadjacent contacts, overlaps and folds. Convex contours use a
complete-turn certificate and retained inward supports instead of pairwise
intersection checks. Miter, bevel and round joins and all three cap styles
retain their existing geometry and tolerance. Ambiguous or intersecting paths,
consumed insets, dashes and multi-contour render unions retain the general
solver. A rejected direct mesh does not repeat the intersection certificate.

## Measured uncached mesh construction

| Existing workload | Before, µs | After, µs | Speedup | Lyon control, µs |
| --- | ---: | ---: | ---: | ---: |
| Cubic circle stroke | 243.71 | 40.62 | 6.00× | 9.67 |
| Open cubic S stroke | 1769.08 | 27.13 | 65.22× | 8.50 |
| Actual Curvex sampled gradient-stroke polyline | 824.79 | 11.27 | 73.18× | 4.29 |
| Closed 1200-point stroke | 5157.90 | 206.96 | 24.92× | 176.73 |

The 1200-point stroke now has 2400 triangles instead of 9280. These are the
same fixtures, widths and tolerances. The table uses eight A/B/B/A cycles of
retained release binaries, with 16 per-version run medians; each run contains
21 samples after three warmups. This task's compilers and tests had stopped
during measurement. Other machine activity was uncontrolled. The
[raw results](../benchmark-results/stroke-optimization.json) preserve all run
medians, ranges, Lyon controls, executable hashes and source hashes.

These are mesh-construction timings, not gradient sampling or whole-document
FPS. Fill algorithms are unchanged; their controls are also retained, including
the 16-hole fill's measured 175→196 µs. The earlier measurement with more visible
background-load variation is saved as
[diagnostic evidence](../benchmark-results/stroke-optimization-initial.json).

## Validation

- **1492 Curvex tests pass**, with the existing ignored timing test also passing
  explicitly. Doctests, all-target checks and release build pass. The final run
  verified unchanged kernel sources throughout; see [report.json](report.json).
- **222 native tests pass**: 145 planar, 18 Curvex parity, 46 polygon and 13
  polygon integration. The shared checkout includes one unrelated BRep
  triangulation test. New regressions compare hundreds of open/closed/reversed
  cap/join/width combinations with the general union, check triangle area and
  exactly-once point coverage, reject crossing/touching/retraced strokes, and
  verify 4096-point translated contours and gradient-stroke holes/colors.
- **Six actual WASM transport tests pass** in an isolated committed-code
  snapshot with the optimization applied. Workspace WASM verification is
  recorded separately in [summary.json](summary.json).
- **36 captures at three zooms** match the previous own kernel's covered pixels
  exactly (IoU 1.0), with no color difference exceeding 3/255. Analytic gradient
  error remains at most 2/255. Cached frames remain identical. The visually
  inspected [contact sheet](render-contact-sheet.png) compares the previous own
  kernel (labelled Original) with this optimization. This is software geometry
  and color verification, not GPU antialiasing validation.

The original Curvex checkout and the existing migration patch are unchanged.
The original 2026-09-12 evidence is preserved separately. New command logs,
captures, source hashes and a compact result are saved here; `sha256.json`
covers these artifacts and the new benchmark reports.

Reproduce the native tests with the repository's pinned toolchain:

```sh
cargo test --offline --manifest-path crates/Cargo.toml -p planar-geometry -p polygon-core
```

The existing [benchmark instructions](../render-benchmark/README.md) and
[Curvex qualification runner](../README.md) apply unchanged. Use commit
`83f40b6` for the pre-optimization comparison; build both versions before
timing them, and retain the executable hashes rather than timing concurrent
Cargo builds.
