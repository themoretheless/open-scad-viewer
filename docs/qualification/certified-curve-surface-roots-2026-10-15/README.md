# Original NURBS curve/surface roots and shared-edge partitioning

## Native root proof

`nurbs_core::intersection::certified_curve_surface::intersections` searches the
complete original curve/surface parameter product. It uses positive-weight,
nonperiodic NURBS and treats every nonempty original knot-span product explicitly.

For original homogeneous fields C=(Xc,Wc) and S=(Xs,Ws), the tensor polynomial
`Xc(t) Ws(u,v) - Xs(u,v) Wc(t)` has the same roots as the geometric intersection.
All coefficients are outward enclosures of the original data. The curve
extractor's translated origin is restored before constructing the polynomial.
Rounded extracted patches, Newton residuals or display samples grant no proof.

A root requires a strict three-dimensional Krawczyk inclusion, an independently
proved invertible preconditioner, and a contraction bound below 0.5. All other
cells are excluded continuously or retained as unresolved. Strict root-owning
cells are exact dyadic subdivisions of the original spans. Boundary/knot roots,
tangencies and coincidence remain unresolved rather than disappearing from
complete coverage. Different parameter roots are retained even when they share
one world position.

Refinement rebuilds fields from original coefficients. Coordinates already at
the binary64 enclosure floor receive a wider search interval so they do not
prevent another coordinate from being refined. This changes the search box,
not source geometry, and every refinement must pass the same strict conditions.
The original owning-cell existence/uniqueness proof is retained if precision or
work is exhausted. Root positional accuracy remains a separate authoring gate.

The sign of `C_t dot (S_u cross S_v)` is proved from the preconditioner determinant
and contraction. The straight homotopy from I to RJ stays invertible, hence
`det(RJ)>0`; the proved sign of det(R) gives the sign of det(J). Positive rational
weights and positive span scalings preserve the geometric crossing sign. This
is an oriented crossing, not a Boolean inside/outside classification.

Fixed limits: 100000 proof cells, 1024 original span products, 4096 tensor
coefficients per product and subdivision depth 48. No existing budget is raised.

## B-rep integration

`edge_partition::partition_at_surface` recomputes this report on the actual
source edge and support, requires complete coverage, checks the selected root's
positional enclosure, and atomically updates both incident face-local uses.
The root and child/lift checks share one work ledger. Split lineage and unchanged
edge identities are retained. The support currently means the full rectangular
NURBS domain; holes and trimmed support boundaries need independent admission.
The returned model remains a candidate until freshly qualified globally.

## Qualification scope

Native tests and STEP evidence are recorded below. This work does not construct
surface/surface intersection curves, classify retained face regions, assemble
fillet ends or unequal-radius converging corners, or establish periodic/tangent
root support. Thickness, whole-fillet G2, general self-intersections, the three
named acceptance parts, WASM/worker/UI/history and the full P0 matrix remain open.
The legacy approximate curve/surface solver is unchanged and remains uncertified.

## Results, 2026-10-15

35 distinct native integration tests passed: eight certified curve/surface cases,
eight edge-partition cases, eight original-boundary correspondence cases, six
existing certified curve/plane regressions and five existing line/surface
regressions. Coverage includes a cubic rational surface and nonplanar cubic
rational curve, rational weights on both sources, multiple original spans and
nonunit domains, opposite crossing orientations, two parameter roots at one
world position, and explicit unresolved boundary/tangent/coincident cases.

The updated `owned_edge_partition_audit` example freshly qualified three
partitioned 10×8×12 mm control cuboids. Two use planar cuts; the new third uses
a genuinely curved bilinear support and a root near t=0.105 (world x≈1.05 mm).
All three passed exact boundary lifts/joins, trim validity, embedded-boundary
and volume admission without increased budgets. The curved-cut authored vertex
had a certified root-position error upper bound of 2.486899575160352e-14 mm.

All three STEP v5 exports independently passed OpenCascade validity, one-solid
count, bounds and the design volume 960 mm³. Volume error was zero; the maximum
reported bounding-box discrepancy was 1e-7 mm. These are control cuboids, not
acceptance of the bracket/flange/enclosure chains.

Generated local evidence:
`output/qualification/curved-surface-partition-2026-10-15/`.
No WASM or UI artifact is published by this qualification.

### Follow-up: actual trimmed support regions

`trimmed-face-crossings-2026-10-15` adds whole-UV-box root admission against a
proven outer contour and holes, including safe exclusion of unresolved surface
cells wholly outside the face. It also binds oriented shell use and preserves
retained root ordering. This is face-domain admission, not authoring retained
face sectors or complete fillet bodies.
