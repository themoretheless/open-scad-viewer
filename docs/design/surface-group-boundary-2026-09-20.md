# Rust surface grouping boundary

The grouping implementation from `themoretheless-rust-surface-grouping` is
isolated in `geometry-bridge/src/mesh_surface_groups.rs`. It preserves current
host sparse-mesh behavior: validate every position, but only weld referenced
positions when vertices outnumber indices. The visual client now requests it
at worker publication; headless clients retain the default ungrouped path.

## Conformance

- Seven native tests cover welding, winding, degeneracy, sparse inputs,
  nonfinite unused positions, layout, angle and triangle limits.
- `scripts/check-surface-group-parity.mts` compares exact IDs and errors with
  the current host implementation across 838 deterministic cases, including
  angular threshold neighbors. The native example consumes the same JSON
  fixtures; signed zero and nonfinite values are covered by native tests.
- Nineteen tests across `surfaceGroupsKernel`, `solidAnalysis`, and
  `meshSurfaceGroups` pass against the rebuilt WASM. These cover display meshes,
  subarray uploads, empty results, refusal/recovery, memory growth and transfers.
- Application typecheck and standalone strict benchmark typecheck pass.

## Measurements

Run `node --import tsx benchmarks/surface-group-boundary.mts` after building
the geometry kernel. Node 22.23.2, macOS arm64. Five warmups, fifteen alternating
TS/Rust samples per case, exact ID checks outside timing. No simultaneous local
build/test workload. Allocation and GC remain part of the measurement.

The Rust interval includes host uploads, ABI response decoding, result copy and
lease release. Both sides recompute groups; neither uses the content cache.

| Connected planar strip triangles | TS p50 A/B, ms | WASM p50 A/B, ms |
| --- | ---: | ---: |
| 128 | 0.0997 / 0.0907 | 0.0855 / 0.0865 |
| 8192 | 2.1423 / 2.1387 | 1.9515 / 1.9646 |
| 65536 | 27.8971 / 27.6899 | 16.4012 / 16.4249 |

The large strip improves about 41%; the medium strip about 8-9%. The small
case is noisy. These fixtures do not establish performance for arbitrary
topologies, cache hits, cold startup, or complete scene publication.

Reports: `/private/tmp/osv-surface-group-boundary-a.json` and
`/private/tmp/osv-surface-group-boundary-b.json`, including all samples and hashes.
Optimized WASM is 7,680,174 bytes, +10,580 versus the preceding local artifact;
SHA-256 `bb075a40cc24aa3127343c0ab7679fd068cef8d5b6cf638bb2b2f54845a39379`.
Historical qualification artifacts were not updated.

## Packaging Check

Production Vite build and `verify-dist` pass: 91 artifacts, 5,786,696 asset
bytes plus 9,708,036 raw WASM bytes, 15,494,732 bytes total. The tracked
`public/wasm/geometry-kernel.wasm` is included with this integration, not just
the ignored generated embedded payload.

The Chrome production worker packaging check passes cold initialization, warm
operations, refusal and recovery for SVG and G-code workers. Report:
`tmp/performance/surface-groups-worker-packaging/report.json`.
This is packaging compatibility evidence, not a replacement for qualification
or a full application interaction test.

## Browser Publication

`tools/browser-qualification/surface-group-publication.mjs` uses two independent
production workers and the current host selection code. Each fixture has three
warmups and nine measured pairs, alternating order. Source dimensions vary for
cache misses and repeat for hits. Exact IDs are compared outside timing.
The measured interval includes worker build, transfer, packet validation and
host selection, but excludes GPU upload and paint.

| Fixture | Cache | Host TS p50 A/B, ms | Worker Rust p50 A/B, ms |
| --- | --- | ---: | ---: |
| Cube, 12 triangles | miss | 0.3 / 0.4 | 0.3 / 0.4 |
| Cube, 12 triangles | hit | 0.2 / 0.2 | 0.2 / 0.1 |
| Sphere, 16128 triangles | miss | 23.6 / 23.1 | 22.4 / 22.4 |
| Sphere, 16128 triangles | hit | 18.2 / 18.2 | 18.3 / 18.3 |

