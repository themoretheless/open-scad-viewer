# Source NURBS, parameter loci and rounding topology — 2026-10-18

The trimmed-face, branch-graph and approximation-topology follow-up is described
in [the 2026-10-19 qualification](../trimmed-branches-topology-2026-10-19/README.md).
This file records the earlier delivery and its original limits.

This qualification supersedes the curved-source and coordinate-rounding limits
in [the polyhedral delivery](../minkowski-3d-2026-10-16/README.md).
All geometric construction and verification remain in Rust. TypeScript carries
requests, validates worker packets and presents the result.

## Original surfaces and filled Minkowski enclosures

Scope: `represented-nurbs-filled-minkowski-enclosures`. A source is either an
original B-rep model or an exact binary64 triangle mesh. The UI uses the original
model whenever one is retained; an unsupported model explicitly refuses rather
than substituting its cached display mesh.

The checker reconstructs rational tensor Bernstein restrictions directly from
the original degree 1–8 B-spline surfaces. Positive weights, complete rectangular
UV trims, consistent chart seams and matching subdivisions are checked exactly.
Multiple knot spans are supported. Arbitrary curved, holed or nonrectangular trim
domains are not admitted by this operation. Source authority is the authored,
oriented face images and their positive-winding material, not a general theorem
about all B-rep topology metadata. The result remains a polygonal body, without
an invented exact symbolic NURBS representation of its Minkowski boundary.

For each chart cell, tensor Bernstein coefficients bound the complete rational
surface against its bilinear corner patch. The mixed corner term then bounds
the bilinear patch against either selected diagonal triangulation. The displayed
source tolerance is a conservative Euclidean surface homotopy bound, not a
promise that the final filled-set distance is the same number.

Exact convex tubes enclose the whole surface homotopy. Winding is unchanged
outside these tubes. A complete plane arrangement proves regularized source
inner/outer enclosures; Minkowski monotonicity gives inner and outer bounds on
the true filled sum. Coplanar source cells have zero volumetric sweep. Partition
cells are merged only when exact hull volume equals their disjoint total volume.

A separate convex-coverage check can prove a filled Hausdorff distance through
inflated nominal and inner regions. If this proof fails, the distance is `null`;
the set inclusions remain certified. A returned distance includes the actual
output rounding and conservatively converts the coordinate norm to Euclidean
distance. Surface approximation does not by itself prove preservation of the
original curved solid's topology.

The API is `minkowski_nurbs_certify` / `minkowski_nurbs_verify`. Downloaded proof
version 2 contains both original sources and all enclosure certificates.
`verify_profile_certificate verify-minkowski-3d` detects this scope automatically;
`verify-minkowski-nurbs` also accepts it explicitly. The request limit for these
Minkowski modes is 8 MB.

## Full nonisolated parameter sets

Span certificate version 2 factors both exact intersection equations by their
full common polynomial in Q[s,t], including univariate content. The checker
recomputes the common factor and verifies both quotient identities. Thus the
complete parameter set is the union of that factor's real zero locus inside
the span square and the independently verified finite residual intersection
set. A pair of identically zero equations represents the whole square.

The output includes `algebraicLoci`, their original parameter domains, stationary
spans and `finiteEventsExhaustive`. A nonempty implicit-locus list makes the
latter false. These exact implicit sets are not an ordered graph of smooth real
branches; a common algebraic factor may have only exceptional real points or no
real branch. Self-intersection loci exclude the diagonal in the original global
parameters, including pairs of different knot spans. Constant curves and
nonmonotone retracing are represented explicitly. Existing finite-event version
1 witnesses remain accepted for their original witness kinds.

## Topology through actual output rounding

Both f64 and f32 output now require an embedding proof for the whole linear
homotopy from exact rational vertices to the actual rounded coordinates. Exact
Bernstein signs prevent triangle collapse and orientation reversal. Swept hull
separation prevents disjoint triangles from meeting. Shared-edge and
shared-vertex predicates prevent extra intersections beyond the common abstract
simplex. Consequently the embedded finite triangle complex keeps its PL topology
through rounding. Identity rounding is checked directly. Unsupported contacts,
collapsed thin features and exhausted proof budgets refuse without publishing
geometry. This is a conservative sufficient proof and can reject safe inputs.

The interface requests f32 coordinates, so conversion to the vertex buffer adds
no further coordinate change. GPU rasterization is outside the theorem. The
polyhedral certificate still represents the exact nominal filled set; its
rounded display has a separately proved coordinate/set error and topology.

## Interface and qualification

Select two bodies with Shift and choose **Сумма Минковского**. The surface
tolerance field controls the original-source bound. The result preserves both
sources, creates one undoable body and offers the source-bound certificate for
download. The status distinguishes a proved final distance from certified
inclusions without a proved distance. Document edits clear the transient proof
strip.

Qualification counts, source/package hashes and measured distribution sizes are
recorded in `manifest.json`. Raw WASM corpora, separate Rust checker results,
build/test logs and a browser export are saved alongside this file. These are
focused regression fixtures, not an exhaustive test of every admissible model.
Certificate checking reconstructs exact algorithms; arithmetic, algorithm
correctness, compiler and runtime remain trusted. No proof-assistant derivation
is claimed.

130 selected native Rust tests passed (74 profile-certificate, 41 retained
Bézier-modeling and 15 Minkowski tests). Host regression suites passed 153 tests
across 16 files. Five targeted UI cases passed; the other 261 cases in that file
were skipped. Application and MCP type checks passed. The final raw WASM and
the separate Rust checker accepted 32 fixture certificates, rejected 62 forged
witnesses and rejected two deliberately unsupported/unsafe input cases.

In the final built interface, a rational degree-2 NURBS roof plus a mesh box
produced nominal volume 67.73333333333333 mm³, a source-surface bound below 1 mm
and a proved final Euclidean distance below 1.52 mm. Its downloaded version-2
certificate independently passed the standalone Rust checker. The nonzero f32
rounding proof checked 780 triangle pairs. Undo restored the two original bodies;
Redo restored the third body. The source tolerance was visibly available in
compact mode. Captured browser warnings/errors: zero. See `browser.png` and
`browser-history.json`.

Generated, public and dist geometry WASM have identical SHA-256
`201b1e3334cd0f85f6018b5a135925de3740b6f41a526ff252f2234a24eeb2e3`,
11,902,675 bytes. Verified dist: 175 artifacts, 8,336,153 asset bytes and
14,930,956 raw WASM bytes. The packed geometry chunk is 4,030,433 bytes
(budget 4,040,000); DirectModeler is 392,900 bytes (budget 393,000). The aggregate
asset budget is 8,350,000. Work remains uncommitted in the shared workspace.

The source chart limits are 64 faces, 8192 surface controls per source, subdivision depth
3, 17 shared stations and 512 chart cells. Existing exact arrangement, hull,
bit-size and 4,000,000-predicate work limits also apply. Unproved cases refuse.
