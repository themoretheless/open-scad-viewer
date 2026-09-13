# Curvex 2D library replacement contract

The latest [2D architecture and performance qualification](../../integrations/curvex/architecture-qualification/README.md) records the transport split, prepared geometry indexes, final benchmarks and verification limits.

Status: **qualified for the recorded Curvex 2D dependency boundary**, 2026-09-12.
The [2026-09-13 stroke optimization](../../integrations/curvex/stroke-optimization/README.md)
updates the stroke performance results with fresh qualification evidence.
The [complex-stroke, robustness and GPU follow-up](../../integrations/curvex/stress-qualification/README.md)
adds the latest migration patch and verifies native Metal rendering on large documents.
The isolated migrated application passes its full original test suite, added
regressions, release build and actual renderer checks. This contract covers
the actual 2D geometry dependencies used by Curvex, including its tests,
examples and benchmarks, and the behavior that must remain usable after their
replacement. It does not replace the user's broader 2D feature-completion goal.

The broader owned path/edit/effect behavior audit below is also implemented and
tested through Rust and the WASM/TypeScript boundary. The concrete
[migration patch](../../integrations/curvex/qualification-results/curvex-migration.patch),
[result summary](../../integrations/curvex/qualification-results/summary.json),
and [reproduction commands](../../integrations/curvex/README.md) are saved
alongside the implementation. Original Curvex remains unchanged.

Reviewed source: `/Users/themoretheless/Documents/Sources/curvex`, clean commit
`bf6967e4df618815e05eefc987947ca6b1c22461`. Source references below are relative
to that checkout and refer to that revision. The repeatable lexical inventory
and all scanned source hashes are in
[`integrations/curvex/baseline-inventory.json`](../../integrations/curvex/baseline-inventory.json).

## Actual dependency boundary

`Cargo.toml:21-24` declares `lyon_tessellation = "1.0"`, `lyon_path = "1.0"`,
`linesweeper = "0.4"`, and `kurbo = "0.13"`. These are the direct replacement
targets. The inventory includes every Rust source, even tests not containing
an explicit dependency import but included from production modules.

| Production source | External surface | Behavior and callers |
| --- | --- | --- |
| `src/boolean_ops.rs` | Kurbo paths, points, elements, `Shape`; linesweeper `binary_op_with_eps`, `BinaryOp`, `FillRule`, result contours | Union/intersection/difference/xor; arrangement cells, Divide, Shape Builder, subtract, crop, trim, merge by color, compound classification, path conversion, finite-output guard, region area and point-in-cell. |
| `src/document/kurbo_convert.rs:15-99` | Path construction; compound self-union | Converts *all* compound contours under the compound's own rule into canonical NonZero geometry. Open paths do not become boolean regions. Must remain below the app/UI layer. |
| `src/document.rs:373-393` | `Option<kurbo::BezPath>` | Cached path conversion for ordinary shapes; compounds use the full region converter. |
| `src/actions/transform/path_commands.rs:979-1058` | Stroke, cap, join, dash options and `kurbo::stroke` | Outline Stroke for open and closed sources, with source width/style, miter 4, optional sanitized two-entry dashes, tolerance 0.05 mm. |
| `src/actions/transform/path_effects/offset.rs:130-246` | Stroke + NonZero normalization + boolean | Offset uses stroke width `2*abs(distance)`, joins, miter 4, tolerance 0.05; compounds use region union/subtract so holes survive; eroded regions can disappear. |
| `src/tools/shape_builder.rs:55-75` | Vector of paths | Hover selection and click actions use the same cached document conversion and contour ordering. |
| `src/ui/shape_render.rs` | Lyon path construction, fill/stroke tessellation, generic vertex/index buffers; boolean Difference for inner shadows | Solid fill cache, compound fills, linear/radial gradient fill, gradient stroke ribbons, silhouette/inner-shadow regions. |

