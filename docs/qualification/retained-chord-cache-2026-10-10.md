# Retained profile chord hull reuse

The weight-profile endpoint-pair implementation now exposes an invocation-local
`WeightChordWorkspace` that holds an immutable profile reference. It reuses
only complete derivative, position and nonzero direction enclosures keyed by
exact UV bits and profile identity. Existing derivative caches cap each map
at 100,000 entries. The BRep wall calculation uses one workspace for the
selected profile shared by its retained restrictions. Geometry, UV validation
and zero-budget refusal still precede cache use. No caller-supplied hull or
certificate grants admission.

Native owner/refusal controls pass: 25 retained-related tests, including the
new warm-cache test and actual rational half-cylinder normal-diameter tests.
Warm repeated planar requests use fewer spans; a one-span incomplete request
can be retried, invalid domains/angles reject, zero budget stays unresolved,
and another owner starts cold. Transition-expression tests also pass (10).
Logs: `/tmp/retained-chord-cache-owner-controls-20261010.log`,
`/tmp/retained-chord-cache-controls-20261010.log`.

The first BRep filter selected zero tests; that result is compilation evidence
only. A corrected named product test and full native NURBS suite run separately.
A matched assembled-wall run uses the same 10,000 cells / 1,000,000 spans /
100,000 trim cells as the fixed-shear baseline. No work or timing gain is
claimed until its terminal report is inspected. This new source generation
postdates the compiled fixed-shear WASM and is not in that publication.

The complete native NURBS library has terminated successfully: 848 passed,
zero failed, two ignored, 42.59 seconds. Log:
`/tmp/retained-chord-cache-nurbs-all-20261010.log`. The corrected BRep product
test `intrinsic_exclusion_reuse_requires_exact_containment` also passes.
Log: `/tmp/retained-chord-cache-brep-product-fixed-20261010.log`.
The matched assembled-body wall run remains live.
Source inputs: `mapped-retained-carrier-2026-10-08/retained-chord-cache-native-inputs-2026-10-10.json`
(1,811 Rust/Cargo files).

A fresh native transport report for this source generation matches the
existing six-case profile bridge fixture exactly, including work and unresolved
UV products. Test `replays_independent_owners_and_preserves_refusal` passes
in 0.44 seconds. Saved report:
`mapped-retained-carrier-2026-10-08/retained-cache-profile-native-transport-2026-10-10.json`;
log `/tmp/retained-cache-profile-native-transport-20261010.log`.
The complete current BRep library has been launched separately; log
`/tmp/retained-chord-cache-brep-all-20261010.log`.

## Matched report written; terminal test result still separate

At the same 10,000 wall cells, 1,000,000 span limit and 100,000 trim cells,
span work drops from 34,841 to 23,890: 10,951 saved (31.4314%). Retained-pair
work alone drops from 12,341 to 1,390. Every pending UV product, reason,
coverage and exclusion counter remains exactly equal to the fixed-shear
baseline. Only cache hits and retained-pair/total work change. The strict
`compare-expression-wall-reports.mjs --work-reduction` gate passes.
Report: `mapped-retained-carrier-2026-10-08/retained-chord-cache-regression-2026-10-10.json`.
Both reports remain incomplete with 4,081 pending regions; this is measured
work reuse, not a runtime benchmark or physical wall-thickness qualification.
All 1,811 source inputs still match the saved retained-cache manifest.

A follow-up partial rational carrier run is now launched with exactly the
pre-cache maximum-work baseline request: 100,000 cells, 1,000,000 spans,
1,000,000 trim-domain cells. This is justified by the new complete retained
hull reuse and stronger intrinsic bound, not a budget-only retry. New output:
`mapped-retained-carrier-2026-10-08/retained-cache-partial-rational-wall-max-2026-10-10.json`.
Log `/tmp/retained-cache-partial-rational-wall-max-20261010.log`.
It must be compared with the saved partial carrier baseline after completion;
its coverage must not be conflated with the complete carrier 10,000-cell run.

The matched assembled-body test is now terminal with exit zero: one passed,
zero failures, 573.28 seconds under concurrent load. Log:
`/tmp/retained-chord-cache-trimmed-wall-20261010.log`. The exact-coverage
work-reduction comparison is therefore backed by a successfully completed
native test. This timing is not a controlled benchmark. Whole-wall coverage
remains incomplete.

Current-source WASM compilation only is launched using the production
reproducible Cargo flags and target directory; log
`/tmp/retained-chord-cache-wasm-compile-20261010.log`. All 1,811 source inputs
match `mapped-retained-carrier-2026-10-08/retained-chord-cache-wasm-inputs-2026-10-10.json`
at launch. It does not optimize or publish. The prior fixed-shear optimizer
uses its separate immutable input snapshot (see `wasm-optimize.mjs`), so that
publication remains the only writer of generated/public output. A later
packaging run must wait for prior publication and must requalify its own SHA.

