# Trimmed NURBS, exact branch graphs and approximation topology

This extends the [2026-10-18 certificate delivery](../certified-geometry-followup-2026-10-18/README.md).
Geometric construction, exact predicates and verification are Rust-owned.
The distribution and runtime evidence for this delivery are bound in
[manifest.json](manifest.json); [source-receipt.json](source-receipt.json) identifies the frozen common source.

## Trimmed source faces

The new NURBS enclosure certificate is version 2, under the existing scope
`represented-nurbs-filled-minkowski-enclosures`. Original outer wires and holes
can be rational, nonrectangular and nonconvex. Rectangular faces retain their
exact knot-cell construction. Other faces use oriented rational cone chains
from an interior point of the natural parameter rectangle to the original
pcurves. Hole loops retain their orientation. The signed chain represents the
authored UV winding, including cancellations outside a nonconvex region or
inside holes; it does not replace the region by its convex hull.

The kernel reconstructs exact homogeneous B-spline restrictions from input
bits. For each rational fan cell contained in one original tensor span it
composes the original surface with the entire rational fan, using exact tensor
Bernstein products. Coefficients bound the continuous surface against the
nominal triangles. Cells crossing surface knots use conservative bounds over
the complete intersected original chart cells. Affine charts have a sharper
curve-to-chord bound and an exact planar reparameterization of the fan cells;
linear trims consequently retain zero geometric error there.

Knot preimages on seams need not be rational. The exact numerator equations
for every surface knot are isolated with Sturm arithmetic. A rational point in
each sign-invariant open interval chooses the appropriate original chart;
whole-polynomial composition identities prove equality on that interval.
Continuity establishes equality at the algebraic cuts. Mesh vertices are not
placed at rounded approximations of these critical parameters. Full-multiplicity
surface breaks require exact equality of both one-sided seam functions. Pcurve
span joins must also agree exactly.

Pcurve membership in the active UV rectangle is checked with exact scalar
nonnegativity, rather than requiring the control hull to fit. An interior
non-dyadic tangency such as `u=3t-9t²/4`, touching `u=1` at `t=2/3`, is admitted
even though its middle Bernstein control is `1.5`. A genuinely outside curve
refuses. Periodic lifts outside the active rectangle are not introduced by this
operation. Source authority remains the authored oriented face images and
their positive-winding material. The certificate is not a blanket validation
of independently stored 3D edge definitions or unrelated B-rep metadata.

For closed signed fan chains, exact directed-edge cancellation is sufficient
to construct winding material. Coincident internal fan contributions may have
more than two incidences. User mesh inputs retain their previous stricter
two-incidence admission. Complete exact polyhedral decomposition regularizes
the nominal filled set before Minkowski addition.

The new source inner enclosure is exact convex subtraction of all swept tubes.
The outer is a checked convex cover of the nominal set and tubes. This outer
cover may bridge concavities or holes; it is an inclusion bound, not a claim
that the source or nominal sum is convex. A separate filled Hausdorff check
first tries tight convex coverage. Exact anchors in the nominal and proved
inner material supply a conservative finite fallback for partitioned regions.
If no such proof exists, the metric remains null. The final reported Euclidean
bound includes actual output rounding and may exceed the per-surface tolerance.

New nested polygon boundaries use strictly positive dyadic barycentric face
centers. This avoids unnecessary thirds on otherwise exactly representable
inputs while preserving interior fan triangulation and complete edge splitting.
Legacy enclosure version 1 retains its original derivation and centroid recipe.

A separate source-seam preflight checks the exact original face images before
expensive approximation search. The current analytic sphere constructor has a
nonzero exact equatorial gap between its stored north/south face images; it
receives `MINKOWSKI_3D_SOURCE_SEAM`, including through WASM, rather than a false
certificate or repeated refinement. The constructor is not modified here.

## Exact decomposition into branches

Span-intersection certificate version 3 binds a branch decomposition to each
common component. Versions 1 and 2 remain accepted under their original witness
contracts, without new branch claims. Missing, additional or altered branch
claims fail verification.

The graph separates the vertical content from the squarefree primitive
bivariate relation. Its projection polynomial includes leading-coefficient,
discriminant, domain-boundary and removed-diagonal events. All real critical
parameters in the unit interval have exact defining polynomials and rational
isolating intervals. At each critical algebraic value alpha, a modular Sturm
calculation in Q(alpha)[t] isolates every real fiber root. A nonunit coefficient
splits the defining factor; signs refer to the particular isolated real root.
No floating-point root position or sampled connectivity is trusted.

Between consecutive critical parameters, branches are the ordered real root
sections of the primitive relation. A rational sample records their ranks and
isolations. One-sided incidence uses rational horizontal guards between the
critical fiber roots. Exact root counts prove that no branch crosses a guard
before the critical fiber; thus every slab branch has a checked endpoint node.
`left` and `right` list these incidences in branch-rank order. Vertical components
have explicit `verticalArcs` between their ordered fiber nodes. Self-source
diagonal nodes/sections are deleted in the original global parameters, including
different source knot spans.

This includes irrational extrema and joints, transverse nodes, cusps, isolated
real points, horizontal/vertical components and retracing. An entire constant
parameter square is explicitly dimension 2 and cannot be mislabeled as a finite
collection of curve branches. Finite residual intersections and stationary spans
remain in their separate complete censuses. The API returns
`branchDecompositions`; `finiteEventsExhaustive` remains false for nonisolated loci.