Sphere main-thread selection drops from 3.8 ms to below the browser timer's
resolution. The complete measured miss path improves only 0.7-1.2 ms, not the
41% of the isolated large-strip benchmark. Hits add about 0.1 ms here; smaller
cases are timer-resolution limited. Reports are in
`tmp/performance/surface-group-publication/report.json` and
`tmp/performance/surface-group-publication-repeat/report.json`.

The worker cache retains its own buffers and sends detached copies; authoritative
CAD faces are untouched. Protocol v7 carries the opt-in request and inferred-ID
metadata. The app requests grouping; other coordinators default to off. The
host bypasses recomputation only for explicitly prepared or authoritative IDs.

The real-app smoke verifies a requested grouped build, successful worker result,
and four objects in the UI. It exposed a pre-existing startup bug: the initial
`doRender` returned before viewport initialization. Removing the renderer guard
allows computation before GPU readiness; viewport initialization already loads
the retained scene. Report: `tmp/performance/surface-group-app-smoke/report.json`.

## Integrated Test Follow-Up

Full Vitest on `58083906` completed in 105.84 s: 3310 passed, 16 failed across
347 files. Nine failures remain in historical qualification/fingerprint checks.
Seven `modelGraphHttp` failures were `listen EPERM` from the sandbox; all seven
pass when rerun with loopback permission. The full-suite log is
`/private/tmp/osv-worker-selection-full-tests.log`. This is not a green full suite.

A new deterministic cancellation test initially received `succeeded` after
cancellation was queued during selection publication. The worker now yields to
the event queue between mesh calls after a 16 ms work slice and rechecks its
terminal state, including after the last mesh. This is not preemption inside
a synchronous Rust call or a strict 16 ms wall-time bound. The regression and
47 worker/coordinator/scene tests pass after the fix; application typecheck
also passes. The full-suite result above predates this narrow lifecycle fix.

Post-fix production browser control (`48a29408`) still passes startup with four
visible UI mesh entries. Sphere miss p50 is 23.4 ms host versus 22.5 ms worker;
hit p50 is 18.2 ms for both. Main-thread grouping is 3.9 ms versus below timer
resolution. Report: `tmp/performance/surface-group-publication-cancellable/report.json`.
`verify-dist` passes with 15,497,094 total bytes.

Remote Node 22 job 105999565937 on `370d3be4` reports 3301 passed, 17 failed,
five skipped: ten qualification checks and seven timeout failures. The separate
STEP interchange V10 workflow passed, but the main CI was still running when
inspected. These are different gates. The CI Vitest command now explicitly
uses `--maxWorkers 2`, matching local control; per-test timeouts and qualification
assertions are unchanged. Whether this removes remote timeout failures remains
unverified until the next CI run. Log: `/private/tmp/osv-ci-35481315494-node22.log`.

### Remote Concurrency Control

Node 22 job 106001779112, run 35482145042 on `bce3b408`, completed with
3312 passed, 10 failed, five skipped in 163.74 s. All seven timeout failures
seen in preceding job 105999565937 are absent, including CLI EOF shutdown,
the fresh-process oracle, analytic B-rep, HTTP MCP, assembly overlap and both
profile cases. The previous run took 245.98 s. This is an observed reduction
of 33.4%, not an isolated same-commit A/B or a guarantee against future flakes.

The remaining failures are qualification/fingerprint checks, including the
ambient `GITHUB_SHA` test fixed locally in `fa775d76` after this CI commit.
The worker cap is retained; assertions and timeouts were not relaxed.
Node 20 and native/Rust jobs were still running at this observation.
Log: `/private/tmp/osv-ci-35482145042-node22.log`.

