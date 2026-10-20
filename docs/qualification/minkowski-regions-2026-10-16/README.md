# Filled Minkowski regions and expanded Bézier certificates — 2026-10-16

## Implementation and claims

The Rust region kernel implements the Minkowski sum of two compact filled
planar regions, including nonconvex boundaries, holes, disconnected components
and positive-winding self-crossings. It also implements regularized Euclidean
disk dilation and erosion. Zero-dimensional erosion remnants are discarded by
the filled CAD region convention. Erosion beyond collapse does not recreate a
positive island from an inverted raw offset loop.

For represented polygonal sources the sum is exact rational geometry. For
curved Bézier/NURBS sources the certificate provides exact rational inclusions
`inner ⊆ true filled result ⊆ outer`, and a nominal polygon for modeling. Equal
inner/outer regions prove an exact set result, including proven empty material.
The tolerance controls source homotopy and disk-kernel enclosures; it does not
assert a universal Hausdorff error or identical topology of the final result.
Binary64 modeling polygons have a separately checked rounding bound. The
certificate's inclusions refer to its exact rational rings, not rounded meshes.

The general-curve path is connected to the existing retained-profile offset
operation. The previous line/circle analytic route remains. New explicit WASM
and TypeScript operations expose the two-profile sum, signed disk offset and
certificate verification. A valid exact region certificate is returned even
when conversion to a regular numeric modeling profile must refuse a contact.

## Proof construction

1. Each original knot span is extracted by exact rational homogeneous B-spline
   restriction. A supplied binary subdivision tree is checked for complete,
   ordered coverage. Positive Bernstein weights and homogeneous control bounds
   prove the complete curve-to-chord homotopy, including zero jets and cusps.
   Exact collinear paths contribute the same winding as their endpoint segment;
   a closed retracing line contributes zero filled material.
2. Outside a checked union of chord tubes, source and polygon winding agree.
   Subtracting the tubes gives an inner region; adding them gives an outer
   region. No simplicity or nonzero-tangent premise is used for this enclosure.
3. Exact vertical decomposition covers filled polygonal material by convex
   cells, preserving holes. The convex hull of every cell-pair vertex sum is
   their exact Minkowski sum. Complete union of all pair sums gives the result.
   Convex recognition is invariant under cyclic vertex rotation, including the
   reflected erosion kernel.
4. For erosion choose a kernel point b0 and candidate D=A-b0. A bounded universe
   contains D+B. Then regularized erosion is D minus the Minkowski sum of the
   closed complement of A with -B. Convex A uses independently equivalent exact
   support half-plane clipping. This implements global filled-region semantics.
5. Disk polygons are independently checked by exact vertex norm and supporting
   edge distances. Their stored quarter-turn/reflection symmetry is exact;
   near-zero cardinal trigonometric artifacts are not admitted as tiny edges.
6. All region crossings and overlaps are split exactly. Adjacent-face winding
   uses linear predicates at M+epsilon*N with epsilon→0+: the first nonzero
   rational coefficient determines each sign. No finite probe or unproved
   tolerance chooses material. Output rounding checks crossings, orientation
   and hole ownership.

The standalone Rust checker reconstructs source restrictions, subdivision
coverage, homotopy inequalities, disk inclusions, convex decomposition,
Minkowski/erosion construction and every claimed output ring. It rejects source
changes, missing leaves, false disk vertices and altered result boundaries.
The exact arithmetic implementation, compiler and runtime remain trusted; this
is executable certificate verification, not a proof-assistant verification of
Rust or a formal certificate of the displayed 3D mesh.

## Bézier extensions

Algebraic intersection producer version 3 adds a complete stationary-root
census to the injective self path. A squarefree exact gcd of homogeneous tangent
numerators supplies rational isolating intervals, including a stationary point
at parameter 1/3. Legacy version 2 injective certificates remain readable but
explicitly return `stationaryCensusComplete=false`.

The per-knot-span certificate covers every required source-cell pair, using
exact source nets throughout. It represents finite events and positive-length
coincident components with exact polynomial relations and image ranges.
Half-open ownership removes duplicated knot events; one-sided stationary roots
are retained and deduplicated. Missing or reordered pairs refuse.

The agreement checker also accepts exact affine identities without a monotone
coordinate. This covers identical/reversed affine bases with cusps or complete
retracing, alongside the earlier nonlinear multi-span image proof.

