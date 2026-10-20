# Automatic contact curves with original-support qualification

## Implemented

`fillet_contact::build` calls the native variable-radius surface-pair builder,
extracts both actual skin contact boundaries and proposes original-support UV
curves using the same natural station interpolation. The source surfaces are
immutable. Each common 3D curve must agree continuously with both the original
support lift and the actual authored skin lift. Sampled stations, sampled skin
deviation and claimed radius ranges never grant contact admission.

Only after both complete lift checks succeed does the result expose contacts
with `VerifiedContacts`. Shared contact work remains capped at 100,000 cells and
1,000,000 exact work. Numerical construction failure, closed spine ownership and
unresolved contact proofs remain distinct. This status is explicitly about two
contact curves, not a fillet body or missed-branch coverage.

`build_g2` also prepares the actual skin's V knot multiplicity for structural C2
continuity. Every knot removal requires a continuous positional error bound;
all Bernstein coefficient work, including a final comparison to the ORIGINAL
skin, shares a 2,000,000-work ceiling. The final skin change must stay within one
quarter of the supplied position tolerance. Failed preparation retains the raw
surface, exposes no admitted contacts and reports attempted preparation work.
After an accepted edit the sampled deviation estimate is expanded by the edit
bound and the spine samples are explicitly retained as proposals.

## Current evidence

Four tests pass in `brep-core/tests/fillet_contact.rs`:

- Plane/plane contacts with constant radius and a linear radius law pass both
  full-interval original-support/skin lift checks.
- Plane/rational-cylinder contacts pass for rolling-ball and G2-section builders.
- The prepared G2 skin passes whole-interval normal and world curvature tensor
  comparison at both plane and cylinder contacts (position 1e-6, normal 1e-5, curvature 1e-4).
- Zero work, construction failure and unsuccessful C2 preparation expose no
  admitted contacts; the latter preserves raw geometry and reports spent work.

## Cylinder-side G2 follow-up: gap closed for this control

The earlier 100,000-cell Unresolved result is superseded for this exact
plane/rational-cylinder control. The interval Bernstein evaluator now clips
every de Casteljau step to the convex hull of its two interval coefficients.
This preserves the correlation between t and 1-t before evaluating the second
tensor axis. It is valid only for a unit fraction, now explicitly checked.
Constant coefficient rows no longer acquire artificial variation; no source
control point, normal tolerance or curvature tolerance changes.

Both complete original-pcurve seams now pass WithinTolerance. Cylinder-side
normal upper discrepancy is 4.2779817842491084e-11 and world curvature tensor
upper discrepancy is 1.777366677336551e-10, against unchanged thresholds 1e-5
and 1e-4. The coupled contact and two-seam run uses 76 of 100,000 shared cells
and zero exact-identity work. Optional C2 storage preparation uses 16,128
Bernstein coefficient work of 2,000,000 and its whole-skin error upper bound is
3.5360211181147064e-14.

Reproduce with example `fillet_contact_g2_acceptance` and output directory
`output/qualification/fillet-contact-g2-2026-10-15`. Its receipt stores actual
supports, skin, both common curves and both pcurves, all tolerances, shared work
and bounds. It explicitly denies global-solid, whole-radius and branch-coverage
claims. The test now requires positive whole-interval G2 on both sides instead
of a passing Unresolved expectation.

Regression evidence: 39 B-rep integration tests, 9 original-jet unit tests,
4 shape-operator tests, 2 shape-region tests, 9 geometric-G2 tests and the
composed-degree test passed (64 total). This includes distorted seam normal and
curvature mismatches, zero work, singular queries, non-C2 knots, rational third
jets and original-span/cache identity checks.

## Remaining objective

The marching extent has no complete branch/end ownership proof. Root-bound
restriction between actual end faces is now implemented in `contact_trim`
(see `../contact-root-trim-2026-10-15/README.md`), including refusal of roots in
holes. General endpoint ownership and full-skin end boundaries remain open. Material-side selection, removal, end transition authoring, unequal-radius
complex corners and closed-body assembly remain required. General whole-radius,
thickness and self-intersection qualification, WASM/worker/UI, the three named
part/history acceptance, full P0 and P2/P3 are not completed by this milestone.
