# Material orientation after complete exact smooth-carrier admission

The new Rust factory takes an opaque `ExactSmoothCollection`, not a caller
report or a collection of sampled normals. This retains one original admitted
body, complete original carrier correspondence, exact gradient/Hessian identities
at the declared classes, and nonzero regular or factored-limit normal coverage
on each complete smooth carrier. The current exact native identities admit
analytic original Bezier charts and exact natural meridian maps. Their positive
weights and full regularity/limit coverage are indispensable premises.

For a connected original smooth carrier, equality of its two tangent planes
and nonzero continuous normals makes the relative normal sign a continuous
function into the discrete set {-1,+1}. It is constant. A single rigorously
enclosed interior anchor can therefore determine this sign on the full carrier.
The anchor supplies neither sampled equality nor sampled regularity evidence.

For each face the admitted original source region owns its UV chart winding.
The material normal is `chart_winding * (S_u cross S_v)`. The factory freshly
encloses both original normals at the same carrier point and requires their
material-signed dot product to have a strictly positive lower bound. Global
body reversal flips both signs and leaves this test invariant. A negative
signed dot refuses the smooth material orientation; an uncertain dot also refuses.
No failing smooth request is silently relabeled C0. Explicit C0 entries are
retained and do not receive smooth material authority.

The anchor must belong to the admitted restriction, including root-valued
endpoints. Both endpoint enclosures are mapped to the canonical carrier. A
representable point strictly between their disjoint enclosures is selected;
overlapping enclosures or adjacent floating-point endpoints refuse. Inverse
correspondence, original UV evaluation and original normal bounds are recomputed
on both sides. One shared step/curve-span/normal-span ledger covers all declared
edges. Exhaustion yields no complete owner.

The distinction is necessary for a real geometric counterexample: extrude the
simple cusp lens bounded by `y=x^2` and `y=-x^2`, `-1<=x<=0`, along Z. Its two
regular walls have exactly the same tangent plane at the cusp edge, but their
material normals oppose. Tangent-plane equality cannot authorize a regular
smooth material seam there. A correct smooth continuation, including a genuine
G1 curvature jump, must instead retain positive material-normal agreement.

This additional owner establishes orientation of exact smooth carriers. It is
not yet an ordinary-vertex graph-disk proof, complete face C1/C2 continuity or
whole-boundary G1/G2, and supplies no new ideal loft/sweep law-to-shell identity.
The active full objective remains open. Native regression validation is pending;
this theorem document alone is not a qualification receipt.
