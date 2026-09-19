# Surface grouping allocation reduction

`inferSurfaceIds` remains a TypeScript connected-smooth-patch algorithm. This
change removes triangle-local index/vector/normal arrays and callback-driven
parent initialization. Cross products and normalization use scalars with the
same arithmetic order. Parents are initialized in the existing triangle loop.
Welding, edge keys, edge counts/orientation, union ordering, angle threshold,
surface numbering, authored-ID handling and cache behavior remain unchanged.

## Measurements

`node --import tsx benchmarks/surface-groups.mts [module-path]` measures uncached
grouping on deterministic wavy grids. Fixture creation and equality checks are
outside the clock; algorithm allocation and GC are inside. Two warmups and
nine samples per fixture; two sequential baseline/candidate runs, with no
simultaneous builds or tests. Each report records source/input/output hashes.

| Triangles | Baseline median ms | Candidate ms | Baseline repeat | Candidate repeat |
| ---: | ---: | ---: | ---: | ---: |
| 2048 | 1.996 | 0.424 | 1.773 | 0.424 |
| 32768 | 17.414 | 9.231 | 17.148 | 9.368 |
| 80000 | 45.644 | 25.789 | 46.934 | 28.414 |

The large fixture improves approximately 39-44%. This is a Node microbenchmark,
not browser frame time, total publication latency, or a universal mesh claim.
Small-fixture results are especially sensitive to JIT state. The unchanged
100000-triangle limit remains; this does not constitute a Rust port.

Raw reports: `/private/tmp/osv-surface-groups-{before,after}.json` and
`/private/tmp/osv-surface-groups-{before,after}-repeat.json`.
The saved reference module is `/private/tmp/osv-meshSurfaceGroups-baseline.ts`,
with its unchanged cache dependency copied alongside. All fixture input and
surface-ID output hashes match between baseline/candidate runs.

## Verification

- 3000 seeded comparisons (1000 indexed meshes at 0/30/60 degrees) matched the
  saved baseline's IDs exactly. This is additional local differential evidence,
  not a substitute for the committed geometry tests.
- Surface-group/cache tests: 12 passed, including existing cylinder/cube/spinner
  cases and new exact-duplicate/signed-zero, winding, degeneracy, empty-input,
  invalid-index, nonfinite-position, stride and angle cases.
- Typecheck, production Vite build and dist identity/budget gates passed.
- Production exact-solid browser scenario passed after the change.

Remaining cost includes string-key coordinate welding and edge-map allocation.
A future Rust port must measure transport/warmup overhead and preserve these
selection semantics rather than only compare the inner loop.

## Sparse vertex buffers and edge-key correctness

A subsequent inspection found that welding every unused position can assign
IDs beyond the numeric edge radix 2097152 even with only three triangles.
For example `(0,2097154)` and `(1,2)` produce the same numeric edge key. A
reproduced input with a connected coplanar pair plus a degenerate triangle
returned `[0,1,2]` before the fix instead of `[0,0,1]`.

Welding now lives in the module-local `canonicalVertexIds` function. When
vertex count exceeds index count, it reuses the canonical array as a temporary
reference bitmap and assigns weld IDs only to referenced vertices. Otherwise
all vertex IDs already fit under the maximum 300000 index references. Thus both
paths keep weld IDs below the radix, without adding another large bitmap.
All input coordinates are still checked for finiteness, including unused ones.
Dense meshes do not pay the extra reference-marking pass.

The benchmark gained a 2048-triangle fixture with 100000 unused vertices.
Against the already scalar-optimized baseline, final results were:

| Triangles / unused vertices | Control median ms | Final | Final repeat |
| --- | ---: | ---: | ---: |
| 2048 / 0 | 0.447 | 0.400 | 0.704 |
| 32768 / 0 | 9.517 | 9.421 | 9.194 |
| 80000 / 0 | 25.910 | 24.771 | 25.376 |
| 2048 / 100000 | 13.628 | 0.597 | 0.611 |

No speedup claim for ordinary dense inputs: small-fixture timing varies and
their main algorithm is unchanged. The sparse synthetic case improves about
22x. Initial all-input marking / inline-welding variants regressed the dense
case and were replaced by the final sparse-only, separated welding path.
Every benchmark input and output hash matches the scalar baseline; the explicit
edge-collision regression intentionally changes the previously wrong output.

Final reports: `/private/tmp/osv-surface-sparse-final-control.json`,
`/private/tmp/osv-surface-sparse-split.json`, and
`/private/tmp/osv-surface-sparse-split-repeat.json`. The reference is saved as
`/private/tmp/osv-meshSurfaceGroups-scalar.ts`.
Fourteen surface-group/cache tests, typecheck, Vite build and dist gates passed.
This does not bound total vertex-buffer scan time or claim the sparse workload
is common in production. The large unused-position map is avoided, not all
input-proportional memory or work.