The completed Node 20.19 job `106001779135` on that same run reports
3311 passed, 11 failed, 5 skipped in 269.63 seconds. Ten failures match the
qualification set above; one `legacyDirectEvaluatorOracle` fresh-process
determinism test still hits its 5000 ms timeout. Thus the zero-timeout observation
is specific to Node 22, not the whole matrix. Rust and official OpenSCAD MCP
jobs have passed; Windows/macOS native jobs remain in progress at this follow-up.
Node 20 log: `/private/tmp/osv-ci-35482145042-node20.log`.

Final outcome for CI run `35482145042` on `bce3b408`: Rust, official OpenSCAD
MCP, macOS native and Windows native jobs all completed successfully. Node
20.19 and 22 retain the failures detailed above. This is evidence for that
commit, not a CI pass for the subsequent centered/isogrid lattice changes.

## Edge Traversal Follow-Up

The Rust implementation no longer retains an insertion-order vector of edge
keys or performs a second hash lookup per edge during union. Every union keeps
the minimum triangle root; connected components and first-triangle compact IDs
are independent of edge traversal order. The 838 shared native/TS fixtures
still agree exactly after this change.

The boundary benchmark now accepts an explicit WASM path, verifies that the
optional compiler actually loaded it, and records its hash separately from
working-tree source hashes. This allows A-B-B-A without changing host code or
rebuilding the historical control. Each series retains five warmups, fifteen
alternating host/Rust samples, and exact-ID checks outside timing.

| Triangles | Old WASM A1/A2 p50, ms | New WASM B1/B2 p50, ms |
| --- | ---: | ---: |
| 128 | 0.06471 / 0.06533 | 0.06504 / 0.06675 |
| 8192 | 1.94746 / 1.95975 | 1.82658 / 1.83971 |
| 65536 | 16.38200 / 16.23463 | 14.71133 / 15.17675 |

The medium strip improves about 6%, the large strip about 7-10%; the small
strip does not improve. Host controls vary between 25.92 and 27.75 ms on the
large case, so do not treat the precise percentage as universal. No complete
scene speedup is inferred from this boundary-only experiment.

Old artifact: `bb075a40cc24aa3127343c0ab7679fd068cef8d5b6cf638bb2b2f54845a39379`.
New artifact: `4965b89d8c271b55cbd522331e17f8e2d79fc66c9c23ad5139ef0c51ed252bfe`,
7,680,136 bytes (38 fewer). Reports:
`/private/tmp/osv-surface-edge-order-{a1,b1,b2,a2}.json`.

The boundary corpus now also includes the final stride-6 display buffers of
a 16128-triangle sphere and a 1020-triangle cylinder, with generated normals
and crease splits. Input SHA-256 values are checked equal across A-B-B-A;
construction remains outside timing and each measured result matches TS IDs.

| Fixture | Old A1/A2 p50, ms | New B1/B2 p50, ms |
| --- | ---: | ---: |
| Sphere, fn=128 | 2.89958 / 2.89242 | 2.69404 / 2.71917 |
| Cylinder, fn=256 | 0.17450 / 0.17717 | 0.17088 / 0.17217 |

The sphere improvement repeats at roughly 6-7%. Cylinder savings are only
microseconds and are not a material end-to-end claim. The large strip in this
same series remains 16.07-16.19 ms old versus 14.76-14.87 ms new. Reports:
`/private/tmp/osv-surface-topology-{a,b,b2,a2}.json`.

## Remaining Optimization

Do not copy the old branch's export-then-render grouping sequence: it computes
groups twice. Group the final display coordinates once in the producer, carry
an explicit distinction between inferred patches and authored CAD faces, and
retain the host fallback/cache for other mesh producers. Compare full
publication before switching that path. The enabled worker implementation groups
once on a cache miss after building display buffers, using the measured upload
boundary. It does not yet eliminate that upload by grouping inside the retained
solid producer. Preserve cache-hit and nonvisual behavior in that next change.
