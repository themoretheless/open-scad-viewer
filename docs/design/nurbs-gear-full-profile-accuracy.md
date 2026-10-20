# Inclusive nominal sharp-gear profile accuracy

The Rust gear report now distinguishes complete nominal profile accuracy from geometric embedding. `profile_error_upper` is available only when every saved involute flank, rational arc and below-base transition line has a continuous bound against its nominal GearSpec component, within the existing 1e-6 budget. These same-parameter component bounds imply a two-sided distance bound between the complete represented profile and the nominal sharp profile. Undercut cutter geometry and root fillets are separate shape definitions.

`derive_gear_parameter_bounds` additionally encloses the low/high tooth half-angles using the exact nominal involute phase theta-atan(theta). Arctangent uses outward half-angle reduction followed by the existing bounded alternating series; no platform atan is trusted. The root/base branch and nominal angular ordering must be separated for the entire parameter family. Uncertain branches leave the candidate unqualified.

`primitives::certify_circle_arc_radians` checks actual saved normalized positive rational quadratics against radius/start/end parameter boxes. It compares centered homogeneous poles and weights, then bounds the rational quotient difference using positive denominator lower bounds. It includes saved-weight rounding and endpoint ownership changes. Angle boxes must be separated, remain in the Taylor owner's range, and define arcs shorter than a half turn. `certify_radial_segment_radians` bounds original constant-weight line interpolation by its maximum endpoint discrepancy against nominal polar endpoints.

The shared outward sine/cosine owner now evaluates its Taylor polynomial at an exact stored midpoint and adds the input interval's outward radius using the global derivative bound one. This avoids large alternating-series interval dependency while retaining the Lagrange remainder and outward arithmetic. It does not assume the midpoint computation or platform trigonometry is exact.

The B-rep builder uses original construction indices for each tooth arc, radial transition, bore and rim. Internal reversal is applied to the nominal parameter mapping as well as independently checked actual knots. Every component must be admitted; the report never substitutes point samples for a missing bound.

`ideal_wall_error_upper` combines the profile bound and independently certified saved-wall/nominal-twist bound with outward addition. Rotation preserves profile error, so the triangle inequality covers the complete original wall domain. A refused component or wall proof leaves this field None. This does not certify face embedding, cap regions, shell nesting or a three-dimensional solid.

Independent tests cover full parameter families, saved weight/endpoint perturbations, descending arcs, forbidden sweeps, nominal half-angles and complete radial lines. Previous involute and helical certificates are rechecked because they share the sine/cosine owner. Gear fixtures include external/internal profiles, a bore, spur/helical/herringbone bodies, exact joins and independent small-profile embedding certificates.

The native milestone remains 189/200. Larger and internal whole-profile embedding, independent solid admission and the other planned catalog entries are unfinished. Native accuracy reports do not qualify packaged browser/editor/STEP delivery.
