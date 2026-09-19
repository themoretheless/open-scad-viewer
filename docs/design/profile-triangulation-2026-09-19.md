# Boundary-preserving profile triangulation: 2026-09-19

Continuation of [the CSG scaling audit](csg-scaling-2026-09-19.md) and
[retained-solid analysis](solid-analysis-2026-09-19.md). Measurements use the
same local M4 Max, Node 22.23.2, full-quality own-Rust WASM evaluator. This is
local evidence, not browser edit latency, CI qualification or deployment proof.

## Cause and Change

The actual prism-generated 4x4 hole profile failed although a similar synthetic
grid passed. Visibility of a proposed hole bridge was insufficient: an existing
bridge endpoint occurs more than once in the stitched boundary. Attaching to
the wrong local sector produced a boundary that ear clipping could not resolve.
The caller then retried through the much more expensive general BSP Boolean.

`planar-geometry::triangulation` now tests the local sector at each candidate
occurrence before containment/intersection checks. Coordinates are never nudged.
It caches candidate distances once before sorting, rejects points outside an
ear's bounding box before cross products, and resumes ear search at the removal
cursor instead of rescanning the same prefix. A linked-neighbor experiment was
not retained: contiguous scans were competitive or faster on these fixtures.

Admission is now 4096 stitched vertices, including two duplicated endpoints per
hole, checked before building bridges. A shared 8,000,000-credit work budget
bounds expensive bridge scans and ear point tests. Bridge scans are charged
conservatively before traversal; ear scans are checked after each candidate, so
one bounded scan can overshoot the credit threshold. Preprocessing/sorting is
vertex-bounded, not included in those credits. This is not a wall-time deadline
or an exact count of CPU instructions. Three-point degenerate output is refused.
No BSP fragment/output budgets or worker deadlines were increased.

This remains heuristic, bounded ear clipping, with superlinear worst cases and
existing fixed floating-point tolerances. It is not an O(n log n) triangulator,
an arbitrary-input validity proof, or a solution for all near-degenerate scales.

## WASM Results

Production `parseOpenSCAD`, two warmups and seven sequential measured calls per
case. Source SHA-256 values match before/after. Every successful iteration checks
finite analytic volume and closed manifold topology outside the timed interval.
Only the geometry WASM and packed geometry bytes changed among selected host
and artifact fingerprints. Compiler, Node worker transport and GPU are excluded.
No simultaneous test/build workload, profiling, forced GC or CPU isolation.

| Plate holes, fn=32 | Before p50, ms | After p50, ms | Result |
| --- | ---: | ---: | --- |
| 16 | 103.31 | 10.39 | 89.9% less time; avoids BSP fallback |
| 36 | 75.89 | 41.69 | 45.1% less time |
| 64 | refused | 111.36 | now supported, 8460 triangles |
| 100 | refused | 321.79 | now supported, 13212 triangles |

The two refusals occurred at the first warmup due to the stitch budget. Their
failure latency is not a successful-build baseline. Afterward relative volume
error is at most 6e-16; all four outputs have zero boundary/nonmanifold edges.
For 100 holes, evaluate p50 is 306.13 ms versus analyze 14.45 ms: evaluation is
still the dominant phase. The new capability is not a claim of general linear
scaling or lower memory usage.

Reports:

- `tmp/performance/profile-csg-wasm-before.json`
- `tmp/performance/profile-csg-wasm-after.json`

New optimized geometry WASM: 7,535,055 bytes (+1035 bytes); SHA-256
`1a2d3c6944e3bc2b8553039ee72ee5f81899557990313e1140f52fc6935e184d`.
The earlier artifact was
`7fe90559f9d65f10adc9d0edcb44cd6a7b8dae701c6827803f453cf10b74eaee`.

## Native Isolation

`npm run bench:profiles -- --out tmp/performance/profile-triangulation-final.json`
builds the locked release example, records selected sources, executable hash,
rustc/OS/CPU, and measures only `triangulate_profile`. Fixture construction,
validation and compilation are outside timing. Three warmups, nine samples.
The baseline is `tmp/performance/profile-triangulation-before-v2.json`.
Its raw fixture arrays match the final report exactly; it predates the complete
runner metadata, so this pair is exploratory rather than a fully attested build
comparison. Final fixture SHA-256:
`84de7cfcab3668389c16e7f5223db889b95142adf6e285a67ee1bf61c13614f3`.

