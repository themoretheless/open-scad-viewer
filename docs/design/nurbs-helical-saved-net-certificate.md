# Inclusive saved helical surface certificate

`nurbs_core::helical_sweep::certify_stored` independently checks a saved surface against the complete original rational-profile helical sweep. Height, start angle and twist are supplied as independently justified intervals. The input curve and saved surface are immutable.

The saved U degree, knots, periodicity and positive weights must equal the original profile exactly. Every V row must use the same original weight throughout. The certificate accepts normalized degree-one spans or full C0 quintic Bezier spans, with up to 256 original controls in either direction. Original V knot differences determine span widths; nonuniform and rounded span boundaries are included.

Outward Taylor sine/cosine bounds enclose ideal rotating-pole jets at each original V break. The certificate builds interval endpoint-Hermite control points and measures their discrepancy from every actual saved pole. For each row/span it adds an outward continuous interpolation remainder: sqrt(2)*radius*abs(twist*span_width)^6/46080 for quintics, or sqrt(2)*radius*abs(twist*span_width)^2/8 for lines. The vertical motion is affine and has no interpolation remainder. Positive rational U basis coefficients form a convex combination, so the maximum row bound covers every original U and V parameter.

The resulting error includes saved-pole arithmetic, actual knot widths and supplied parameter uncertainty. It does not assume platform trigonometric accuracy and does not use samples as the proof. Invalid, nonfinite, nonmatching bases, nonconstant V weights, unsupported knot layouts or angle enclosures outside the Taylor owner's -9..9 domain refuse a certificate. Rounding of a later point evaluator is separate.

`involute::derive_gear_twist_bounds` encloses height*tan(helix_degrees*pi/180)/(module*teeth/2), using the existing Machin/Taylor parameter owner. Exact zero helix has an exact zero interval. Signed angles below 80 degrees, supported module/tooth/height ranges and both herringbone halves are covered.

`brep_core::gear::gear_with_report` checks every actual saved wall with the nominal twist intervals. Only if all wall bounds are at most the existing 1e-4 budget is `sweep_rounding_certified` true; `sweep_deviation` then includes those bounds. A refused or over-budget certificate leaves the candidate explicitly unqualified. The wall bound is relative to the saved outline. Its error must be combined with separately established nominal flank/root-profile error before claiming an ideal complete gear bound.

Root transitions, whole-profile embedding and solid geometric admission remain unfinished. The four gear catalog entries remain planned, and the native total remains 189/200. This certificate advances their existing implementation rather than introducing another catalog item.