## Compiled ABI and packaging follow-up

The reproducible Cargo compilation completed successfully in 4m43s. The raw
WASM SHA256 is `c0eb987de43c2cc7ad3b419c26e743854d814d049c434d06d54383b91aed6f95`
(14,765,238 bytes). Six direct ABI parity cases passed, including whole
coverage, stopped coverage and curved coverage; zero-budget refusal, invalid
domain refusal and recovery also passed. Evidence:
`mapped-retained-carrier-2026-10-08/retained-cache-unpacked-abi-2026-10-10.json`.
This does not qualify optimized WASM, the worker or the UI for this generation.

After the prior fixed-shear publisher completed, the full production packaging
was launched separately. Log: `/tmp/retained-chord-cache-wasm-build-20261010.log`.
Its optimized publication and exact-SHA worker gates remain pending.

## Full B-rep regression

The retained-cache native library suite completed with exit zero: 1,035 passed,
zero failures, three ignored, 916.72 seconds under concurrent load. Log:
`/tmp/retained-chord-cache-brep-all-20261010.log`. This confirms regression
coverage for this source generation; it does not turn unresolved wall regions
into qualified geometry. The matched partial-rational maximum-work run is
still separate.

## Matched partial rational carrier completed

The endpoint-cache partial carrier test completed with exit zero: one passed,
1,217.76 seconds under concurrent load. It reaches the unchanged 100,000-cell
limit with 795,987 spans, versus 94,606 cells and the 1,000,000-span limit in
the pre-cache baseline. All 435 initial pairs were inspected. The result
remains incomplete: 46,287 pending regions across 45 face pairs, versus 42,787
previously. Pending products are not byte-identical because additional cells
are processed; counts alone must not be interpreted as worse geometry or
stronger qualification. Exact endpoint containment is checked separately.

Stage work: retained pairs 218,236 -> 15,298; corrected pairs 267,790 -> 264,936;
intrinsic 370,121 -> 395,624. These runs process different numbers of cells,
so total span reduction is not an equal-coverage work-reduction certificate.
The next intrinsic-cache source generation is distinct from this binary.
Comparison: `mapped-retained-carrier-2026-10-08/retained-cache-partial-rational-comparison-2026-10-10.json`.

Exact endpoint containment passed for every new pending product, allowing
exchange of endpoints only on the same immutable face. Thus additional
subdivision introduces no unresolved region outside the previous pending
products. The report still contains unresolved regions and admits no wall
certificate. Evidence:
`mapped-retained-carrier-2026-10-08/retained-cache-partial-rational-containment-2026-10-10.json`.

## Endpoint-cache WASM publication completed

The full build completed with exit zero. Optimized public SHA256:
`c5d0f3bb20987d7be380374a82d41cfc959fdef632afb7d4967fe3443309fbbe`,
13,089,099 bytes (raw 14,765,238 bytes). Build log:
`/tmp/retained-chord-cache-wasm-build-20261010.log`.
Exact-SHA root worker and profile-coverage worker qualification have been
launched separately. This publication predates the retained intrinsic cache.

Both exact-SHA packaged worker gates completed with exit zero. Root-ended
mapped-edge checks: two passed, 28 unrelated tests skipped (3.86 seconds).
Profile worker: native parity, whole-coverage parity, rational-profile parity,
normal-diameter preservation, native-error recovery, zero-budget refusal and
cancel/Retry all passed; both worker instances checked the requested artifact.
Reports:
`mapped-retained-carrier-2026-10-08/retained-cache-root-worker-2026-10-10.json` and
`mapped-retained-carrier-2026-10-08/retained-cache-publication-worker-2026-10-10.json`.
These are bounded packaged-worker checks, not full browser/geometry acceptance.

The complete real worker-boundary suite passed against this publication:
34 tests, zero failures, 15.73 seconds test time (17.26 total). Log:
`/tmp/retained-cache-real-worker-boundary-all-20261010.log`.

An actual browser fixture mounted the production ProfileBridgeCheck component
and production worker. Keyboard Enter started whole coverage, disabled inputs
and focused cancellation while pending. The planar transition completed with
22 cells; normal-chord exclusion qualified. The UI explicitly keeps thickness,
self-intersections and closed-body qualification separate. Console errors: none.
Screenshot `/tmp/retained-cache-browser-panel-20261010.png`. The temporary tab
was closed; the user's model and draft were untouched. This is isolated-panel
acceptance, not modeler selection or general geometry acceptance.
