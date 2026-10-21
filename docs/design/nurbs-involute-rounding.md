# Stored involute curves: inclusive error qualification

`nurbs_core::involute::certify_quintic` verifies an actual stored quintic
Bezier chain against the analytic base-circle involute. The supplied finite
binary64 center, radius and angular endpoints define the nominal curve as
exact input data. It accepts normalized, unit-weight C0 quintic chains with
1..51 spans, and roll angles within [-8,8]. Unsupported layouts are refused.

`certify_quintic_placed` additionally supports a planar rotation and reflection,
2D or 3D stored controls, and descending roll parameters for reversed flanks.
The placement angle lies within [-8,8] and reflection is exactly -1 or +1.

For each original knot interval, outward arithmetic encloses the ideal
endpoint-Hermite control points. The maximum Euclidean discrepancy between
these enclosures and the actual stored poles bounds the difference of the two
Bezier curves by nonnegative partition of unity. The sixth-derivative bound
`radius * sqrt(25 + theta^2)` and endpoint-Hermite remainder add a continuous
coordinate-combined geometric bound over the entire interval. The original
stored knot positions determine the nominal angular endpoints and widths;
nonuniform rounding of knots and parameter reversal are therefore included.

Sine and cosine evidence comes from Taylor polynomials at zero with a
Lagrange remainder `abs(theta)^64 / 64!`, evaluated using outward arithmetic.
Platform sine/cosine calls are used only by the constructors and test oracles,
not as proof. The certificate returns the Hermite remainder, maximum pole
error and combined bound separately. It makes no claim about future rounding
inside a caller's point-evaluation algorithm.

`brep_core::gear::involute_flank` reserves half the 1e-6 allowance for pole and
knot placement, constructs the flank, and checks the actual final rotated or
reversed curve. It refuses more than 32 controls or an inclusive bound above
1e-6. Gear outlines now use this checked constructor. The geometry report's
`flank_stored_parameters_rounding_certified` distinguishes this completed
scope from `flank_rounding_certified`: earlier derivation of radii/angles from
module, pressure angle, backlash and clearance is not yet certified. Helical
sweep rounding, gear-root topology and independent global solid qualification
remain separate. Gear catalog entries remain planned.
