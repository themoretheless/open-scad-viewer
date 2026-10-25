# Closed and degenerate surface intersections in Rust

This extends NURBS-0030 without changing the 200-capability milestone.

`nurbs_core::ss_intersection::construct_qualified_ss_branches` returns either an exact original quadratic-graph classification or the existing certified regular open-arc assembly. Each result owns its original surface pair and can replay its proof with `reverify`.

## Admitted exact family

Both original surfaces must be single clamped polynomial graph patches of degree one or two in each parameter, with uniform positive weights and the same axis-aligned invertible affine XY grid. Their native parameter domains may differ. Checked exact binary64 identities convert the original Bernstein heights into coefficients. The height difference must reduce to `a*(r-cu)^2 + b*(s-cv)^2 + k`; rounded coefficient identities and nonrepresentable centers are rejected.

Line/ellipse factor geometry is expressed in the original normalized UV chart. World stations retain the original surface lift, including common nonlinear height terms; a UV segment does not imply a straight world edge.

The classifier covers the entire source domains for each admitted result:

- A strictly interior ellipse has four algebraic cardinal vertices, four rational quarter charts, one closed branch and paired UV trims. Closure follows the exact graph, not endpoint proximity.
- A definite zero level is an isolated tangent point, with no fabricated curve.
- A rank-one zero level is a tangent double line with multiplicity two.
- An indefinite zero level with exactly representable slope and clipped endpoints has two crossing lines, four halfedges and one shared singular node.
- A rank-one negative level with an exactly representable square root produces one or two simple parallel line components after exact clipping; if both factors lie outside the original rectangle, the complete result is empty. Each admitted edge has multiplicity one.
- Identical heights produce a two-dimensional coincident-patch owner, not a curve. Proven disjoint levels produce an empty result.

Station queries return outward enclosures of the selected exact original point and both native UV coordinates. Irrational ellipse radii remain algebraic selectors; rounded square roots do not become topology evidence. Query work and coefficient work are bounded. Resource exhaustion propagates through the dispatcher and never silently buys a larger fallback budget.

## Remaining boundaries

This is not a universal singular NURBS solver. Clipped ellipses, general hyperbolas, irrational or inexactly clipped crossing slopes, nonrepresentable parallel factor roots, mixed quadratic terms, multiple spans, varying rational weights and unrelated XY parameterizations need additional classifiers. Unsupported exact-family inputs may use the existing complete regular open-arc proof if that proof succeeds. No sampled fallback receives a qualified owner.

The implementation is native Rust. WASM compilation establishes portability only; this extension does not establish editor transport, package installation or runtime UI support.