Tests with direct dependency names appear in `tests/unit/app_tests.rs`,
`boolean_ops_tests.rs`, `compound_ops_tests.rs`, `outline_stroke_tests.rs`,
`shape_builder_tests.rs`, and `render_cache_tests.rs`; other unit files have
behavioral contracts and dependency names in comments. Explicit benchmark
imports occur in `benches/perf_cache_round2.rs` and
`benches/perf_top12_verify.rs`. The checked-in JSON is the exhaustive per-line
inventory; this table is the semantic grouping.

## Required geometry behavior

1. **Editable curves and path fidelity.** The path boundary needs multiple
   subpaths with explicit closure, MoveTo/LineTo/QuadTo/CurveTo/ClosePath,
   construction, iteration, clone/default/empty checks, append/extend, and
   independent subpath reversal (`boolean_ops_tests.rs:76`).
   Quadratics convert exactly to cubics using the two-thirds controls
   (`src/boolean_ops.rs:2121-2215`). Existing Curvex storage remains f32 mm;
   solver calculations are f64. A contour with a non-finite converted point is
   rejected without injecting invalid state into the document.
2. **Boolean curve preservation.** The result must contain retained/subdivided
   genuine source cubic arcs, not only a flattened polygon or straight-line
   segments relabeled as cubics. This is explicitly checked by
   `app_tests.rs:644-648,699-810,908-924,959-979,1361-1376` and
   `boolean_ops_tests.rs:364-375`. Rounded protrusions, disjoint rounded shapes,
   ellipses, holes and islands must retain their geometry. The solver epsilon
   is 0.05 mm (`boolean_ops.rs:1977,2085-2118`); the epsilon must actually
   control approximation/intersection handling, including near-tangent input.
3. **Region semantics.** Both EvenOdd and signed NonZero are needed across all
   contours; same-winding nesting, reverse-winding holes, islands in holes,
   self-intersections, coincidence, edge-touching, emptiness and disconnected
   output must agree with what is rendered and selected. Document compound
   normalization uses the source's rule, then downstream boolean folds use
   NonZero. Output orientation is observable. Existing normalization fallback
   returns raw input on solver failure/empty normalization rather than silently
   dropping a source (`document/kurbo_convert.rs:53-99`).
4. **Analytic path queries.** `Shape::area`, `Shape::bounding_box`, and
   `Shape::winding` are used. Area drives real/no-op decisions in Trim and Shape
   Builder (`boolean_ops.rs:1243-1258,1550-1552,2709-2719`), with holes included
   through signed winding. Shape Builder point containment evaluates original
   curves without allocating a fresh flattened path on every hover
   (`boolean_ops.rs:2021-2029`). Exact cubic extrema are needed for bounds;
   the distinct control-point bbox area helper must retain its existing
   meaning (`boolean_ops.rs:2668-2707`). Core analytic signed-area, bounds and
   winding functions are preferable to a mutable polygon approximation.
5. **Stroke and offset.** Support open/closed and multiple subpaths, butt/round/
   square caps, miter/round/bevel joins, the supplied miter limit, dash offset
   and dash pattern, closure-seam handling, invalid/zero-width guards, and
   tolerance at or below 0.05 mm for Curvex's calls. Closed stroke rings must
   include outer and inner boundaries (`outline_stroke_tests.rs:136-200`).
   Offset preserves source compound holes, resolves self-crossing stroke
   regions, handles full erosion, and returns valid editable contours.
6. **Tessellation and mesh attributes.** Path building needs begin, line_to,
   cubic_bezier_to, end(closed), and build; multiple closed subpaths form one
   fill job. Fill tolerance is `0.25 / bucketed_zoom` mm for cached solid fills
   (`shape_render.rs:298-299`) and 0.05 mm for gradient fills (`:1640-1642`).
   Gradient stroke ribbons use the same already-flattened polyline as the solid
   stroke, width in mm, and 0.05 tolerance (`:1338-1379`). Meshes contain vertex
   positions and u32 triangle indices. Curvex requests **only vertex position**
   from Lyon (`:215-218,1368-1371,1649-1652`); gradient sampling, alpha,
   premultiplication, UV and world-to-screen mapping remain in Curvex. No Lyon
   custom attribute interpolation or GPU state emulation is required. Compound
   gradient tessellation must retain disjoint islands and holes, skip invalid
   and collinear contours, and accept self-crossing zero-signed-area bow-ties
   (`:1537-1659`).
