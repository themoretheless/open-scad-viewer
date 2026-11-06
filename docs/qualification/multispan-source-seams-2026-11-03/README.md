# Multispan original-source seams — 2026-11-03

Status: partial native implementation. No new closed-body admission.

## Change

The original-support extrusion comparator now covers every intersected original U/V knot cell of an affine seam. Outward inverse-affine bounds retain source/candidate intersections and knot endpoint overlaps; rounded roots or fitted surfaces never replace the original source. Every branch borrows that original surface and evaluates its actual coefficients and one-sided limiting jets. Reversed and non-dyadic clocks are supported by the same enclosure path.

Each branch is paired with the appropriate candidate span. Subdivision retains source ownership; a branch is vacuous only after a certified interval separation from its original chart. The full original seam is covered. Exhausted shared query work cannot grant acceptance; preparation has explicit bounds of 256 original source cells and 4096 pairs.

For literal axial product cells, exact coefficient structure proves equality with the cell's own trace cylinder. This shortcut is conditional on projected-trace regularity already proved by the pair comparator, station-independent rational profile weights, literal transverse constants, common axial station coordinates and strictly monotone station controls. Other sources retain the actual differential comparison. Per-cell arithmetic is centered on original cell coefficients without modifying either saved surface.

The regularity flag concerns the covered one-sided fields. Internal G2 across knots of either surface remains a separate obligation; corresponding piecewise fields alone do not qualify a globally smooth body. Topology, embedding, curvature cap, thickness and volume are not inherited from a seam report.

## Validation

32 surface-G2 tests and 36 region-replacement tests passed (68 total). New regressions cover different original/candidate knot layouts, a hidden second-jet defect in a later source cell, non-dyadic source knots, reversed clocks, zero work and one less cell than a completed proof needs. Existing pole ownership, subnormal coefficient drift and rational gauge regressions remain green. Source hashes and exact test names are in `review.json` and `native-results.txt`.

## Current end-to-end result

The origin-zero 10-mm cube with radii 0.1 / 0.2 / 0.35 mm reaches the exterior G2 inspections. All six positions are verified. Plane seams 1,3,5 are WithinTolerance. Curved seams 0,2,4 remain Unresolved: their regularity and tensor upper bounds are not established. The former single-source-cell refusal is gone. Sewing still refuses with `BREP_CENTRAL_LOFT_UNPROVEN: Original central G2 remains unproven`.

This is insufficient evidence, not a certified curvature or G2 counterexample. No new closed body is published and no whole-body <=10/mm, WASM, worker, UI or STEP qualification is claimed. The complete thread goal remains open.
