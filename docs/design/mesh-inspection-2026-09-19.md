# Exact mesh inspection: 2026-09-19

Continuation of [dense display preparation](mesh-render-2026-09-19.md).
This step targets the shared polygon kernel's indexed-topology inspection,
not rendering or a reduction in mesh validation.

## Evidence and Change

The extended `bench:analysis` separates uncached kernel inspection from display
analysis and host asset hashing. On the retained 48,384-triangle sphere fixture,
baseline inspection takes 16.52 ms versus 2.48 ms for asset hashing. The normal
`CadSolid` wrapper caches its first inspection report; the benchmark deliberately
calls the production `cad/inspect` command directly, so repeated measurements
exercise the actual work rather than the wrapper cache. This command includes
volume/topology, surface area, bounds and response transport.

The former `Mesh::edges` built a `BTreeMap<(usize, usize), Vec<[usize; 2]>>`.
Every distinct edge owned a small vector. `inspect` traversed it to count
boundary/nonmanifold/orientation problems, then dropped it. `boundary_loops`
and `thicken` immediately rebuilt that same structure to consume boundary edges.

`mesh_topology::EdgeUses` now stores all directed edge occurrences in one vector,
sorts it by canonical endpoint pair, then walks contiguous groups. Unstable tie
order is acceptable because only multiplicity, direction equality for two uses,
and single-use boundary edges are observed. Boundary edges retain the same
canonical ordering as the tree; thickened output indices do not change.

The retained representation packs each occurrence into one `u64`: canonical
endpoints occupy 31/32 bits and direction occupies the last bit. A compile-time
assert binds that representation to the existing 300,000-vertex admission
limit; all accepted indices fit. Grouping ignores the direction bit, while
two-use orientation compares it. Tests include the largest admitted endpoint.
The first experiment used `[usize; 2]` with a custom sort key; the packed scalar
uses the standard integer sort and proved both faster and smaller in WASM.

`Mesh::inspect_with_edges` returns the existing report together with this
temporary index. Public `inspect` drops the index; boundary and thickening
operations reuse it within the same call. They explicitly drop it after
extracting their boundary work, before later allocation/inspection stages.
There is no cache on mutable `Mesh`, no invalidation protocol, new public API,
dependency, unsafe code, or relaxed resource limit.

Validation still precedes allocation of the topology index. Volume accumulation
and degeneracy arithmetic keep their exact order and tolerances. Coordinate
equality does not weld distinct vertex indices. Empty meshes remain not closed;
edges used more than twice remain nonmanifold; orientation conflicts are counted
only for two-use edges, exactly as before. The report still says self-intersection
is `not_checked` and error bounds are not certified. This is not a new geometric
correctness certificate or an exact-predicate Boolean implementation.

The algorithm remains O(T log T), using O(T) temporary storage. Native allocation
counts and peak RSS are not measured here; fewer allocation sites and less
repeated construction must not be presented as a measured peak-memory reduction.

## Correctness Contract

Native tests compare all grouped directed occurrences, counts and ordered
boundary edges against the original tree implementation. Cases include empty
input, orientation conflicts, open and nonmanifold meshes, repeated/self edges,
reversed order, a dense sphere, 100,000 repeated triangles, and 30 deterministic
triangle soups over several index ranges. Existing input/resource checks remain.

Integration regressions check unchanged boundary-loop and thickened-index order,
diagnostics without coordinate welding, ambiguous-boundary refusal, malformed
indices/UVs and nonfinite coordinates. The public WASM API has matching cases in
`geometryInterop.test.ts`. Tests also exercise Boolean, surface-conversion and
slicer callers, since they share `polygon-core` inspection.

## Measurements

M4 Max, darwin arm64, Node 22.23.2 / V8 12.4.254.21. Each side has three warmups
and nine samples, run sequentially without overlapping test/build jobs or
profiling. No forced GC, CPU isolation or frequency control. Validation and
verification hashing are outside timing. This is warm local behavior, not
browser edit latency or cold startup.

Uncached inspection through the same production command, p50 milliseconds:

| Solid | Original tree | Pair-array candidate | Retained packed array |
| --- | ---: | ---: | ---: |
| cube | 0.0248 | 0.0237 | 0.0236 |
| sphere, 32 segments | 0.321 | 0.297 | 0.193 |
| sphere, 128 | 5.258 | 3.127 | 2.301 |
| three spheres, 128 | 16.518 | 10.119 | 7.026 |
| cylinder, 128 | 0.0977 | 0.0709 | 0.0509 |