7. **Editor invariants.** Preserve transaction behavior, selection, IDs,
   z-order, source styles (including gradients and effects), compound fill
   rules, cache revisions, and undo/redo. Many operations call the same geometry
   while having distinct document semantics. A kernel-only test does not cover
   these. Preserve the dependency direction checked by
   `tests/architecture_boundaries.rs`; model conversion cannot call app/UI
   modules. Preserve `tests/unit_test_files_wired.rs` so tests cannot disappear
   from the executed suite through a missing include.

Ordinary hit testing in `src/hit_test.rs` is already Curvex code. It is not a
hidden Kurbo API requirement; its tests nevertheless must still pass because
boolean/offset output changes what it receives. Likewise, all Curvex's own
editing/effect helpers need not be moved into an adapter to replace its four
direct dependencies, but the broader feature goal still requires their real
behavior to be supported by the shared library.

## Smallest faithful migration

Keep Curvex's document and editor workflow. Supply a small conversion boundary
to the shared kernel, and replace the seven production consumers above plus
their test/bench imports. A local adapter may expose the narrow existing path
and mesh interfaces to reduce mechanical churn; it must compute through the
independent kernel and must not retain the old crates behind renamed exports.
Alternatively migrate those consumers to native kernel types. Either option
must ship an actual patch and build the resulting Curvex.

Necessary kernel additions at the reviewed baseline are curve-preserving
multi-path booleans, analytic curve area/bounds/winding, reliable multi-contour
fill normalization, tolerance-controlled stroke outlines, and indexed stroke
tessellation. Existing `BezierPath`, `StrokeOptions`, and fill mesh types can
be reused. A curve boolean may use flattened segments for candidate discovery
only if it tracks source-curve parameters, refines intersections within the
requested tolerance, and reconstructs genuine de Casteljau subcurves. A
polyline boolean followed by arbitrary curve fitting is not proof of preserved
curve semantics.

At the audited baseline, `stroke::outline_stroke` unconditionally calls the
0.25-mm default flatten operation and offers no tolerance parameter, while
`rings`/`pathfinder` return polygons. These are substantive gaps, regardless of
feature names. Independent agents may be repairing these files; use current
code and new evidence for the qualification, not this historical status.

## Completion gates

### Owned path/editor behavior audit

The shared library also covers the owned geometry helpers outside the four
external dependencies. This is a public API audit, separate from the final
migrated-application qualification below. Source references are relative to the
recorded original Curvex checkout.

