# Complete interval width operator bound

`surface_self_chord::cosine_upper` retains the original Frobenius variation
bound and takes the minimum with an outward width-Gram maximum-row-sum bound.
Let W contain complete derivative interval widths. Every difference E between
enclosed differentials satisfies |E| <= W componentwise. Therefore
||E||_2^2 <= rho(W^T W) <= max row sum(W^T W). This supplies a conservative
upper numerator for the existing least-singular-value chord inequality.
The existing original differential/normal/continuity obligations remain.
No samples or midpoint rank decide admission.

A complete independent diagonal enclosure [.7,1.3] now qualifies: the old
Frobenius numerator .72 exceeds the lower squared singular value .49, whereas
the new numerator .36 proves the bound. Tests cover scales 1e-90, 1, 1e90 and
refusal of enclosed singular or correlated uncertain columns.

Focused surface self-chord tests completed with exit zero: 44 passed, zero
failures, including real rational-cylinder diameter preservation and budget
refusals. Log `/tmp/width-gram-self-chords-focused-20261010.log`. Full NURBS
regression and fresh native transport report are launched separately.

Source generation:
`mapped-retained-carrier-2026-10-08/width-gram-native-inputs-2026-10-10.json`.
This postdates the intrinsic-cache binaries currently being optimized/tested.
The old running jobs cannot qualify this new width bound. Full BRep coverage,
WASM publication and browser qualification for this generation remain pending.

Full NURBS regression completed with exit zero: 849 passed, zero failures,
two ignored, 60.54 seconds. Log `/tmp/width-gram-nurbs-all-20261010.log`.
Fresh native six-case profile transport equals the previous fixture exactly,
including span work and unresolved UV products. Parity evidence:
`mapped-retained-carrier-2026-10-08/width-gram-transport-parity-2026-10-10.json`.

Compile-only reproducible WASM build is launched for this generation:
`/tmp/width-gram-wasm-compile-20261010.log`. All 1,811 manifest inputs match.
The prior intrinsic-cache optimizer uses its own immutable compiled input;
no second public artifact writer is launched. Complete-carrier matched wall
coverage remains live in `/tmp/width-gram-trimmed-wall-20261010.log`.

## Original NURBS chart acceptance

Added an actual quadratic tensor-product surface with x(u)=.7u+.3u^2,
y(v)=.7v+.3v^2, z=0 and positive unit weights. Its independently varying
complete derivatives lie in [.7,1.3]. `qualify_rectangle` issues the original
whole-chart certificate; every actual nonzero chord lies in XY and every
regular normal is parallel to Z. The focused test passes. Log:
`/tmp/width-gram-original-chart-final-20261010.log`.

The initial test used the wrong qualification function signature, then
incorrectly expected a zero-budget report. Both test mistakes were corrected:
original-surface qualification explicitly rejects zero work as invalid input;
the diagnostic profile API separately returns unresolved. Production behavior
was not changed to accommodate the test. Failed logs are preserved:
`/tmp/width-gram-original-chart-20261010.log` and
`/tmp/width-gram-original-chart-fixed-20261010.log`.

The additional test postdates the full 849-test run and compiled/running
width-Gram binaries. Its source manifest is
`mapped-retained-carrier-2026-10-08/width-gram-chart-test-inputs-2026-10-10.json`.
Only cfg(test) code changed; no broader completion or whole-wall claim follows.

Compile-only WASM completed with exit zero in 3m06s. Raw SHA256:
`2b283e91cd891f49ee8f6d202c0a2762a42cc386fb740952f7362e5af5d5ba00`,
14,766,205 bytes. Six direct ABI cases plus zero-budget/invalid-domain refusal
and recovery passed. Evidence:
`mapped-retained-carrier-2026-10-08/width-gram-unpacked-abi-2026-10-10.json`.
This is a raw compiled artifact; the public artifact remains the endpoint-cache
publication until the separately running intrinsic-cache publisher completes.
The additional cfg(test) chart test does not change the production algorithm.

## Matched assembled-body coverage completed

The matched complete-carrier wall test completed with exit zero: one passed,
376.27 seconds under concurrent load. At 10,000 cells it exactly reproduces
the intrinsic-cache report: 23,653 spans, 4,081 pending regions, 80 intrinsic
exclusions, 881 angle exclusions and 1,745 distance exclusions. The complete
report comparison passes, including work and every pending product. The new
width bound therefore has no observed coverage or work advantage on this
assembled-body request; its demonstrated improvement is the separate original
nonlinear planar NURBS chart. Whole-wall qualification remains incomplete.
Evidence: `mapped-retained-carrier-2026-10-08/width-gram-exact-regression-2026-10-10.json`.

Matched partial-rational-carrier coverage is launched after the width bound
change, with the same 100,000 cells, 1,000,000 spans and 1,000,000 trim cells
as the earlier partial-carrier runs. This is a separate specimen from the
complete-carrier 10,000-cell regression. Log:
`/tmp/width-gram-partial-wall-max-20261010.log`. Its comparison must wait for
terminal results from both generations; it is not a budget-only retry.

## Matched partial-wall comparison guard

`compare-expression-wall-pending.py` now accepts paired `--before-request` and `--after-request` sidecars. It refuses different specimen test names, cell/span/trim budgets, missing fields, and sidecars naming another report. It records SHA-256 hashes of both sidecars and their source manifests. This pins the declared workload and source inventories; it does not independently prove that a test name denotes identical geometry.

Control execution passed using an existing native wall report: identical requests accepted; individually changed specimen, cell budget, span budget, trim budget and report identity each refused. The two maximum-work partial-rational calculations remain live and have not emitted their positive-budget reports yet. Do not run a comparison against request files alone.

## Full B-rep regression

The width-Gram generation completed the full B-rep library suite: 1,035 passed, zero failures, three ignored, 928.99 seconds (`/tmp/width-gram-brep-all-20261010.log`). This predates the later section-data-cache edit. The ignored curved embedding roadmap gates remain unqualified. The matched maximum-work partial-carrier calculation is still separate and has not completed at this observation.

## Maximum-work partial rational carrier: terminal

One test passed, zero failures, 1543.50 seconds under concurrent workload (`/tmp/width-gram-partial-wall-max-20261010.log`). The same 100,000-cell, 1,000,000-span, 1,000,000-trim request produced all 46,287 previous unresolved products exactly unchanged.

The new bound proves 24 products at the intrinsic stage that previously required the later angle stage: intrinsic exclusions 1,070 → 1,094; angle exclusions 21,247 → 21,223. Original-pair span work decreases by 24; total 788,883 → 788,859. There is no observed coverage gain for this specimen.

Strict report comparison and the existing work-reduction mode correctly refused the changed exclusion-stage counters. A separate explicit `--intrinsic-reclassification` mode now requires equal total intrinsic+angle exclusions, nondecreasing intrinsic proofs, nonincreasing later angle proofs, nonincreasing work, and exact equality of all remaining coverage fields and pending products. Existing strict modes remain unchanged. Four negative controls refuse unbalanced proof counts, changed coverage, increased work and a changed pending reason.

Evidence: `mapped-retained-carrier-2026-10-08/width-gram-partial-wall-reclassification-2026-10-10.json`; exact pending containment with matched request/source-manifest hashes is recorded in `width-gram-partial-wall-pending-2026-10-10.json`. No whole-wall certificate follows from this comparison.
