# Fixed parameter shear for intrinsic chord bounds

The original-chart and corrected-expression intrinsic bound now also tries
two fixed triangular parameter shears after diagonal column normalization.
For Q with determinant one, JQ encloses the same continuous-path derivatives
in coordinates delta=Qz. Endpoint normals still satisfy n·JQ=0. Midpoints
choose a finite shear coefficient only; rank, variation and angle estimates
use outward interval arithmetic over the entire original derivative hull.
Each sheared enclosure is independently column-normalized. No geometry
samples or caller-created certificates supply admission.

A skewed interval matrix with columns (1,0,0) and
([19.62,20.38],1,0) remains unresolved under the raw and diagonal-only bounds;
the fixed shear proves intrinsic normal-chord exclusion at squared sine 1e-4.
The test repeats at scales 1e-90, 1 and 1e90 and retains refusal when the
enclosure contains a singular matrix.

The first focused run found an existing raw Gram overflow on the large-scale
control (42 passed, one failed). Raw arithmetic is now an optional bound:
overflow leaves it unresolved and the normalized enclosure is evaluated.
The repaired focused run passes 43 tests, including rational actual-normal-chord
refusals and budget/coverage controls, in 0.93 seconds. Logs:
`/tmp/fixed-shear-self-chord-20261010.log` and
`/tmp/fixed-shear-self-chord-fixed-20261010.log`.

Full NURBS and the matched 10,000-cell assembled-wall calculation are running
separately. This source change is not present in the still-running earlier
original-normal-cache WASM build. No full-wall completeness, universal fillet
support or runtime improvement is claimed by the matrix fixture.

The full current NURBS library completed: 847 passed, zero failed, two ignored,
49.74 seconds. Log: `/tmp/fixed-shear-nurbs-all-20261010.log`.
The complete BRep library and matched assembled-wall test remain separate
pending checks. Source inputs are recorded in
`mapped-retained-carrier-2026-10-08/fixed-shear-native-inputs-2026-10-10.json`.

## Matched wall report observed before terminal test completion

The positive-budget report has been written: all 435 initial pairs visited,
10,000 cells, 34,841 spans, 4,081 budget-pending regions, complete=false.
The prior cache-only run used 34,842 spans with the same region count.
Intrinsic exclusions increase from 78 to 80. Pending UV products are not
byte-identical, so exact-report regression is not claimed.
`scripts/compare-expression-wall-pending.py` passes exact endpoint containment
of every new unresolved product in the preceding pending set (including
same-face endpoint symmetry). Evidence:
`mapped-retained-carrier-2026-10-08/fixed-shear-pending-containment-2026-10-10.json`.
This establishes no new unresolved parameter region, not full physical wall
qualification or a performance gain. All 1,811 Rust/Cargo inputs still match
the frozen WASM source manifest at this observation. Full BRep test and WASM
publication have not yet reached terminal results.

The matched assembled-wall test has now terminated successfully: one test
passed, zero failures, 690.76 seconds under concurrent load. Log:
`/tmp/fixed-shear-trimmed-wall-20261010.log`. The positive-budget report remains
incomplete as described above; test success is not a whole-wall certificate.

The newly compiled, unpackaged WASM binary also passes the direct binary ABI
gate for plane, rational proposed expression, actual normal diameter, whole
coverage, stopped coverage and curved whole coverage. Zero-budget refusal,
invalid-domain refusal and recovery pass. SHA:
`b0324cc708243f348a1f06a7d656133a14810e93c2cc379572f05cf875b8f82e`,
14,765,184 bytes. Evidence: `mapped-retained-carrier-2026-10-08/fixed-shear-unpacked-abi-2026-10-10.json`.
This is not the optimized public artifact or packaged-worker acceptance.

The complete fixed-shear native BRep library run has now terminated with exit
zero: 1,035 passed, zero failed, three ignored, 1,204.50 seconds. Log:
`/tmp/fixed-shear-brep-all-20261010.log`. This source generation includes the
side-remainder cache and fixed shear; it predates the subsequent retained
profile workspace. The full suite therefore does not qualify that newer
workspace integration. The new browser qualification fixture passes
`vue-tsc --noEmit` separately; log `/tmp/profile-coverage-browser-typecheck-20261010.log`.

Binary optimization has completed (14,765,184 -> 13,089,068 bytes). The build
reported SHA `ffb7c3ba14d85c5e5872f2b5bebe17dba49da81bef48c27b26ec27ab69983eaf`.
At this observation packing remains live and the public file still has the
preceding SHA, so no new-publication or packaged-worker success is claimed.

Publication has now terminated with exit zero. Public WASM SHA matches
`ffb7c3ba14d85c5e5872f2b5bebe17dba49da81bef48c27b26ec27ab69983eaf`,
13,089,068 bytes. Source generation is fixed-shear plus side-remainder cache,
not the subsequently compiled retained-profile workspace. Pinned public-worker
root-ended and profile-chord gates have been launched for this exact SHA.

Both exact-SHA packaged-worker gates have terminated successfully. Root-ended
mapped-edge replay/refusal: two selected tests passed (11.01 seconds); admitted
control has all six embedding pairs qualified and four root checks, with
volume/orientation qualification. Profile chord gate checks two actual worker
artifacts (initial and after cancellation), exact native/whole-coverage/rational
parity, preservation of a genuine normal diameter, error recovery, zero-budget
refusal and cancel/Retry. Reports:
`mapped-retained-carrier-2026-10-08/fixed-shear-root-worker-2026-10-10.json` and
`fixed-shear-publication-worker-2026-10-10.json`. Logs:
`/tmp/fixed-shear-root-worker-20261010.log`,
`/tmp/fixed-shear-publication-worker-20261010.log`.
These are bounded fixtures, not general fillet or whole-wall qualification.

An actual browser production-panel run on the published fixed-shear SHA
also passes: keyboard Enter dispatch, pending/cancel controls, then complete
normal-chord exclusion across the planar proposed transition (22 cells).
Captured console errors are empty. The UI explicitly states that thickness,
self-intersections and closed-solid qualification require separate checks.
Screenshot: `/tmp/fixed-shear-browser-panel-20261010.png`. Temporary fixture
tab closed, user model untouched. No general geometry or full matrix claim.