| Curvex behavior | Shared core / transport | Verified semantic details |
| --- | --- | --- |
| `path.rs`: node insert/delete/split, multiple node splits, segment deletion, handles, smoothing/corners; `actions/path_nodes.rs`: averaging | `path::BezierPath` node methods, `average_anchor_targets`; `path2d` node actions | Cubic subdivision retains controls; closed seam and open endpoints remain distinct. Handle linking validates indices, ignores absent moved handles and absent endpoint opposites, and promotes opposite line handles. Mirrored and Symmetric intentionally coincide in Curvex. |
| `actions/transform/path_effects/corners.rs:116`, `path.rs:610` | `corners::round_corners`, `rounded_open_polyline`; `round_corners`, `rounded_open_polyline` | Round Corners uses actual anchors rather than cubic flatten samples. Open endpoints stay pinned. Per-node Round/Chamfer/Inverted/Notch and signed-radius chamfers are available for open and closed contours. |
| `actions/transform/distortions.rs:1342` | `BezierPath::simplify`; `simplify` | Default-tolerance flattening precedes RDP. Closed rings split at the farthest pair, so a collinear start seam is removable. Convex hull/calipers selects the same diameter without quadratic work; regression compares 85 irregular rings against an independent quadratic reference. |
| `path.rs:758–868`, `path_join.rs` | `endpoint_tangent`, `join_paths_with_bridge`, `join_paths_at_tangents`, `close_path_at_tangents`; join/close actions | Custom line/cubic bridge geometry survives. Welding uses both coordinate tolerances. Automatic joins extend forward tangent rays only within three endpoint distances (minimum reach 1), otherwise use a straight connector. Collapsed handles use endpoint chords. |
| `snap.rs:551–840` | `edit::SnapGeometry`, `find_geometry_snap`; `snap_geometry` / `findGeometrySnap` | Line, Rectangle, Ellipse, Polygon, Polyline, BezierPath and Compound preserve semantic candidates. Rounded primitive outlines use their visible samples; BezierPath uses real anchors. Midpoint, Endpoint/Vertex/Corner, Center, Intersection, Perpendicular priorities precede distance. Adjacent flattened chord ends never become intersection snaps. Legacy `find_snap`/`findPathSnap` retains its prior contract. |
| `snap.rs:68`, `:243`, `:935`, `:976` | `find_drag_snap`, `compute_distance_marks`, `snap_to_rays`, `intersecting_paths`, `intersecting_path_directions`; corresponding transport wrappers | Drag ties choose the first target and include guide extents. Cardinal gaps and margins to the smallest enclosing rectangle are retained. Ray snapping is optional within angular tolerance, projects onto the ray, and ignores displacement under 2 units. Crossings retain actual directions/points; parallel directions deduplicate within one degree. |
| `align.rs` | `align_boxes_relative`, `distribute_objects`, `distribute_spacing` | Selection/key/page/last-selected references, edge/center distribution, explicit positive or negative gaps; translation formulas match the source. The transport uses errors for invalid inputs where editor commands may instead return no-op. |
| `scissors.rs`, `scissors_line.rs` | `scissors::{hit_test,cut_at,cut_at_many,knife_hits,knife_cut,knife_split}` and compound variants; corresponding path/compound actions | Cubic hit sampling and de Casteljau cuts follow Curvex. `CompoundCutHit` retains ring identity. `PathPieces` is figure → contours, so untouched holes/islands remain grouped. Filled knife pieces regroup by chord side and even-odd containment. A hole-only/disjoint-ring slice stays one compound. Original winding is preserved; regrouping has Curvex's even-odd contract. |
| `path.rs` primitives and `actions/transform` effects/arrays | `effects`, `corners`, `BezierPath` plus typed TypeScript wrappers | Ellipse/spiral/polar grid/arc/pie/segment/star, corner styles, ridge zig-zag, pucker/bloat, detailed smooth roughen, twist, seam/winding-aligned blend, free distort, grouped scatter and copy-only step/radial/grid arrays. Closed concentric offsets honor compound fill rules. Hatch/stipple expose fill, holes, tolerance, jitter, seed and marker parameters. Legacy array defaults remain compatible. |

`crates/planar-geometry/tests/curvex_parity.rs` provides 18 additional regressions
for the behavior above, including exact filled areas after compound slicing.
`tests/path2dCurvex.test.ts` runs the typed APIs against generated geometry WASM;
bridge dispatcher tests exercise the same JSON actions. These complement the
effects/unit suites and do not replace the full application gates. The kernel
uses binary64, explicit resource budgets and validated `Result` errors; it is
not a promise of bit-identical f32 rounding or editor document/history policy.

### Qualification requirements

