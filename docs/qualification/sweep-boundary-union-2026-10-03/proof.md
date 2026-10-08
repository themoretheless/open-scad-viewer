# Complete retained boundary error composition

## Meaning and ownership

`continuousBound` at the completed progressive miter construction describes a two-sided Hausdorff bound between its complete retained boundary set and the original authored boundary set. The two sets include all ruled wall spans and, on an open path, the filled outer-minus-hole endpoint regions. It does not assert that either boundary is embedded, that shell orientation is correct, or that a volume approximation is bounded. Those obligations remain independent material-admission checks.

Only the owning constructor assembles the certificate. Its native level certificate, correction, actual retained decomposition, ideal domains, actual cap regions and endpoint plane predicates use the same original request and retained body. The streaming path snapshots numerical request data at first advance, keeps cancellation controls live and gives callers independent preview copies. Certificates do not propagate through arbitrary edited or derived graph nodes. Native profile-level reports remain partial and retain `continuousBound:false`.

## Walls

The native whole-level certificate encloses each original control trajectory on every retained traversal interval, using outward interval arithmetic. It includes scalar and affine law jets, transported or authored/guide frame jets, twist and the miter projection. Endpoint comparisons measure actual binary64 retained coordinates against the original ideal endpoint enclosures. Incoming and outgoing corner sides and the cyclic closing interval are audited independently.

With unchanged positive profile weights and common profile basis, the retained surface is the rational convex combination of those control trajectories. Thus the maximum control-trajectory error bounds every profile parameter and both directions of the paired surface map. Bounded section correction is added outward. Actual retained wall coefficients are either identical to the corrected sections or independently bounded against the original knot-span decomposition. Actual full unit-square face domains and complete body face ownership must be established; missing, repeated or orphan faces cannot provide two-sided coverage.

The final wall bound is the outward sum of native retained-level error, correction displacement and decomposition error. A missing term prevents the whole-boundary certificate.

## Filled caps

The source profile region is independently certified as planar, simple, regular and with the original outer/hole ownership. Nonzero plane normal and a nonsingular initial-frame projection bind this source region to the local profile coordinates. Strictly positive scale and affine axis laws and certified endpoint frames give an invertible affine image at each endpoint, preserving the region and its holes.

Actual retained cap regions are independently verified with their actual planar charts and trim curves. Exact region correspondence, or certified decomposition correspondence, preserves the paired outer/hole boundary layout. Endpoint contour error, correction displacement and cap decomposition error are summed outward to obtain `epsilon` for each paired 3D boundary point. The original ideal and actual retained cap planes must have a certified nonzero normal dot product.

For two planar material regions in one plane, pointwise paired oriented outer/hole boundary distances at most `epsilon` imply two-sided region-set Hausdorff distance at most `epsilon`. To prove this, take a point at distance greater than `epsilon` from the other filled region. The straight boundary homotopy cannot pass through this point: every homotopy segment stays within `epsilon` of the other boundary. The total outer-minus-hole winding number is therefore unchanged, excluding membership in the first region. Apply the argument in both directions. Intermediate homotopy curves need not remain simple; the actual two endpoint regions and paired loop ownership are independently certified.

For distinct nonparallel planes, orthogonally project one material region onto the other plane. The nonzero normal dot certifies that projection is a homeomorphism, including all holes. Projection does not increase paired boundary distance, so the planar region-set error is at most `epsilon`. Signed height relative to the destination plane is an affine function on the original plane; its absolute maximum over the filled compact region is attained on its boundary and is at most `epsilon`. Orthogonal planar and normal errors then bound Euclidean error by `sqrt(2) * epsilon`. Apply the same argument in the reverse direction. The square-root multiplier is rounded outward.

For exactly parallel planes, height is constant. Paired boundary distance at most `epsilon` gives the sharper planar distance at most `sqrt(epsilon^2 - height^2)`; the same winding argument and orthogonal reconstruction give total distance at most `epsilon`. This sharper transfer is used only when exact parallelism is certified. An unresolved parallelism check uses the general transfer, while unresolved projection or region ownership yields no filled cap bound.

## Union and budget

For paired sets whose individual two-sided Hausdorff distances are bounded, the union distance is bounded by their maximum. The completed boundary certificate therefore takes the maximum of the final wall bound and both filled cap bounds. A closed path has no cap obligation, but includes its cyclic wall interval. Taking a maximum adds no floating-point rounding.

The certificate distinguishes existence of a finite continuous bound from satisfying `max_deviation`. `withinBudget` compares the complete union bound with the original budget, never a wall-only or sampled estimate. Missing proofs produce `errorUpper:null` and `continuousBound:false`; a finite bound over budget keeps the bound but records `boundary-budget-exceeded`. Invalid budgets cannot produce a certificate.
