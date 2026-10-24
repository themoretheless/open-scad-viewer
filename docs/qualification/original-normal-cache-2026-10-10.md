# Original endpoint normal reuse

`OriginalChordWorkspace` now retains complete interval normal hulls keyed by
exact original UV rectangle bits. The workspace owns an immutable surface
reference and angular policy. Partial hulls are not cached, cache size is
bounded, rectangles are validated before reuse, and zero-budget pair requests
still return unproven without geometry admission.

A real rational normal-diameter test isolates work savings: cold query 7 spans;
rectangle certificates warm but normals absent 4 spans; fully warm 2 spans.
Removing only the normal cache restores the two endpoint enclosure queries.
All paths correctly retain the actual normal diameter as unexcluded. Reversed
endpoint order, varied budgets, foreign surfaces, degenerate boundaries and
invalid domains retain their existing controls.

19 self-chord tests pass. Log:
`/tmp/original-normal-cache-isolated-20261010.log`.
Full NURBS suite is recorded separately when terminal.

This change follows WASM publication
`7525335b37e56aa8548e38b12206c175ac2e0a74a441fb6d5b478269498090b9`.
It is native source work only until rebuilt and requalified. The already-running
partial rational maximum-work audit is the pre-cache baseline and is preserved.
No whole-wall completeness or end-to-end performance improvement is claimed.

Full native NURBS library completed: 845 passed, zero failed, two ignored,
37.88 seconds. Log: `/tmp/original-normal-cache-nurbs-all-20261010.log`.
This validates the updated source; the published WASM still predates the cache.

Three BRep wall product tests also pass with the updated native dependency:
exact containment, same-face symmetry reuse, diagnostic scope/reason/work
serialization. Log: `/tmp/normal-cache-brep-product-tests-20261010.log`.
This is not the complete shell wall calculation.

The assembled complete-carrier comparison was started with the saved baseline's
explicit limits: 10,000 wall cells, 1,000,000 spans, 100,000 trim-domain cells.
It writes `mapped-retained-carrier-2026-10-08/original-normal-cache-trimmed-wall-2026-10-10.json`
and its request sidecar. Log: `/tmp/original-normal-cache-trimmed-wall-20261010.log`.
Compare only after terminal output. The existing `expression wall pending
reasons budget=435` log is the intentional zero-budget refusal control, not the
final result of either positive-budget audit.

## Matched assembled-shell result

The complete-carrier 10,000-cell run passed in 308.91 seconds under concurrent
work. Span work falls from 38,693 to 35,238 (3,455 saved, 8.929%). Only the
original-pair stage changes: 8,870 to 5,415 spans. The pending UV products and
reasons match the baseline exactly, as do coverage, exclusion and symmetry
counters. Both runs remain incomplete with 4,081 pending regions. This is a
measured work reduction, not full wall qualification or a timing benchmark.

Evidence: `mapped-retained-carrier-2026-10-08/original-normal-cache-comparison-2026-10-10.json`
and the native wall report plus request sidecar. WASM build and full BRep suite
were launched for this updated source; neither result is credited before
terminal verification. Rust input manifest:
`mapped-retained-carrier-2026-10-08/original-normal-cache-wasm-inputs-2026-10-10.json`.

The full BRep library run for this source generation completed: 1,035 passed,
zero failed, three ignored, 1,220.76 seconds. Log:
`/tmp/original-normal-cache-brep-all-20261010.log`. It predates the later
side-remainder cache and fixed parameter-shear bound; it does not qualify those
new source changes. The WASM packaging process remains separately tracked.

WASM publication completed successfully. Public and generated worker checks
use SHA `41a8746ec39e719a4bf9866771fe65c4e3dd6f7e3162b531b9bea6431903884a`,
13,084,015 bytes. The build compiled this generation before the subsequent
source edits; it includes original-normal caching, not side-remainder caching
or fixed parameter shears. SourceBody root-ended mapped-edge worker checks
pass (two selected tests, 13.77 seconds). The profile-chord worker gate passes
native, whole-coverage and rational-profile parity, actual-normal-diameter
preservation, error recovery, zero-budget refusal and cancellation/Retry.

Reports: `mapped-retained-carrier-2026-10-08/original-normal-cache-root-worker-2026-10-10.json`
and `original-normal-cache-publication-worker-2026-10-10.json`.
Logs: `/tmp/original-normal-cache-wasm-build-20261010.log`,
`/tmp/original-normal-cache-root-worker-20261010.log`,
`/tmp/original-normal-cache-publication-worker-20261010.log`.

## Partial rational maximum-work baseline

The preceding pre-cache partial-carrier audit completed successfully in 1,577.80
seconds, but coverage is incomplete: 94,606 cells, 1,000,000 spans, 1,000,000
trim-domain cells; all 435 initial face pairs visited; 42,787 pending regions
in 45 face pairs, all budget-limited. Dominant stage costs are intrinsic
370,121 spans, corrected pairs 267,790, retained pairs 218,236. These remain
larger optimization targets than original pairs (52,734). The 17,057 pending
self-products on face 28 dominate the pending set.

Evidence: `mapped-retained-carrier-2026-10-08/partial-rational-wall-max-work-summary-2026-10-10.json`
and its report/request. This tests endpoint-normal-aligned chords, not an
unconstrained physical minimum wall thickness. It is not directly comparable
to the complete-carrier 10,000-cell cache run.

## Actual browser worker check

On 2026-10-10 the isolated `cad-draft-tabs.html` fixture executed its
production worker duplicate action against public WASM SHA
`41a8746ec39e719a4bf9866771fe65c4e3dd6f7e3162b531b9bea6431903884a`.
The visible result was “Worker duplicate verified; saved draft unchanged”,
exact document roundtrip true, local edits zero, and no captured console
errors. Worker action took 541.1 ms in this single cold observation; this is
not a benchmark. The existing saved draft revision remained
`5e54c627-6499-4be4-b853-aaf0c4ca08e1`.
Screenshot: `/tmp/original-normal-cache-browser-worker-20261010.png`.
The temporary fixture tab was closed; the user's model tab was untouched.
This checks transport and document retention for duplication, not the full
command matrix, crash recovery, or general fillet admission.
