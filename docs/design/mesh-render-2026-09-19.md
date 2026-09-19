# Dense display preparation: 2026-09-19

Continuation of [retained-solid analysis](solid-analysis-2026-09-19.md).
This change optimizes an existing Rust stage, not a new TypeScript-to-Rust
port. It preserves all display-buffer bytes and the existing analysis ABI.

## Evidence and Scope

The current dense fixture has 48,384 triangles. A fresh production-parser
baseline measured build p50 77.15 ms, including 57.97 ms in analyze. The CPU
profile spends substantial sampled time under `analyzeSolidInKernel`; stripped
WASM function numbers alone do not identify every Rust source function.

The new `bench:analysis` runner therefore measures the existing export, render,
BVH, semantic-edge and combined bindings separately. For the same three-sphere
solid, render took 25.82 ms, export 9.23 ms, BVH 11.61 ms and edges 2.37 ms.
Export is included in render; independent BVH/edge replays include uploads and
host copies absent from the combined call. These numbers are diagnostic
replays, not an additive decomposition. Solid construction, metrics, provenance,
host asset hashing, cold startup, worker transport and GPU are excluded.

## Implementation

`mesh_render.rs` owns detached display preparation, with no access to handle
registries or persistent scratch buffers. `mesh.rs::render_buffers` remains the
small adapter that acquires the export snapshot. The result is still leased
and released by the existing analysis registry; host arrays remain exclusively
owned copies that survive source deletion and WASM memory growth.

Two local data-structure changes replace allocation-heavy lookup paths:

- Adjacency uses counts, prefix offsets and one flat incident-triangle array
  instead of a `Vec` allocation per source vertex. Repeated incidences and
  original triangle order are retained, including degenerate triangles.
- Rounded-normal properties are indexed under each source vertex using links
  in a flat property array. This replaces the global `(source, normal)` hash
  table. First-property indices directly produce merge pairs, eliminating the
  second source-id hash table and the intermediate raw-id stream.

Output indices still follow first encounter, normals still accumulate in
triangle order, and the 1e-7 rounding key is unchanged. `Math.hypot`/`Math.round`
compatibility helpers retain their exact operations. Similar-looking helpers
elsewhere are not consolidated without checking their numerical contracts.
No new unsafe code, dependency, cache, worker policy or resource-limit change.

This does not make smoothing linear in arbitrary topology. Normal accumulation
is still O(sum of squared vertex incidences); property lookup is linear in the
number of distinct normal groups at that source vertex. Typical smooth vertices
have one group, but high-valence crease-heavy inputs are not claimed to have
constant-time lookup. Resident/peak native memory and allocation counts were
not measured, so fewer small allocation sites are not a quantified memory win.

## Measurements

Apple M4 Max, macOS arm64, Node 22.23.2 / V8 12.4.254.21. Each side has three
warmups and nine sequential timed samples, without simultaneous builds/tests,
profiling or forced GC. CPU frequency/isolation is not fixed. Validation and
SHA-256 checks happen outside each timed interval. The separate CPU profiling
pass ran only after the baseline timing samples were complete.

| Retained solid | Render before, ms | Render after, ms | Combined before, ms | Combined after, ms |
| --- | ---: | ---: | ---: | ---: |
| cube | 0.022 | 0.012 | 0.033 | 0.023 |
| sphere, 32 segments | 0.534 | 0.378 | 0.697 | 0.506 |
| sphere, 128 segments | 8.679 | 6.626 | 14.446 | 11.796 |
| three separated spheres, 128 | 25.822 | 20.287 | 39.028 | 34.515 |
| cylinder, 128 segments | 0.248 | 0.181 | 0.371 | 0.268 |

Dense render p50 is 21.4% lower, combined analysis 11.6% lower. Unchanged dense
export/BVH/edges vary by +4.2%/+1.4%/+4.1% between runs, showing the measurement
environment is not perfectly stationary. Tiny-body percentages are not a
product-latency claim.

