# Native perspective injectivity proof

`nurbs_core::surface_injectivity::certify_projective_projection` now accepts two affine numerator rows and one affine denominator row, each over `[x,y,z,1]`. It transforms outward intervals of the original homogeneous NURBS control net, bounds the rational projection derivatives, and combines every knot rectangle into one global Jacobian hull. A constant inverse matrix with contraction bound below 1 proves injectivity over the complete natural chart. Subdivision never substitutes separate local proofs for that global condition.

Every visited section must have a strictly positive denominator enclosure. Incomplete coverage, periodic charts, a singular projected derivative or failure of contraction yields no certificate. Input validation rejects nonfinite coefficients and invalid work limits. The method does not construct rounded projected control points.

All eight surface-injectivity tests pass, including positive and zero/crossing denominator cases, an incomplete 15/16 budget, incomplete 31/32 multi-span coverage, a globally folded chart with locally invertible sides, and a valid periodic chart. The B-rep regression proves every one of the eight sphere charts for radii 2, 3, 0.00001 and 1000000 using 16 cells per chart, preserves its source and refuses a 15-cell budget.

This API is a native proof foundation. It is not yet part of the automatic face diagnostic report or checked worker protocol. It does not certify edge/lift agreement, intersections between separate faces, shell nesting or filled-volume validity. Radius 2 still fails exact boundary agreement. The current WASM packaging process began before this API was added and therefore must not be cited as its browser qualification.
