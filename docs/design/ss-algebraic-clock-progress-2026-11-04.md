# Exact algebraic clocks, SS points and restricted edges

`QualifiedSsAlgebraicClock` owns a rational power polynomial and an exact
unique-root isolator. Sturm sequences use BigRational arithmetic throughout;
nonroot endpoints enclose exactly one DISTINCT real root, including repeated
roots. A point isolator requires exact polynomial zero. Zero/constant
polynomials, absent/multiple roots, invalid or noncanonical rational inputs,
nonfinite binary inputs and exhausted operation/degree/coefficient policies
cannot create an owner. Trailing zero coefficients are removed from the stored
mathematical definition. Reverification recomputes the normalized Sturm proof;
historical input-work counts may differ after removing redundant input zeros.

Clock refinement bisects with exact root counts. A midpoint becomes an exact
point only when polynomial evaluation proves it is the selected root; otherwise
it remains merely a subdivision. The root polynomial/isolator remains the
geometry definition. Defaults are degree 64, 100,000 counted operations and
32,768 coefficient bits; allowed ceilings are 64, 1,000,000 and 131,072.
Every refinement query has a fresh explicit allowance and at most 1,024 steps.
Rational input normalization checks are included in input validation; work
counts cover retained rational arithmetic/validation results.

`OriginalSsAlgebraicPoint` binds that selected clock to an ORIGINAL qualified SS
graph. Its entire owned isolator must lie in the graph's parameter domain.
Queries return exact clock bounds plus conservative original UV4/world images.
Outward binary conversion is checked against the rational bounds and never
becomes a new point definition. A rational clock closer than one binary64 ULP
remains distinct. Selector identity requires the literal shared curve and clock
owners; semantic equality of independently created algebraic definitions needs
its own future proof and cannot be inferred from overlapping boxes.

`QualifiedSsAlgebraicPort` keeps the point definition while proving membership
on an incident graph. Same-graph ports retain the exact clock enclosure. Other
regular affine-frame bindings use the complete original UV enclosure, exact
source-clock frame bounds and native graph uniqueness. Ordered original source
roles must agree. Integer phase shifts retain the existing exact periodic seam
checks. Nonlinear blowdown and unsupported collapsed-node aliases remain
refusals. The old binary point API delegates its affine-frame membership through
the same implementation, with its exact degenerate clock interval unchanged.

`OriginalSsAlgebraicEdge` proves endpoint order with rational bounds and retains
both symbolic point definitions. Overlapping endpoint boxes cannot authorize
order. It encloses the original graph over a conservative outer restriction;
neither its world curve nor its UV images are fitted or replaced by poles.
Replaying clocks, points, ports and edges uses the original source proofs.

Qualification covers irrational simple and repeated roots, rational repeated
roots, two distinct exact clocks that round to the same binary64 value, invalid
input and resource refusals, whole original world/UV images, mixed-axis graph
membership, wrong branch/invalid period rejection, and endpoint overlap/order.
The original port, periodic relative-path, root BRep and topology-transfer
regressions are included. The SS facade remains at its 799-line architecture
limit; the new implementations live in separate modules.

These are kernel geometry primitives. They are not yet consumed by the old
binary-only BRep vertex/port/wire admission pipeline. Next work must integrate
exact clocks into its common point representation, preserve original node/pole
aliases, and prove cross-source face-local junctions and winding. Automatic
constraint-polynomial generation for third-face intersections, complete
singular/coincident and periodic/pole arrangements, full finite trim, general
intersecting Boolean/healing, persistent lineage/history, actual bridge/WASM
calls and the wider quad/offset/fillet/G2 audit remain in the full goal.

Frozen qualification: all 17 tests passed. Source hashes and full native/BRep
logs are in `docs/qualification/ss-algebraic-clock-2026-11-04/`. Architecture
remains at 54 pinned oversized files, zero layer violations and 799 facade
lines. This qualifies the primitives and old-path compatibility, not general
algebraic-vertex BRep/trim admission.
