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
