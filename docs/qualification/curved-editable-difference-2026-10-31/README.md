# Exact editable curved difference under a common quadratic deformation

Rust API in `minkowski3d::curved`:
`certify_quadratic_difference(container, tool, coefficient, depth)` and
`verify_quadratic_difference(container, tool, result_model, certificate)`.
The result owns an editable native `Model`, exposed by `model()` or `into_model()`.

## Accepted geometry and proof

The accepted class is F(P) and F(Q), with the same global ambient map
F(x,y,z)=(x,y,z+k*x²), where P and Q are represented polyhedral materials.
The global inverse (x,y,z-k*x²) and determinant-one Jacobian are explicit.
Neither P nor Q needs to be convex. This is a bounded curved-source operation,
not arbitrary algebraic-domain deletion.

Both original Models first receive full checked source-atlas certificates.
Every original tensor knot cell must satisfy a complete exact coefficient
identity with F applied to an affine source chart. Original trim-loop nesting
is checked, including holes; every trim must have a complete straight inverse
image. The independently stored edge curves and trims are checked by the input
atlas proof. Original UV lifts are preserved by the source proof.

Exact positive-winding reference chains are generated from the original trims.
The new recipe chooses a rational fan origin off all boundary supporting lines,
preventing a rectangular-domain midpoint from subdividing just one side of a
triangular seam. Signed fans can lie outside concave trims; their complete closed
chain, not unsigned individual fans, determines material. Legacy shear replay
retains its previous origin recipe.

All reference source planes partition the container bounding box. Winding is
constant within each full-dimensional cell; cells in P and outside Q are kept.
Shared faces cancel, and exact global edge subdivision produces matching seams.
The boundary must have paired edges and cyclic connected vertex links. Boundary
triangles are mapped through F, including newly exposed tool-plane caps.

Connected coplanar reference triangles are merged into oriented regions. Internal
edges cancel, exact projected signed areas classify the outer loop and holes,
and branching boundaries are refused. Each region becomes an exact tensor
degree-(2,2) NURBS surface with its complete UV trim loops. Each shared edge
becomes one exact degree-2 curve with two face-local pcurves. Unused interior
triangulation vertices are removed from the editable topology. No control coordinate is silently rounded: conversion to Model's
binary64 representation must round-trip exactly as a rational. Unsupported
control coordinates are refused. The result is not a triangulated planar
approximation to those curved surfaces.

Connected triangle components become closed shells. Exact reference signed
volumes distinguish outward and cavity shells. Exact winding assigns each cavity
to its unique containing outward shell. Multiple bodies and enclosed cavities
are supported. Topology IDs are rebuilt and Model validation runs. Nonempty
results receive a fresh complete source-atlas proof, including cross-shell
separation. Empty material returns Model::empty and an explicit absent result
atlas (not a fictitious empty-surface proof).

The certificate binds the two original Models, coefficient, depth, exact
reference boundary, both source atlases and result atlas. Verification repeats
the whole operation and compares both the certificate and native Model. Producer
and verifier share the Rust solver; this is checked replay, not an independently
implemented theorem prover. Reference volume in tests is not a generic curved
mass-property API, though this particular map preserves volume.

## Bounds and remaining scope

Existing source-atlas degree/work/separation limits and exact arrangement bounds
remain. Output is limited to 1024 faces, coefficient magnitude to 1e6, depth to
0..3 and fan-origin search to 1024 attempts. Exhausted budgets, unsupported charts,
nonmanifold boundaries, nonrepresentable exact NURBS controls and unproved source
or result atlases return errors. The global source budget can impose lower limits
than the output constructor.

Still open: arbitrary independent NURBS carriers, arbitrary selected algebraic
atlas cells, nonlinear inverse-image trim curves not straight in the common
reference, irrational Model control coordinates, and general surface-surface
intersection cap construction. Existing enclosed curved subtraction is unchanged.
There is no new UI/MCP, STEP, WASM or browser qualification here.

## Validation

Native regression cases cover a curved crossing cut in a nonconvex L-shaped
container, exact new curved cap evaluation, result edit/triangle forgeries,
coefficient mismatch, cavity ownership, disconnected bodies, a through-hole
with two genuinely holed cap faces, empty material and
refusal of nonrepresentable exact controls. See result.json and native-tests.log.
The scoped hash ledger does not freeze all parallel workspace changes.
