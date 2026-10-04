# Collapsed transition tip continuity

The current transition has the parametric form

`S(u,v) = ((R-r(u)+r(u)*a(v))*e(u), H-r(u)+r(u)*b(v))`,

where `e` is the planar unit circle and `(a,b)` is a quarter circle from
`(0,1)` to `(1,0)`. The radius uses a cubic smoothstep and vanishes at one end.

On the plane contact (`v=0`) the tangent plane is horizontal for every
positive radius. On the cylinder contact (`v=1`) its normal is radial and
horizontal. As `u` approaches the zero-radius endpoint, both contact points
approach the same original sharp-rim point, while these normals remain
orthogonal. Consequently the tip has no unique limiting tangent plane.
Zero first derivative of the radius law does not remove this singularity.

`collapsed_transition_tip_has_distinct_normal_limits_not_a_regular_g1_point`
tests entry and exit, both angular directions, and three distances to the
tip. It also checks the undefined normal on the collapsed boundary itself.
This numerical regression supports the analytic argument above; it is not
an interval proof for arbitrary surface data.

The preview may represent a solid with an explicitly singular endpoint,
but must not claim regular G1 continuity across the complete closed span.
Enabling a production operation requires an explicit endpoint contract,
validated boundary quotient topology, and absence-of-intersection checks.
If a regular end transition is required, the zero-radius construction must
be replaced with an end treatment whose adjoining retained surfaces also
have a compatible tangent plane. Interior contact and constant-span seam
checks alone cannot establish that requirement.
