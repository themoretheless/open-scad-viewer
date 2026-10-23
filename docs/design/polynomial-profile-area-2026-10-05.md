# Polynomial profile area and analytic display fixes

Final WASM SHA-256:
`ed2e5a614790b691c5e41ae04409a49a4c6f805c301c5c3ca3267dc4fd91be84`.
The final source passed 918 CAD roadmap tests, 55 profile/worker/manufacturing
tests, type checking and dist budgets. Both mouse and keyboard profile scenarios
passed on this artifact, including independent OpenCascade STEP measurements.
G-code/Laser browser export and reimport also passed with both interactions.
See `docs/qualification/polynomial-profile-2026-10-05/final/manifest.json`.
The full command matrix and the broader P1–P3 roadmap remain incomplete.

## Changes

Equal-weight NURBS knot spans of degree at most two now use Simpson integration
of the Green area integrand. The integrand has degree at most three, so this
quadrature is exact in real arithmetic. All evaluations and accumulation use
outward intervals. Half-cell derivative jets are multiplied by two to restore
the full-cell parameter. Division by 12 uses interval division. Cells without
a representable interior midpoint retain the existing general integration path.

This changes area integration only. Profile admission still requires the
existing closure, simplicity, separation, nesting and orientation proofs.
Rational profiles with unequal weights retain adaptive interval integration.

Sphere UV quarter circles with weights `[1, 1, 2]` now use the quarter-disk
display tessellation alongside the symmetric `[1, sqrt(1/2), 1]` traversal.
The mesh evaluates the authored arc and retains its edge sample registry.
Triangle and position limits remain unchanged.

## Evidence and acceptance

- Analytic tessellation integration suite: five tests passed, including sphere
  and both cone poles at LOD 1, 2, 4, 8, 16 and 32.
- Area suite: five tests passed on the final source, including the midpoint
  guard. Tests cover signed area, translation,
  reversal, self-intersecting chains and insufficient tolerance/work limits.
- Native rounded polynomial profile: four quadratic quadrants enclose 30 mm²;
  profile admission and extrusion passed without substituting a circle.
- Extended native regression: 780 B-rep and 547 NURBS library tests passed;
  three B-rep tests remain explicitly ignored. This run predates the midpoint
  guard; the final area suite separately passed with that guard.
- Added real worker acceptance: prepare four original curves, extrude over
  five millimetres and compare with the analytic volume 150 mm³.
- WASM snapshot `a86645ce1295debbd1de0e9d48e438f8cf6b5ce045e8dc41f78f31563f0e4068`
  passed 29 profile, real-worker and partial-annular protocol tests. This
  snapshot predates the final midpoint guard; final source acceptance is recorded above.
- The polynomial arch profile passed mouse and Tab/Enter browser preparation,
  cancellation, extrusion, STEP export, Undo/Redo and JSON reload in the new
  interface. OpenCascade independently accepted both STEP exports: one valid
  solid, volume 23.333333333333336 mm³, maximum bounds error about 1e-7 mm.
  These results use the same pre-guard WASM snapshot, not the final build.

Relevant tests are `planar_area`,
`profile_region::tests::polynomial_rounded_profile_has_proven_area_and_extrudes`,
`analytic_brep_tessellation`, `brepCurveExtrusionModelGraph` and
`mainSolidWorkerRealBoundary`. Browser acceptance uses
`check-solid-retained-profile-browser.mjs --general-nurbs` and its keyboard mode.