## What approximation topology proves

Certificate version 2 has independently reconstructed `sourceTopology` receipts.
These prove a sufficient radial-homeomorphism condition; they do not infer
topology from a small distance or matching sampled meshes.

1. Construct the strict radial kernel of the exact nominal boundary by
   intersecting all oriented facet halfspaces. Its exact interior center can
   lie in a nonconvex solid. Check oriented edge closure, single-cycle vertex
   links, strictly positive radial triangle determinants and winding degree 1.
2. For a B-rep source, validate its connected closed topology and original UV
   profiles. Check exact face-image gluing on every authored edge and exact
   source-vertex positions. Pcurve tangents must be regular, and zero-angle
   cusp stars remain unproved. Convex, reflex and ordinary smooth trim joins
   are handled under the positive-sector condition.
3. Exact Bernstein derivative numerators and rational interval products prove
   strictly positive oriented radial Jacobians on every original surface chart.
   When independent intervals lose correlated terms, a complete tensor
   Bernstein polynomial for `(N-cW) · ((Nu W-N Wu) × (Nv W-N Wv))`
   proves the sign directly over the original rational chart (denominator W^5
   is positive). A whole-chart stereographic sphere regression exercises this
   stronger path; it does not establish global source gluing.
   Full chart coverage includes all knot spans and both sides of internal
   knots. Incomplete or merely numerical rank evidence cannot pass.
4. The center must lie outside every closed swept tube. Its original-chain
   winding degree must equal 1. An orientation-preserving local spherical map
   of the closed source manifold has positive local index at every seam/vertex;
   degree 1 makes it a global radial graph. Both source and nominal filled sets
   are consequently closed three-balls, with continuous positive radial functions.
5. When both operands pass, their Minkowski sums are strict radial sets about
   the sum of their centers: scaling either source strictly inward maps its
   compact set into its interior. The sum therefore also has a continuous
   positive radial function. Radial interpolation gives a homeomorphism of
   original and nominal filled sums. The separately proved full rounding
   homotopy extends this result to the actual f32/f64 output boundary.

`approximationTopology` reports this proved condition separately from
`roundingTopology`. A valid enclosure can still have null approximation topology.
For example the holed tube fixture retains its nominal hole and has valid source
inclusions, but it has no radial kernel and receives no radial topology proof.
General higher-genus/non-star-shaped solids and changes of knotting do not
inherit a theorem from this criterion. The UI explicitly distinguishes these
unproved cases. This is not a proof-assistant derivation: arithmetic, algorithm
correctness, the stated topological arguments, compiler and runtime remain trusted.

## Bounds and verification

The existing degree 1–8, positive-weight, exact-coordinate, 64-face and
8192-source-control limits remain. Original seam preflight is limited to
2048 coedges and runs before the approximation depth search. The shared station limit is 33; rectangular
chart cells are bounded by 512. Nominal authored fan input remains bounded by
1024 vertices and 2048 triangles. Exact arrangements, hulls and coefficient
bit sizes retain their explicit limits. Branch projection degree is bounded
by 256; algebraic root/fiber isolation and factor splitting have finite budgets.
An exhausted or unproved predicate refuses its claim.

`scripts/check-trimmed-branches-topology.mts` calls the raw staged WASM directly.
`scripts/check-certified-followup.py` orchestrates a separate Rust checker and
does no geometric computation. New certificates are replayed in native Rust;
changed sources, graph connectivity, cell bounds and topology receipts are
negative fixtures. Earlier polygonal and planar certificate corpora are checked
again. `manifest.json` records the final selected test counts, hashes, browser
Undo/Redo/export checks and measured distribution sizes. These finite fixtures
are qualification evidence, not a theorem of success for every admissible input.

## Runtime delivery

A kernel rebuild must also bind the actual runtime provider identity. The first
browser load caught the previous 68dd record refusing the replacement artifact.
This delivery adds mesh/B-rep runtime manifests v4 for the new kernel; the
published v3 manifests remain byte-identical (`preserved-v3-manifests.json`).
The existing admission and fingerprint checks stay enabled. The provider corpus
was rerun on the new artifact (19 files, 303 tests), and the fresh runtime audit
executes both real providers. An import-inventory assertion was updated for the
new immutable historical evidence file; the final provider corpus is green.

The browser operation uses the original nonrectangular rational cap trims of
the authored cylinder, with the source model retained. The exported certificate
is checked by the standalone Rust program; Undo restores two original bodies
and Redo restores the result as the third. The final clean browser load restores
all three bodies without warnings or errors. `browser-log.json` intentionally
retains the initial, corrected old-artifact admission failure;
`browser-fresh-load-log.json` records the clean final launch.

The 68dd Sweep native owners were not changed by this work. Their old
qualification remains attached to that old artifact. The new delivery reruns the
100-file public modeling/Sweep corpus without increasing its timeout and records
its own result; deeper historical Sweep proof/UI evidence is not relabeled as a
new universal guarantee.

Final artifact: `b4cd5d582ad11bb4f11aebbc0567f98bbf33df7f8a2d57f3a28a2b1b00cc9e06` (12071071 bytes).
Passed: 142 focused native tests, 154 host tests, 5 targeted UI tests,
303 provider tests and 736 public modeling tests in 100 files.
The fresh proof corpus contains 36 certificates and 92 rejected forgeries;
9 older certificates and 15 older forgeries also replay independently.
These are scoped regression results; the full repository suite was not run.