Production `bench:cpu`, identical source and complete geometry/replay signatures:

| Dense fixture stage | Before p50, ms | After p50, ms | Change |
| --- | ---: | ---: | ---: |
| whole build | 77.149 | 73.849 | -4.3% |
| evaluate | 19.266 | 19.993 | +3.8% |
| analyze | 57.974 | 53.609 | -7.5% |

All 25 retained-solid phase input/output hashes match between runs. The CPU
runner also matches geometry, BVH, edges, inspection and STL signatures.
Dense geometry SHA-256 remains
`fe8442fa6ea5f08f7075a04252b7d8123309c818ccf4fa2a5003e38f735f9483`.
Raw reports and profile:

- `tmp/performance/render-analysis-before-v2.json`
- `tmp/performance/render-analysis-after.json`
- `tmp/performance/dense-analysis-profile-before/report.json`
- `tmp/performance/dense-analysis-profile-before/dense-sphere/build.cpuprofile`
- `tmp/performance/dense-analysis-after/report.json`

Current WASM is 7,531,290 bytes, 3765 bytes smaller. SHA-256:
`93c2a2ef5fc9cd7c321b91ddc0be0845a4d537d743fc2618664d3977b67fe222`.
The baseline artifact was
`1a2d3c6944e3bc2b8553039ee72ee5f81899557990313e1140f52fc6935e184d`.
Selected-source manifests and actual WASM hashes accompany the reports;
they are not a complete reproducible-build attestation.

Verification on this final artifact:

- All 265 release `geometry-bridge` tests passed, including the three new
  native parity cases. Log: `tmp/performance/mesh-render-native-all-tests.log`.
- 184 targeted TS tests passed. Full Vitest with two workers: 3171 passed,
  14 failed, 328 files, 100.26 seconds. The 14 failure titles exactly match the
  preceding triangulation run: historical fingerprints/qualification, old
  B-rep refusal expectations, MCP inventories and mechanical image attachments.
  No new failures or lifecycle errors in this run. Logs:
  `tmp/performance/mesh-render-targeted-tests.log` and
  `tmp/performance/mesh-render-full-tests.log`.
- Application/MCP typechecks, strict benchmark/oracle typechecks, Vite build
  and `git diff --check` passed. Selected hashes were rechecked against the
  measured report after verification; runtime sources/artifacts still match.
- `verify-dist` still fails the existing total-size gate: 6,333,240 bytes
  versus 5,600,000 allowed. This is 1810 bytes less than the preceding build,
  not enough to close the gate. Logs: `tmp/performance/mesh-render-vite-build.log`
  and `tmp/performance/mesh-render-verify-dist.log`. Historical qualification
  evidence was not rewritten; this is not release or production qualification.

## Correctness and Next Work

Native tests compare every output buffer against the original map-based
algorithm, using f32 bit patterns rather than numeric equality. They cover
empty/unused vertices, repeated incidences, degeneracy, primitive/dense meshes,
reversed triangle order, 257-way crease vertices and multiple cosine thresholds.
An independent TypeScript oracle uses native JavaScript `Math.hypot` and
`Math.round`; it passed on both baseline and rebuilt WASM. It covers 0/52.5/90/180
degree thresholds, a dense sphere and reflected/nonuniformly scaled/rotated
solids with large translations. Existing analysis tests retain lease/free,
memory.grow and transfer checks. The source registry's ownership did not change.

The next dense-path questions are planar face grouping during export, metrics
inspection before the combined ABI call, and host asset hashing afterward.
The gap between the isolated combined call and parser analyze includes these
operations; it must not be attributed to one function without measuring it.
The larger design issues remain shared evaluation semantics, native cancellation
and incremental graph ownership. A pure display module improves testability
but does not solve those independent contracts.

Continuation: [exact indexed inspection](mesh-inspection-2026-09-19.md) measured
the metrics path and replaced its per-edge tree allocations with a packed sorted
index, reused within boundary/thickening operations. The other directions above
remain separate work.
