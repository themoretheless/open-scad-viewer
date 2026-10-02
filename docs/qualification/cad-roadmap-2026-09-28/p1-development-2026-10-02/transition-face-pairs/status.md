# Exact partial annular boundary construction

Native and actual WASM checks on 2026-10-02. Mouse and keyboard browser regression checks
for these changes pass. Preview commit remains disabled.

Plane trims retain authored X/Y directly. Their identity chart covers every
outer control, including a binary64 value beyond nominal radius. Transition
plane contact and tangent controls retain exact height. Constant-radius arc
controls use the same multiplication order as the torus surface boundary.
Cylinder walls are ruled between the original bottom and contact rail, so
the contact rail is a natural surface boundary rather than an approximate
polynomial height trim. This preserves the intended retained wall and removes
independently rounded UV height laws.

Exact agreement has direct paths for identity planes and full affine natural
Bezier boundary traversals. Nonuniform pcurve weights do not enter the affine
shortcut. Source perturbations, reversal and work exhaustion remain tested.
The boundary audit runs simpler traversals first while retaining original
output order and the total-work limit; its minimum-sufficient-budget test
passes. Equal per-use budget division was rejected after that test failed.

The regenerated quarter specimen has all 108 uses exactly equal, all joins
exact and all 27 UV regions valid. Work is 387974 units. Within-face absence
is proven on all 27 faces, including the two owned quotient boundaries.
The embedding audit visits all 351 pairs: 234 disjoint, 43 certified shared
contacts and 74 unresolved. Eighteen hull certificates become eligible only
after exact agreement, trim winding and face injectivity succeed.

All 18 regenerated STEP specimens pass independent OCCT solid validity,
full bounds and integrated-volume checks. Maximum volume error is 5.548135959543288e-07 mm³;
maximum bounds error is 1.0000002248489182e-07 mm. The oracle scope is
these enumerated specimens; it does not prove general absence of intersection.

Remaining: distinct-face classification, endpoint G1, wall thickness, complete
volume qualification, arbitrary placements and the full P0-P3 roadmap.

Additional current native regression checks: all 13 curve/surface agreement tests
and all 10 boundary agreement tests passed. A new 18-case exact-boundary matrix
(three scales, three radii, two directions) passes all 108 uses and joins per
model under the same one-million-work cap. This matrix covers axis-aligned
construction; it does not qualify arbitrary rotated placements.

Current WASM packaging completed successfully: SHA256
`ec1628b9a209703be2770dbf8d2cc3fffd38507c858b18d17339778c9b9144f8`.
The regenerated ruled-cylinder model passes six actual-kernel cases (original
and rotated placement, three span budgets) and two worker-handler cases.
All 27 faces are injective at the full budget; whole-model absence remains
unproven. The two frontend self-intersection/boundary suites pass 20 tests.
Type checking passes. Full native B-rep suite passes: 716 passed, zero failed, three ignored.

An experimental one-sided hull support filter passed its focused regressions
but added no certificates on this specimen (234/43/74 unchanged). It was
removed; its separate output is retained as investigation evidence.

Eight actual-WASM partial-preview cases also pass, retaining preview-only
status, source/body identities and finite indexed display meshes.

Production build and artifact verification pass: 140 artifacts, 7,275,381
asset bytes and 11,805,167 raw WASM bytes. Both mouse and keyboard browser
scenarios load the regenerated model, inspect all faces at full budget,
retain the unproven whole-model result, exercise partial budgets, and preserve
the document. The mouse quotient screenshot was visually inspected.

## Expanded pair search

The same source model is rechecked with 1024 geometry cells and 100000 UV
domain cells per pair. All 351 pairs are visited, consuming 77950 geometry
cells and 717156 domain cells: 240 disjoint, 43 shared boundary and 68
unresolved. Whole-model absence remains unproven.

The six pairs without shared topology (6/15, 6/23, 9/18, 9/26, 15/23,
18/26) close after 365–569 geometry cells. Targeted searches retain explicit
unresolved boxes at budget 256 and cover the entire domain at budget 1024.
The remaining 68 consist of 16 shared-edge and 52 shared-vertex pairs;
these require qualified boundary-contact proofs. Increasing a budget is
not accepted as proof if any unresolved box remains.

Reproduce the expanded audit with `transition-boundary-prerequisites`,
`--regenerate --exact-only --embedding --pair-cells=1024
--pair-domain-cells=100000`. See `embedding-budget1024.json` and
`separate-pairs-budget.json` for complete reports.

The new native regression `partial_annular_separate_pairs_require_complete_search_coverage`
passes for all six pairs, checks both insufficient and sufficient budgets,
and verifies source preservation. No production runtime changes in this step.

## Native oblique vertex support

A bounded additional hull proof uses exact orient3d signs on original
binary64 controls. Candidate planes pass through an owned shared vertex and
two control points. Opposite closed half spaces restrict any contact to the
plane; one net must reach it only at that exact vertex. Supporting geometry
without shared ownership, overlapping interiors and shifted vertices refuse.
Candidate search is limited to 16 distinct controls per vertex and nets of
at most 64 controls. Missing certificates retain the unresolved result.

Six hull-contact tests and three boundary-embedding tests pass. A nonsingular
dyadic shear of sphere quadrants demonstrates a vertex certificate where
world-axis hull filtering cannot certify it. The regenerated transition
recheck retains 240 disjoint, 43 shared and 68 unresolved pairs; the new method
adds no certificate for this specimen. WASM packaging of this native change
and its runtime qualification remain pending.