| Profile | Before p50, ms | After p50, ms |
| --- | ---: | ---: |
| synthetic 1x1, 32 segments | 0.0097 | 0.0088 |
| synthetic 2x2, 32 | 0.1110 | 0.0711 |
| synthetic 4x4, 4 | 0.1450 | 0.0699 |
| synthetic 4x4, 32 | 0.5874 | 0.7451 |
| synthetic 7x7, 32 | 6.0387 | 4.2011 |
| synthetic 8x8, 32 | refused | 6.3239 |
| synthetic 10x10, 12 | 7.3446 | 3.5533 |
| synthetic 10x10, 32 | refused | 15.6001 |
| actual prism 4x4, 32 | refused | 0.3895 |
| actual prism 6x6, 32 | 3.4121 | 2.0632 |
| actual prism 8x8, 32 | refused | 6.7726 |
| actual prism 10x10, 32 | refused | 16.1727 |

The synthetic 4x4/32 case regresses by about 0.16 ms (27%). It is retained and
reported, not hidden by selecting only favorable fixtures. This local tradeoff
accompanies the bridge correctness fix and budget checks. The actual 4x4 prism
fixture differs in coordinates/order and must not be substituted with it.

## Correctness Contract

Shared native fixtures validate positive triangles, total area, bitwise-authored
coordinates (normalizing signed zero), every oriented input boundary segment,
and exactly paired interior edges. Volume alone would miss T-junctions. Tests
cover 1..100 holes, 4/12/32 segments, collinear boundary vertices, ring orientation,
hole order/rotation, reflections, translations and scales 0.001/1/1000. Actual
`prism_boolean` tests cover 4/16/36/64/100 holes and closed analytic volume;
production parser regressions cover 16/36/64/100 holes through WASM.

## Verification

- Release native tests for `planar-geometry`, `polygon-core`, `brep-core` and
  `geometry-bridge`: 1177 passed, zero failed. Log:
  `tmp/performance/profile-triangulation-native-final-tests.log`.
- Full Vitest, `--maxWorkers 2`: 3165 passed, 14 failed across 327 files,
  100.17 seconds. Failure titles exactly match the prior bootstrap run: nine
  historical fingerprint/qualification checks, two old B-rep refusal
  expectations, two MCP tool inventories, one mechanical image-attachment
  failure. No new test failures or worker readiness/join errors in this run.
  Log: `tmp/performance/profile-triangulation-full-tests.log`. Historical
  manifests were not regenerated to disguise mismatches.
- Application `vue-tsc`, MCP `tsc`, strict standalone CSG benchmark typecheck
  (including the repository HarfBuzz declarations), native runner syntax check,
  and `git diff --check` passed.
- Vite build passed. `verify-dist` still fails the existing total-size gate:
  6,335,050 bytes versus 5,600,000 allowed, +1050 bytes from the preceding build.
  Logs: `tmp/performance/profile-triangulation-vite-build.log` and
  `tmp/performance/profile-triangulation-verify-dist.log`.

The standard CPU suite also completes all four fixtures, three warmups/nine
samples, without other test/build workloads. Current build/analyze p50 in ms:
small bracket 2.05/0.58, medium CSG 114.14/9.56, 256 bodies 12.83/6.71,
48,384-triangle mesh 79.36/59.43. Report:
`tmp/performance/profile-triangulation-cpu/report.json`. The medium CSG case
now succeeds. Other numbers are a current snapshot, not an A/B claim against
earlier sessions; observed machine performance changed between series.

## Design Direction

Keep authored boundary identity separate from triangulation indices. In this
repository B-rep `triangulate_boundary` requires input-owned UV positions; the
existing planar sweep tessellator can add positions and is not a drop-in cap
triangulator. The small `clip_ears` helper separates bridge construction from
clipping without introducing a second public triangulation interface.

Two primary-source references checked on 2026-09-19:

- [Mapbox earcut.hpp](https://github.com/mapbox/earcut.hpp) uses spatial hashing
  and z-order acceleration, but documents possible nonconforming T-junctions
  and no correctness guarantee on arbitrary inputs. Inference for this project:
  a direct replacement needs our exact boundary/topology tests, not just faster
  rendering or area agreement.
- [Spade constrained Delaunay triangulation](https://docs.rs/spade/latest/spade/struct.ConstrainedDelaunayTriangulation.html)
  preserves constrained edges, with restrictions on intersecting constraints.
  Inference: a candidate backend still needs ring-domain selection, input
  validation, preserved boundary identity, resource limits, and artifact-size
  measurements. No Spade or Earcut dependency was added or benchmarked here.

A greenfield design would expose one boundary-preserving triangulation contract
with a typed refusal, independently testable domain selection/predicates, and
bounded native ownership. The next experiment should compare a proven CDT
backend against this fixture corpus, including adversarial geometry and WASM
size, before replacing the bounded implementation. General curved-body BSP,
subgraph caching, cooperative native cancellation and browser cold startup
remain separate work, not implied by these results.
