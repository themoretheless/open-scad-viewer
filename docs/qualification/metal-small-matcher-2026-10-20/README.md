# Qualification of real small-update call sites, 2026-10-20

**Decision: rejected; original production matcher restored.**

## Audit

The existing `ComputeRuntime::write_small_parts` joins contiguous ranges of one
destination, not unrelated buffers. Native/portable Rust queue-write call sites
were searched (excluding target and saved benchmark-source copies).

| Caller | Existing updates | Direct safe application |
| --- | --- | --- |
| compute-core runtime and low tensors | one contiguous array per call | already one write; no adjacent sequence to merge |
| math bounds/moments/distance/neighbors | params, points/queries/targets in separate allocations | requires new input arena and binding ranges; several math files are wasm-only |
| SDF and polygon lattice | params and independently sized node/triangle/aux allocations | cannot concatenate writes without changing the binding layout/allocation scheme |
| raster scene | one cached scene write | unchanged scenes already skip the write |
| raster style/morph | distinct fields in one object uniform | no production Rust call site updates both together; adding a combined setter alone would not integrate anything |
| descriptor matcher | params + A/B descriptor arrays before two GPU dispatches | candidate: one small input arena, explicit nonoverlapping binding ranges |

## Candidate scope and safety

The matcher candidate retains one arena only when aligned header plus cached A/B
capacities fit in 4096 bytes. Header uses storage-binding offset alignment; each
descriptor is 512 bytes. A/B bindings remain read-only and disjoint; shaders and
outputs are unchanged. All writes still precede the same submission. Readback
buffers, two map operations, CPU decoding and errors remain the same. The B offset
is capacity-based, so shrinking A writes padding through its unused capacity.
Growth beyond 4 KiB retains the original independent-buffer path. No direct CPU
mapping of an in-flight destination or deferred write is introduced.

Important scope limit: `features::recommended_for_descriptor_matching` chooses CPU
below 10000 candidate pairs. Thus tiny grids use this GPU path only with explicit
GPU selection or direct GpuMatcher calls. This candidate cannot improve default
Auto matching at these sizes and must not be presented as such.

## Full backend measurement

The release unit-test fixture calls the actual `GpuMatcher::try_match_descriptors`
method, including capacity checking, all uploads, two compute dispatches, output
copies, submission, both synchronous map/read operations, decoding and output
allocation/drop. No upload-only timer is used. Pipeline construction and first
allocation occur outside warm replay timings, equally for both paths.

Both matchers share one Metal device/queue. A test-only switch selects the old
allocation/write path; it is absent from normal builds. Baseline code retains the
original parameter concat and three independent writes. A third baseline matcher
provides AA controls. Inputs and outputs are bitwise checked before warm-up.

Cases: 1x1, 2x2, 3x4, 4x3 eligible small grids; 4x4 and 16x16 original-path controls.
Per case: 150 ms warm-up; 32 ABBA/BAAB blocks, each sample averages 16 complete
calls; results are buffered until timing is complete. Three sequential runs with
no own builds/tests while timing. User-initiated thread QoS on macOS; no core
pinning, thermal lock or exclusive desktop access. Other applications may be
active. No outlier removal or run selection. `analyze.py` calculates median
paired log ratios and 4000 fixed-seed bootstrap resamples.

This measures the complete GPU matching backend, not feature extraction, pair
filtering, image reconstruction or default Auto processing. Further integration
requires that this complete backend first show a stable benefit.

## Decision: rejected

No stable full-backend improvement across three runs: eligible-grid median ratios 0.990–1.123 with almost all 95% intervals crossing 1; AA medians 0.959–1.053. See paired-summary.csv. The production matcher is restored byte-for-byte to its pre-experiment source. The rejected delta and fixture remain here for reproducibility. To reproduce, apply rejected-matcher.patch and place small_batch_tests.rs under crates/photogrammetry-core/src/gpu/matching/. The common byte helper is recorded in the adjacent SDF candidate evidence.