The new quantified image-set certificate removes the monotone-coordinate and
regular-jet premises entirely. For each source cell it checks primitive
squarefree common bivariate factors against target cells. An exact projection
contains all leading-coefficient, discriminant and unit-interval endpoint roots.
A complete Sturm census and one checked real target fiber on each projection
section prove coverage on a dense set of source parameters. Continuity of the
source and compactness of the finite union of target images extend coverage to
all critical parameters. Reversing the two source roles proves exact equality
of image sets. This is distinct from a parameter bijection or traversal count.

Fixtures include a nonmonotone line retraced at a different nonlinear speed,
a cubic cusp whose degree-six reparameterization reaches the cusp at sqrt(1/2),
and coverage switching between two target cells at an irrational parameter.
A larger retraced image and a wrong target choice on one section are rejected.
Boolean duplicate admission uses this fallback only with matched endpoints and
already admitted injective pieces. Multi-span validation preserves original
source definitions; Boolean and normalization work on active retained spans.

## Explicit boundaries

“General” refers to filled-region Minkowski semantics for admitted inputs, not
an exact symbolic Bézier/NURBS representation of every resulting boundary.
Curved results may have a nonempty uncertainty region between the inclusion
bounds. Image-set equality does not assert oriented traversal multiplicity or a
pointwise parameter bijection. The finite event census can still refuse
nonisolated constant parameter fibers; equality of constant image sets is a
separate coverage claim. Exhausted algebraic budgets explicitly refuse.
The filled-region operation does not depend on resolving exact curve loci.

Input curves: degree 1–8, positive weights, finite planar controls within the
existing source limits, exact closed/C0 wires, up to 128 extracted source spans.
Curve bounds: at most 512 leaves and depth 24. Polygon kernel: at most 512 x
stations, 1024 convex cells, 2048 cell pairs and 8192 union input edges; exact
coordinate and arrangement incidence budgets are checked. Multi-span event
certificates admit up to 64 cells per curve and 4096 pair witnesses. Quantified
image coverage admits 64 cells per curve, at most 64 relations per source cell,
a projection degree of 1024 and 256 real critical roots. Budget exhaustion
explicitly refuses. The winding rule is positive/material-left;
construction history and accumulated error from earlier edits are separate.

## Validation

The delivered source and package were qualified with 110 selected Rust tests
(62 certificate, 41 retained Bézier/modeling, 7 bridge tests) and 78 integration
tests in 12 files. Both application and MCP TypeScript checks passed. These are
focused regression suites, not a claim that every repository test was run.

The final WASM produced and verified 15 certificate fixtures and rejected 15
forged witnesses. The separate native Rust checker accepted the same 15
WASM-produced certificates and rejected all 15 forgeries. Five resulting
extrusions had closed, consistently oriented meshes, no degenerate triangles
and no nonmanifold edges. Mesh self-intersections were not checked, and these
checks are not a formal certificate of the 3D mesh.

The built application imported the frame Minkowski result, displayed the inner
hole in both sketch and solid views and emitted no captured browser warnings
or errors. See `browser.png` and `browser-log.json`. New two-source Minkowski
operations are exposed through the Rust/WASM and TypeScript APIs; the existing
general-curve offset workflow uses the new region kernel. This delivery does
not add a dedicated two-source Minkowski UI or a certified 3D solid sum.

Generated, public and dist geometry WASM all have SHA-256
`dc1a3adaac014563cbc85c8624b71e43b7468b05d48d4cac86040809ece00b02`
and 11,671,558 bytes. Dist verification passed for 173 artifacts: 8,249,224 asset
bytes plus 14,699,839 raw WASM bytes. The geometry chunk is 3,952,614 bytes,
within its unchanged 3,970,000-byte limit. The integrated concurrent CAD UI
chunk measured 388,741 bytes, requiring a bounded 390,000-byte limit; the
aggregate asset limit is 8,260,000 bytes with 10,776 bytes of headroom. These
measurements include concurrent workspace changes, not only this feature.

`manifest.json` binds the qualification to source hashes and package identities.
`wasm-proofs.json` retains the requests, witnesses, forgeries, modeling checks
and 57 per-call timings; the slowest call in this small fixture set was about
1.874 seconds, not a general performance bound. `standalone-results.json`
contains the independent checker results. The `*-pre-image.json` artifact is
an earlier 12-fixture intermediate and is not final qualification evidence.
Build and test logs are saved alongside the report. Work remains uncommitted
in the shared dirty workspace; no push was performed for this request.