Dense inspection is 57.5% faster in the retained candidate. All 35 phase
input/output signatures match both alternatives and the original, including
the complete inspection response. Unchanged dense render replay varies from
21.334 to 20.348 ms, BVH from 12.045 to 11.888 ms and asset hashing from 2.476
to 2.411 ms, so the environment is not perfectly stationary. Tiny-body
percentages are not a claim of user-visible latency improvement.

Whole production parser/build, same source and output signatures:

| Fixture | Before build, ms | Packed build, ms | Before analyze, ms | Packed analyze, ms |
| --- | ---: | ---: | ---: | ---: |
| small bracket, 540 triangles | 1.911 | 1.700 | 0.468 | 0.402 |
| 64-hole plate, 8460 triangles | 112.176 | 97.283 | 8.047 | 6.289 |
| 256 cubes, 3072 triangles | 12.241 | 12.139 | 6.093 | 5.940 |
| dense mesh, 48384 triangles | 72.773 | 56.391 | 52.885 | 44.823 |

Dense whole-build p50 decreases by 22.5%, the plate by 13.3%. Dense evaluation
also decreases from 19.728 to 11.423 ms: primitive/Boolean construction uses
the same inspection implementation, so the gain is not confined to the final
analyze stage. The cube-heavy result is effectively unchanged. The rejected
pair-array candidate measured 12.719 ms on that fixture (about +3.9% versus
baseline); it was not retained or silently omitted from the raw evidence.

All four CPU fixture definitions, complete geometry metrics/bytes and replay
signatures (BVH, edges, inspection, STL and other stages) match. Independent
phase replays must not be added together to explain the whole build.

Final WASM: 7,530,902 bytes, 388 bytes smaller than the baseline. The pair-array
candidate was 7,549,576 bytes. Retained SHA-256:
`48d78fc477a26165b56649084cd7acf7545fa73dce7d7bd3ae6637dcd34192b7`.
Baseline SHA-256:
`93c2a2ef5fc9cd7c321b91ddc0be0845a4d537d743fc2618664d3977b67fe222`.

Raw results:

- `tmp/performance/mesh-inspect-before.json`
- `tmp/performance/mesh-inspect-after.json` (pair-array experiment)
- `tmp/performance/mesh-inspect-packed-after.json` (retained)
- `tmp/performance/mesh-inspect-cpu-before/report.json`
- `tmp/performance/mesh-inspect-cpu-after/report.json` (pair-array experiment)
- `tmp/performance/mesh-inspect-packed-cpu-after/report.json` (retained)

## Verification

- Release tests for `polygon-core`, `geometry-bridge` and `slicer-core`: 383
  passed, zero failed. This covers both direct dependent crates of
  `polygon-core`, not every Rust workspace package. Final log:
  `tmp/performance/mesh-inspect-packed-native-tests.log`.
- Full Vitest, two workers: 3173 passed, 14 failed, 328 files, 102.51 seconds.
  Failure titles exactly match the preceding display-preparation run: nine
  historical fingerprint/qualification checks, two old B-rep refusal
  expectations, two MCP inventories and one mechanical image-attachment case.
  There are no new failures or lifecycle errors in this run. Public WASM
  boundary/thickening regressions passed on the final artifact. Log:
  `tmp/performance/mesh-inspect-packed-full-tests.log`.
- Application and MCP typechecks, strict benchmark/interop-test typechecks,
  Vite build and `git diff --check` passed. Selected source/artifact hashes
  were verified again after the tests and still match the measured report.
- `verify-dist` still fails its total-size limit: 6,333,440 bytes versus
  5,600,000 allowed. Packed output is 200 bytes larger than the previous
  build despite the smaller raw WASM; raw and compressed size are separate
  measurements. Logs: `tmp/performance/mesh-inspect-packed-vite-build.log`
  and `tmp/performance/mesh-inspect-packed-verify-dist.log`. Historical
  qualification artifacts were not rewritten. The overall gate remains red.

## Reproduction

```sh
npm run bench:analysis -- --out tmp/performance/mesh-inspect-NEW.json
npm run bench:cpu -- --warmups 3 --out tmp/performance/mesh-inspect-cpu-NEW
```

Both runs must follow a fully completed geometry build, including wasm-opt and
packing, with no overlapping compile/test jobs. Baselines are
`tmp/performance/mesh-inspect-before.json` and
`tmp/performance/mesh-inspect-cpu-before/report.json`. Reports include input and
output signatures, selected source/artifact manifests, environment and all raw
samples. These are local measurements, not browser or production qualification.