| Gate | Required evidence |
| --- | --- |
| Source and repeatability | Original Curvex commit and clean/dirty status, full source hashes, a reviewable migration patch or reproducible staging command, shared-kernel revision/diff. The migration must apply to the recorded source. |
| Dependency replacement | Patched Curvex manifest has none of the four direct dependencies; every production/test/bench reference routes to the replacement. `cargo metadata`/`cargo tree` show the actual solver/tessellator dependency closure contains no old implementation or renamed wrapper. Save remaining GUI/SVG transitive dependency chains explicitly. |
| Full Curvex validation | Run `cargo test --lib --bins --tests`, `cargo test --doc`, `cargo check --all-targets`, and `cargo build --release` on the patched checkout. Record actual commands, exit status, totals and failures. Do not delete, ignore, weaken or retarget assertions to pass the replacement. `--all-targets` check covers examples and Criterion benches without accidentally executing long benchmarks as tests. |
| Additional geometry regressions | Exercise analytically known and differential cases for both fill rules, compound nesting and direction, self-intersections/tangencies/coincident edges, sharp/rounded/cubic boolean output, tolerance scaling, cap/join/dash/seams, erosions and invalid input. Verify region coverage/topology, analytic area/bounds, actual curve control points/retained arcs, and finite indexed meshes. Differentials may use old libraries in a separate test harness, never as the replacement implementation. |
| Full viewer validation | Existing planar-geometry tests plus relevant WASM/TypeScript geometry wrappers and build checks pass with the new API. Every required 2D operation and parameter is callable by the intended Curvex integration. |
| Rendering and interaction | Render representative solid and gradient fills/strokes, compound holes/islands/bow-ties, outlined strokes and offset ghosts. Check at low/high zoom, exercise Shape Builder hover/click, undo/redo and cache invalidation on the migrated Curvex. Preserve color/opacity semantics, not only triangle counts. Save reproducible evidence. |
| Performance | Compare the same release benchmark workloads on original and patched Curvex, especially repeated boolean, arrangement, stroke, fill cache, gradients and drag loop. Investigate material regressions rather than assuming compile/test success preserves interactive usability. |

The original lockfile also contains Kurbo through `usvg`, `svgtypes`, and
`peniko`. Removing the direct geometry dependency is distinct from eliminating
every transitive Kurbo package in the GUI/SVG stack. Qualification must state
which is proven, with reverse dependency evidence; a claim of complete absence
requires migrating those remaining users too. It must never be inferred from
removing four lines in Cargo.toml.

## Qualification results — 2026-09-12

All results refer to the frozen source hashes in
[`qualification-results/report.json`](../../integrations/curvex/qualification-results/report.json).
The runner verified unchanged source hashes across the complete run. It clears
only Curvex's package artifacts before testing: shared target directories can
otherwise reuse a test binary from another isolated copy with the same package
identity. It additionally requires all three new migration tests to execute.

| Gate | Result |
| --- | --- |
| Original and patch | Original commit `bf6967e4df618815e05eefc987947ca6b1c22461`, clean before and after qualification. Reproducible isolated migration and full source/lockfile patch saved. |
| Dependency boundary | All four direct dependencies removed. Entire native kernel runtime closure is `planar-geometry → osv-math`. Actual native Cargo trees retain Kurbo only under GUI/SVG dependencies; neither Lyon crate nor linesweeper is in the native application package set. |
| Actual Curvex | **1492 passed, 0 failed**, one original timing test remains marked ignored and was also run explicitly successfully. All 1490 original test names remain, plus three migration tests. Doctests, `check --all-targets`, and `build --release` pass on Rust 1.98.1. |
| Native viewer kernel | **142 planar unit +18 Curvex parity +45 polygon unit +13 polygon integration tests pass**. Includes all subsets and cyclic orders of four overlapping rectangles, near-axis crossings, coincident/tangent cubic arcs, retained source controls, winding, holes, stroke widths/caps/joins and bounded invalid-input behavior. |
| Transport and WASM | **8 actual bridge dispatcher tests**, **6 actual generated-WASM tests**, Vue and MCP TypeScript checks pass. Final WASM and kernel source hashes are retained. |
| Actual render pipeline | **36 captures ×3 frames** through Curvex paint plus egui tessellation: 12 scenes at zoom 0.5/2/8, including shadow ghosts. Minimum baseline/replacement coverage IoU **0.999754**; all cached frames are pixel-identical to uncached frames within each implementation. Gradient channels differ from the independent analytic oracle by at most **2/255**. |
| Document/editor behavior | The unchanged application tests exercise document transactions, Shape Builder containment/selection and extraction, Divide, source styles, IDs, undo/redo and cache revisions. Added cache tests retain both original geometry and sampled mesh identities for **300 gradient shapes ×3 frames**, and validate translated gradient seams at coordinates 10⁷. |
| Performance investigation | Actual release Boolean workloads measured in A/B/B/A order on retained, hashed binaries with no team builds running. 50 ellipse subtractions: **8.36 ms vs 17.09 ms**; the remaining small-operation ratios are documented. Direct tessellation and actual cold/warm paint costs are recorded separately. |

