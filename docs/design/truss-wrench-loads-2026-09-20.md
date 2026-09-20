# Explicit Truss Forces and Moments

`mechanics-core::truss_loads` replaces the legacy scenario's heuristic moment
conversion with a bounded, resultant-preserving nodal load assembly. It is
connected to geometry WASM and the existing shared CAD worker, not yet to
workbench scenario controls. The old support-selector, material factors and
strength recommendations are not imported.

## Contract and Method

`TrussInput` accepts either the original per-node `forcesN` model or a
`TrussWrenchModel` with `loads`. Each load specifies unique selected node indices,
an explicit `originMm`, `forceN` and `momentNmm` in global XYZ axes. Units are
N and N mm, not N m. There are 1-32 loads, each selecting 1-125 existing nodes.
The existing model limits of 125 nodes/400 members remain unchanged.

Both `solveTruss` and `computeTrussAnalysis` accept this union. The native operation
`truss_solve_wrenches` admits exactly `op`, `nodesMm`, `members`, `restrained` and
`loads`; each load admits exactly `nodes`, `originMm`, `forceN`, `momentNmm`.
Providing both `forcesN` and `loads`, unknown fields, duplicate/out-of-range node
indices or an excessive count fails closed. Signed loads are superposed before
the existing stiffness solve; overflow of the combined force is refused.

The explicit origin matters: a force applied away from it contributes a moment.
A caller asking for zero moment about that origin does not implicitly mean a
force at the selected points' centroid. The assembly honors both specified
resultants, adding a force couple where needed and possible.

Implementation:

- Compute the selected points' centroid using local offsets, and transform the
  requested moment to that centroid.
- Share the net force equally and compute a minimum-squared-norm force couple.
  Centered coordinates are normalized by their maximum radius.
- Use the existing nalgebra symmetric eigensolver on the 3x3 matrix
  `sum(|r|^2 I - r r^T)`, with at most 64 iterations.
- Numerical rank threshold is `1e-12` times the largest eigenvalue. A requested
  couple outside the admitted span by more than `1e-10` relative residual is
  refused, never silently dropped.
- Recheck total force and moment about the supplied origin with componentwise
  backward residual bound `1e-9`. This is not a forward-error certificate.

Rank deficiency has a different meaning here than in the structural solver: two
selected nodes can represent a couple perpendicular to their joining line but
not torsion along it. One point cannot represent a pure couple. Coincident points
can share a force but cannot add a couple. These cases return
`TRUSS_LOAD_UNREALIZABLE` where appropriate; nearly rank-deficient selections can
also be refused. No rotational DOFs, member bending or artificial stiffness are
introduced. The structural solver's singularity refusal is unchanged.

## Verification

- All 23 mechanics-core tests pass in debug/release (18 unit, 5 section tests).
- Nine new load-assembly tests include the exact three broken legacy moment
  axes, an analytical two-node couple, a single-node refusal and combined loads
  about an offset origin.
- 140 rotation/scale combinations from `1e-250` through `1e250` preserve force
  distributions; rotated collinear axial-torque cases are refused. Translation
  by 1e9 mm and input reordering are checked separately.
- A 125-node/366-member integration test assembles loads, solves the structure
  and independently checks that support reactions cancel both resultants.
- All 265 geometry-bridge tests pass; the adapter also tests 32-load admission,
  33-load refusal, invalid selections, ambiguous schemas and aggregate overflow.
- Thirty focused actual-WASM/worker/protocol tests pass. These include a real
  nonzero force couple on two independently supported tripods, signed load
  superposition and typed unrealizable-load recovery in the same worker.
- Full Vitest: 3,363 passed, 9 failed across 351 files in 109.13 seconds.
  The failures remain `engineManifest` (1), `g0ToolchainFingerprints` (6),
  `g1GithubActionsQualification` (1), `qualificationPlanArtifact` (1).
  No immutable qualification archive was rewritten. Log:
  `/private/tmp/osv-truss-wrench-full-tests.log`.
- Targeted mechanics Clippy with `--no-deps -D warnings`, wasm32 compilation,
  Vue/MCP and standalone benchmark/harness type checks pass.
