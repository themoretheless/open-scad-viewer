# Real SDF polygonization: small-input batching, 2026-10-20

Historical noisy experiment. A later CPU/GPU-quiescent qualification passed and
retained the integration: [clean-window results](../metal-small-sdf-clean-windows-2026-10-20/README.md).

**Decision: rejected. No production integration remains.** The pre-experiment
SDF/matcher/byte transport source was restored; the existing typed
`ComputeRuntime::write_small_parts` API from the previous turn is preserved.

Candidate: combine the five SDF GPU inputs (40-byte grid parameters, node kinds,
node parameters, aux indices and triangles) in one retained allocation when its
aligned binding ranges fit within 4096 bytes. A `gpu_compute::write_small_parts`
byte primitive packs the live data and zero padding into one queue write. The
WGSL and output layout do not change. Binding offsets use the device's required
storage alignment. On cache reuse, live lengths may shrink; padding includes
unused cached capacity. Larger inputs retain independent buffers/writes.

Four writes for a primitive field (triangle input empty), five for a mesh field,
become one. Allocation reuse and the submission boundary are unchanged. No
in-flight buffer is CPU-mapped, no GPU wait removed by assumption, and no update
is deferred through a submission. Existing synchronous readback remains intact.

## Full public workflow

The release fixture calls `polygonize_accelerated(field, grid, Acceleration::Gpu)`:
budget checks, `Field::to_flat`, parameter building, every input upload, dispatch,
output copy/map/readback/decode, CPU snapping/boundary checks, marching tetrahedra,
and final mesh allocation/drop all lie **inside** the timing. This is neither
an isolated upload API nor just the GPU sampler. Only device/pipeline setup,
fixture construction and first-capacity warm-up are outside the timing, equally
for both paths. The test-only thread-local override selects an actual baseline
or candidate sampler; it cannot return None and trigger CPU fallback. The override
and the old-path switch are absent from normal builds.

Cases: sphere and closed signed tetrahedron mesh, 4^3 and 8^3 cell grids.
Baseline and candidate mesh position bits and indices match before every timed
case. A separate regression also compares the raw f32 sample bits through
sphere -> tetrahedron -> sphere capacity reuse.

Same Metal device/queue, deterministic inputs, user-initiated thread QoS. Three
serial process runs, no own builds/tests during timing. Each case warms for
150 ms, then collects 32 counterbalanced ABBA/BAAB blocks. Each sample averages
16 **complete public calls**. A second old-path sampler supplies AA controls.
No timed logging or extra verification, no trimming/rejected runs. External
applications remain active; no frequency or thermal pinning. CSVs retain all
paired samples; the analyzer uses median log ratios and 4000 fixed-seed paired
bootstrap resamples. Binary/source hashes and hardware are in environment JSON.

The tiny test grids normally select CPU under Auto. Explicit GPU is used here
to guarantee the intended backend and forbid mixed CPU/GPU evidence. The same
small input arenas could also be used for large grids, but their end-to-end gain
is not inferred from these measurements.

## Reproduce

Build the release sdf-core lib tests with features gpu and run the already-built
executable through `run.py`. The runner selects only the ignored
`gpu::small_batch_tests::paired_polygonize` test. `analyze.py` reads the three
paired CSVs and retains every sample. Output files cannot be overwritten.

## Exploratory series and final acceptance rule

`paired-*` and `repeat-*` are both retained. The first stable series showed
roughly 3–6% gains. Under external contention some AA medians diverged as far as
0.539 or 1.398; run duration changed from 4.42 to 70.63 seconds without a source
change. Concurrent CPU-intensive compiler/test processes were observed. This
explains why those series cannot alone qualify a small end-to-end gain; exact
causal attribution to one process or GPU clocks is not established. No thermal
warning was reported by pmset, and memory_pressure reported 89% free at inspection.

Before the final series, the byte primitive was shared with the typed runtime
wrapper, Apple Silicon Metal gating was added, and regression coverage was
extended to >4 KiB fallback and raw byte-validation errors. Timing code and
workloads are unchanged. The `final-*` series uses that final binary.

Acceptance rule fixed before final timing: all four full-workflow cases must
have positive median speedup in all three final runs and a paired 95% interval
above 1 in at least two of three runs. Every final AA control must have median
within 5% of 1 and interval relative width <=10%. If these conditions fail, the
candidate is not retained. The two exploratory series remain visible regardless
of the final decision. There is no further repeat-until-fast loop.


## Final results and restoration

| Full workflow | Median speedup, three final runs | Runs with 95% interval above 1 |
| --- | --- | --- |
| Sphere, 4^3 cells | 1.034–1.080x | 0/3 |
| Sphere, 8^3 cells | 1.003–1.077x | 1/3 |
| Signed tetrahedron, 4^3 cells | 0.928–1.019x | 0/3 |
| Signed tetrahedron, 8^3 cells | 0.999–1.020x | 0/3 |

Final AA medians span 0.952–1.110, with broad intervals. The acceptance rule fails
both its effect-confidence and control-stability requirements. These data do
not establish a repeatable end-to-end improvement or a repeatable regression.
The candidate was therefore rejected, rather than qualifying the favorable
exploratory runs alone. No more timing series were run after this decision.

The descriptor-matcher candidate was likewise rejected after three complete
backend runs; see [matcher evidence](../metal-small-matcher-2026-10-20/README.md).
All raw observations are retained: three matcher series plus nine SDF series
(three exploratory, three repeat, three final), with AA controls throughout.

Before restoration, both candidates passed bitwise output equivalence checks.
Final SDF regression tests additionally passed raw-byte validation, the 4096-byte
boundary, oversized/misaligned/out-of-range errors before mutation, shrinking
capacities, arena growth beyond 4 KiB and fallback reuse. See correctness.log and
final-correctness.log. The ignored benchmark tests were explicitly run by the
runner; they are not silently skipped GPU evidence.

`restored-source-sha256.json` records exact equality with the saved pre-experiment
runtime/SDF/matcher sources. Git diff also confirms no changes in the byte
transport, SDF backend or matcher after restoration. Other concurrent repository
changes were not modified. No new source change requiring broader compilation
remains from this experiment.

To reproduce the final rejected candidate, apply `final-candidate.patch` and copy
`final-small-batch-tests.rs` to `crates/sdf-core/src/gpu/small_batch_tests.rs`, then
build the sdf-core release lib tests with feature gpu. The final patch includes
only this experiment's runtime-wrapper delta, preserving the preceding typed
API. `candidate.patch` and `small_batch_tests.rs` preserve the earlier prototype.
The top-20 CPU process summaries before/after repeat/final runs retain CPU load
and executable names; unrelated project paths have been removed from them.