The [render contact sheet](../../integrations/curvex/qualification-results/render-contact-sheet.png)
shows the original, migrated and difference images. Nonlinear gradients are
intentionally corrected: sampling only polygon boundary vertices made the old
radial/conic/repeated output depend on tessellation diagonals. The migration
samples interiors and one-sided seam values, while retaining Curvex's color,
spread, opacity and cache behavior. The old pixels remain in the evidence; the
analytic gradient is the color oracle. This is headless CPU geometry/color
validation, not a manual GPU-antialiasing or platform-window test.

The maximum observed single-shape paint+tessellation times in these scenes
were **2.74 ms** for uncached drawing, **3.81 ms** for the first draw that
populates the cache, and **0.104 ms** for a warm cache. These measurements do
not imply a whole-document frame-rate guarantee. In the independent
[stroke/fill benchmark](../../integrations/curvex/benchmark-results/render-final.json),
4096-point fill is about **0.134 ms vs Lyon's 0.322 ms**. General uncached stroke
construction at that initial qualification was substantially slower: **0.335–0.652 ms** for the tested
open curves and **1.74 ms** for 1200 points; Lyon takes roughly 0.002–0.004 ms
and 0.080 ms respectively. This residual cost is explicit rather than hidden
behind cached or unrelated gradient-sampling timings. The complete
[Boolean report](../../integrations/curvex/benchmark-results/boolean-report.md)
records both improvements and residual regressions after the Divide fix.

The original Curvex [interaction benchmarks](../../integrations/curvex/benchmark-results/interaction-final.json)
also run unchanged apart from dependency namespaces, on 500 paths with 120
cubic segments each. Ready release binaries were measured in A/B/B/A order,
with 20 samples per workload per run:

| Interaction/query workload | Original | Replacement |
| --- | ---: | ---: |
| Drag tick: five moved shapes, revision invalidation, snapping and distance marks | 195.91 µs | 198.48 µs |
| 100 hit-test queries with a warm document cache | 925.31 µs | 926.67 µs |
| Scissors hover with a warm document cache | 498.93 µs | 502.60 µs |

The observed median differences are 0.15–1.31%. The envelopes of the two
per-run 95% median intervals overlap for each workload; these short runs do
not establish a material performance regression. Raw Criterion samples and
commands are retained. These loops exclude painting and do not conceal the
uncached stroke costs above.

This qualification is bounded by the actual API used by the recorded Curvex
revision, rather than every API of the replaced third-party crates. Native
paths and render input support 65,536 source segments; arrangement,
intersection, tessellation and adaptive-sampling work limits remain explicit.
Finite renderer coordinates are no longer artificially capped at ±10⁶, and
5000-point/translated/very small and large scale regressions pass. Arbitrarily
narrow radial/conic stop bands are not guaranteed by bounded adaptive preview
sampling. Remaining GUI/SVG Kurbo usage and these resource/preview limits must
not be described as an unrestricted replacement of every third-party API.

## Subsequent GPU / Metal qualification

The [retained Metal implementation and report](../../integrations/curvex/metal-renderer/README.md)
supersede the earlier CPU-only rendering qualification for the tested Apple M5
platform. The isolated migration passes 1494 application tests, 288 exact GPU
pixel comparisons at DPI 1/2, ten edit/restore/renderer/device lifecycle phases,
and native startup with retained rendering both enabled and disabled. On the
5000-shape spatial workload, median warm CPU frame preparation falls from
40.560 to 10.402 ms; submit-to-render-completion falls from 6.117 to 3.076 ms.
These are workload measurements, not GPU-exclusive timers or editor FPS.
The report documents bounded memory, ordinary-renderer fallback, a verified
patch, raw samples and remaining host/platform/precision limits.