- Production Vite build and `verify-dist` pass: 92 artifacts, 5,952,042 asset
  bytes plus 9,740,267 raw WASM bytes, totaling 15,692,309 bytes.

The geometry WASM is 7,712,405 bytes, SHA-256
`eb44e5441614ed25604b0e8cf095cf2df7d75826de9236f7ca2b4444b716b8f8`.
That is 11,195 bytes (about 0.15%) above the preceding truss-enabled artifact.
No new dependency or dependency version was introduced.

## Measurements and Tiering

The native `bench_truss_loads` example measures validation, assembly, rank
admission and resultant checks, not a structural solve. Twenty warmups, 31 samples
of 100 calls each, with construction and independent checks outside timing:

| Selected points | Native p50 A / B, ms |
| --- | ---: |
| 2 | 0.00027 / 0.00033 |
| 4 | 0.00082 / 0.00041 |
| 25 | 0.00277 / 0.00137 |
| 125 | 0.00758 / 0.00507 |

These are initial macOS/aarch64 costs with visible microbenchmark variability,
not a speedup against the incorrect legacy implementation. Reports:
`/private/tmp/osv-truss-loads-native-{a,b}.json`.

`benchmarks/truss-boundary.mts` alternates equivalent direct nodal and single-
wrench models; analytical member forces and global equilibrium are checked
outside timing. Default `TRUSS_WARMUPS` is now 200 per form (configurable 0-1000),
with 31 measured calls per form. It records engine flags and every sample.

With 20 warmups, the 125-node samples switched abruptly from about 12.3 ms to
3.6 ms midway through a single process. The same artifact with 200 warmups gave:

| Nodes | Direct nodal p50 A / B, ms | Wrench p50 A / B, ms |
| --- | ---: | ---: |
| 4 | 0.02179 / 0.02233 | 0.02300 / 0.02413 |
| 43 | 0.72850 / 0.72321 | 0.73179 / 0.72596 |
| 125 | 3.58263 / 3.55204 | 3.58783 / 3.53612 |

The 125-node p95 is 3.75/3.61 ms for nodal input and 3.72/3.63 ms for wrench input.
Its encoded input decreases from 38,749 to 36,008 bytes because one resultant
replaces 122 individual force vectors; the response is unchanged. These fixtures
use one load, not the maximum 32 simultaneous loads.

V8 controls on Node 22.23.2 identify a tiering contribution: `--liftoff-only`
keeps the 125-node p50 near 10.57 ms after 200 warmups; `--no-liftoff` gives about
3.53 ms after 20. `--no-wasm-tier-up` alone did NOT isolate the baseline tier in
this environment (about 3.54 ms), so that flag is not used as contrary evidence.
The smaller 43-node case is also faster with optimizing-only compilation than
with the normal fixed warmup; 200 calls do not promise that every function on
every engine has reached its final tier. This is engine behavior, not a newly
claimed algorithmic speedup. No runtime compiler flags or hidden prewarm solves
were added to the application.

Reports: `/private/tmp/osv-truss-wrench-{boundary-a,boundary-b,steady-a,steady-b,
no-tier-up,liftoff-only,optimizing-only}.json`. Builds/tests did not run
concurrently with any timed process. Reproduce with the benchmark command from
the preceding boundary report, optionally setting `TRUSS_WARMUPS` or Node's
explicit V8 compiler flags.

The production Chrome worker harness, now with 200 warmups, passed force/moment
loading, typed refusal/recovery and cancellation/restart. Maximum-model worker
round-trip p50/p95 was 4.2/4.4 ms; the 4-node median was below the browser timer's
resolution, not literally zero execution time. Startup plus first solve was
163.1 ms. The main-thread animation callback advanced 96 times with a maximum
observed 16.8 ms gap on an otherwise empty page. This is not full-scene FPS.
Report: `/private/tmp/osv-truss-wrench-browser/report.json`.

## Publication Context

The preceding published WASM commit `7ff1ee7c` completed CI run 35485757963:
Rust, official OpenSCAD MCP, macOS and Windows passed. Both Node jobs have only
the known nine qualification-binding failures. This is prior-commit evidence,
not a CI claim for these new load-assembly changes.
