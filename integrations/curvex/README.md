# Curvex replacement qualification

The latest [2D architecture and performance qualification](architecture-qualification/README.md) records the transport split, prepared geometry indexes, final benchmarks and verification limits.

The replacement contract is in [curvex-2d-replacement.md](../../docs/design/curvex-2d-replacement.md).
This directory contains the source inventory, a reproducible native migration,
rendering qualification fixtures, and application-level performance workloads.
The contract defines the evidence required to qualify a replacement.

**Qualified against the recorded source revision.** See the
[result summary](qualification-results/summary.json),
[migration patch](qualification-results/curvex-migration.patch), and
[render comparison](qualification-results/render-contact-sheet.png).

The [2026-09-13 stroke optimization](stroke-optimization/README.md) adds direct,
certified ribbon meshes and fresh native, Curvex, WASM and renderer validation.
Its measurements and source hashes are separate from the original qualification.

The latest [complex-stroke and GPU qualification](stress-qualification/README.md)
adds numerical robustness fixes, bounded large-document caching, and native
Metal captures for up to 5000 visible shapes. Use its
[updated migration patch](stress-qualification/results/curvex-migration.patch)
for these changes; earlier patches remain historical evidence.

The independent implementation is in `crates/planar-geometry`; its only runtime
dependency is the shared `osv-math` crate. Removing the four direct geometry
dependencies is distinct from removing transitive Kurbo users in the GUI/SVG
stack. Dependency evidence records those reverse chains explicitly.

Create a fresh isolated migration with a reviewable patch:

```sh
python3 integrations/curvex/prepare.py /path/to/curvex /tmp/curvex-osv
```

The destination must not already exist. The command keeps the original source
read-only, routes the four direct geometry dependencies to `planar-geometry`,
adds the gradient sampling integration, and writes `osv-migration.patch` plus
`osv-migration-source.json` into the destination. It preserves the original
test assertions.

Run all qualification gates on a fresh isolated migration:

```sh
python3 integrations/curvex/qualify.py /path/to/curvex /tmp/curvex-qualified --target /tmp/curvex-build
```

The runner performs native Curvex tests, doctests, all-target checks, a release
build, dependency-closure checks and real renderer capture. It records command
exit codes, logs, compiler, source hashes, test names and the updated lockfile
patch. `--baseline` optionally takes a separate isolated original copy for a
release renderer comparison. `--resume` refreshes a previously staged migration
from the verified original and reruns every gate, preserving prior evidence.

The gradient integration samples nonlinear colors inside triangles and splits
repeated linear/conic seams explicitly. It attaches samples to the existing
geometry cache entry, retaining its capacity and revision behavior. Additional
actual-renderer tests cover recoloring, translated seams and 300 cached shapes.

`render_qualification.rs` executes Curvex's actual `draw_shape` and
`draw_shape_cached` paths and egui tessellation at three zoom levels. It covers
solid/gradient fills, nested holes/islands, self-crossings, open/closed strokes,
retained cubic Boolean output and shadow ghosts. Paint/frame CPU times exclude
JSON serialization.

`compare_render.py` requires numpy and Pillow. It rasterizes captures using a
top-left triangle rule, checks coverage and cache invariance, compares gradients
to an independent analytic oracle, and saves PNGs and metrics:

```sh
python3 integrations/curvex/compare_render.py baseline-render.json migrated-render.json /tmp/curvex-render-comparison
```

This verifies geometry and colors without a GPU; GPU antialiasing is not part
of the comparison. The original radial/conic/repeated gradients were coarse
and depended on triangulation diagonals. Their old pixels are retained as
evidence; analytic color is the correctness oracle. Adaptive sampling has work
limits: arbitrarily narrow radial/conic stop bands are not guaranteed.

Regenerate the complete inventory against the source being migrated:

```sh
python3 integrations/curvex/inventory.py /path/to/curvex > /tmp/curvex-2d-inventory.json
```

`baseline-inventory.json` records the original reviewed source revision and the
SHA-256 of every Rust file in `src`, `tests`, `benches`, and `examples`, plus the
manifest. It includes every lexical reference to the four direct geometry
dependencies. Comment-only matches are separated from candidate code matches.
Method calls and imported aliases are described in the contract and must also
be verified by compiling the migrated project.

Qualification must take place on an isolated copy or checkout. The original
Curvex source need not be modified to make a concrete, reviewable migration.
Keep the patch, original revision, dependency-tree evidence, commands, exit
codes, test totals, and performance/visual results together here when available.

The actual Curvex Boolean benchmark builds the same
[`boolean_benchmark.rs`](boolean_benchmark.rs) example in an isolated original
copy and the migrated copy. It times original application entry points for
ellipse unions, rounded subtraction, 50 ellipse cutters, three-circle Divide,
compound normalization, and a self-intersecting cubic. It records seven
calibrated samples per workload in original/migrated/migrated/original order,
output contour/curve counts, source and executable hashes, and build logs:

```sh
python3 integrations/curvex/run_boolean_benchmark.py /tmp/curvex-original /tmp/curvex-osv /tmp/curvex-benchmark --target-dir /tmp/curvex-target
```

Use isolated copies for both benchmark arguments: the runner adds the example
to each one. `kernel_source_unchanged_during_build` must be true for a frozen
source qualification. All raw samples remain in `comparison.json`; noisy or
intermediate runs must not be relabeled as final evidence.

After other builds and benchmarks stop, repeat the measurements without
rebuilding by adding `--replay-from /tmp/curvex-benchmark` and selecting a new
output directory. The runner verifies the retained executable hashes, checks
that the fixture source still matches, and preserves the original build and
source hashes alongside the fresh raw samples. Current source changes are not
implicitly included in a replay: rebuild first when they affect the workload.

The final [interaction measurements](benchmark-results/interaction-final.json)
use three original Criterion workloads for drag invalidation, warm hit testing
and warm scissors hover on 500 complex paths. The report preserves build/run
commands, executable and source hashes, per-run confidence intervals and links
to the raw Criterion samples. These measurements exclude painting; direct
stroke/fill costs are recorded in
[render-final.json](benchmark-results/render-final.json).

Before publication, the staged commit was exported to an isolated directory
and tested with the repository's pinned nightly toolchain: 141 planar unit,
18 Curvex parity, 45 polygon unit, 13 polygon integration and 8 bridge path2d
tests passed. The earlier frozen qualification captured a shared working tree
that also contained an unrelated BRep triangulation module and its one test.
That module and export are excluded from this replacement commit; the historical
source hashes and 142-test log remain unchanged as provenance.

## Retained GPU / Metal qualification

The [Metal renderer report](metal-renderer/README.md) records the completed
Apple M5 qualification: retained geometry/batching, 288 exact pixel comparisons
including Retina, ten lifecycle phases, 1494 application tests, native startup
and fallback, plus full-canvas measurements through 5000 shapes. The original
Curvex checkout remains unchanged; the report includes the verified migration
patch, payload budgets, source hashes and platform limits. For the kernel and
transport architecture, see [the 2D follow-up](architecture-qualification/README.md).
