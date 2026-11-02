# Original pole collars, finite native qualification

The new private `PolarCollar` token refers to the actual original NURBS surface and a named U/V boundary. It proves injectivity modulo that complete collapsed side on a caller-proposed normalized inward collar. Original interval Bernstein polynomials are restricted directly; no rounded rational patch substitutes for the original map. The legacy whole-chart API still checks extent 1.

The radial/angular map has a positive radial derivative, positive angular derivative and a strictly positive weighted dominance margin over the entire collar. Equality of two nonpole source points forces equality in this extended map, hence equality of their parameters. The radial coordinate separates all nonpole points from the pole. The compact quotient therefore maps homeomorphically onto its image. Away from the collapsed side the radial/angular coordinates are smooth functions of the source point with nonzero denominators; their nonsingular Jacobian also proves rank two of the original surface there.

`TangentPoleCollar` freshly composes that proof with the exact uniform plane-limit proof on the same original surface and boundary. `brep_core::source_pole_tangency::qualify_local` binds both private facts to the actual owned collapsed restriction, including retained parameter/root subintervals. No caller-written proof flags can admit a pole. Exact work and collar cells have separate explicit limits; no partial token is returned on refusal.

Regression coverage includes a planar surface that folds away from its pole: its local collar succeeds while the complete chart still refuses. Other cases cover all four natural sides, nonunit domains, positive rational weight scaling, repeated rays, a noncollapsed side, invalid radii and one-short cell/exact-work budgets. The source definition and source pointer are preserved.

## Remaining scope

This does not prove that the complete incident fan is a regular disk, that other charts cannot intersect the collar, or a unique curvature/G2 limit at the pole. Higher-order radial degeneracies and charts outside the sufficient polar theorem still refuse. General ideal loft/sweep regularity, global disjointness, topology and full frame/cap/closed-seam qualification remain open. Sharp joins retain explicit C0. No TypeScript geometry or WASM release change is made.

The separate PR 61 archive restoration repairs immutable historical byte identities only; its general architecture audit remains red. This native receipt is not release or complete semantic G1 qualification.
