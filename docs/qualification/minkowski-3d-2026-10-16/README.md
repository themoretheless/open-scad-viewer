# Certified polyhedral 3D Minkowski sum — 2026-10-16

The original-surface enclosure and rounding-topology follow-up is documented in
[the 2026-10-18 qualification](../certified-geometry-followup-2026-10-18/README.md).
The limits below describe this original delivery.

## Contract

The new Rust operation computes the regularized Minkowski sum of the compact
positive-winding material represented by two closed, consistently oriented
triangle meshes. Nonconvex solids, inner cavities, disconnected components and
their merging are supported. It is not the convex hull of a nonconvex input.
Input binary64 coordinates are treated as exact rationals, including exact
coordinate welding of duplicated seam vertices. No tolerance CSG or floating
point hull is trusted by the certificate checker.

Certificate scope: `represented-positive-filled-polyhedral-minkowski-sum`.
The exact output triangle boundary and source cell census are bound to both
original mesh buffers. `minkowski_3d_verify` and the standalone Rust checker
reconstruct the entire exact construction and compare the claimed boundary;
the witness is an executable, source-bound replay certificate, not a proof
assistant derivation. Exact arithmetic, the algorithms, compiler and runtime
remain trusted. Verification does not use the geometry bridge or mesh CSG.

## Construction and checking

1. Validate finite coordinates, nondegenerate triangles and exactly two
   oppositely oriented incidences per welded source edge. The intended material
   is the closure of the open region with strictly positive oriented winding.
   Reversed isolated outer shells therefore represent no positive material.
2. Split the exact source bounding box by every distinct triangle plane.
   Complete plane arrangements partition space into convex cells. On each open
   cell, triangle winding is constant. An exact generic ray evaluates it;
   edge or coplanar hits cause another rational direction or an explicit
   refusal. Keep all and only the cells with positive winding.
3. An outward closed supporting triangle chain has constant positive
   multiplicity on its convex hull boundary. When all source vertices lie in
   every oriented facet halfspace, this gives a checked convex shortcut.
4. For every retained source-cell pair, construct the exact convex hull of all
   vertex sums. Incremental hull triangles use rational orientation predicates;
   their oriented edge closure and all supporting halfspaces are checked.
   A degenerate unsupported horizon explicitly refuses.
5. A complete arrangement of all pair-sum facet planes partitions the result
   box. Keep every cell whose interior lies in any pair sum. Minkowski addition
   distributes over the complete unions of source cells, so this is the full
   filled result, including holes and component merging.
6. Cancel opposite internal cell faces. Split boundary edges at all exact
   incidence vertices before face-fan triangulation, avoiding T-junctions.
   Check that the final triangle chain has no unmatched oriented edge.
   Boundary contacts need not define a manifold at every point; the certified
   object is the filled polyhedral set and its oriented boundary chain.
7. Return exact rational vertices and exact rational volume. The binary64 output mesh
   has a separately checked, outward-rounded per-coordinate displacement bound.
   `displayCoordinatesExact` is true only when that bound is zero. A nonzero
   mesh rounding bound alone does not prove isotopy or preservation of very small
   geometric features after rounding. Subsequent GPU float32 conversion and
   rasterization are outside this bound.

## Interface

Select two bodies, using Shift for the second, and choose **Сумма Минковского**
in the 3D toolbar or command palette. The operation runs in the cancellable
geometry worker. It creates a new body while keeping both source bodies and
adds one Undo transaction. Errors, cancellation and stale replies do not
publish geometry. The confirmation strip allows downloading
`minkowski-3d-proof.json`, including both original mesh buffers and the certificate.
World coordinates are added directly; translating either operand translates
the resulting sum by the same vector.
Any later document edit clears this transient strip rather than presenting a
stale certificate as a claim about edited geometry.

The result is a polyhedral body with no invented retained B-rep. For curved
B-rep selections the interface explicitly states that the certificate covers
their selected display meshes, not the original curved surfaces. No certified
curved-source approximation bound or exact symbolic NURBS Minkowski boundary is
claimed. Existing mesh-based OpenSCAD Minkowski operations remain separate.

Native API: `brep_core::minkowski3d::{certify,verify}`.
WASM API: `minkowski_3d_certify`, `minkowski_3d_verify`.
TypeScript is transport/UI glue in `geometry/minkowski3d.ts` and `solidMinkowski.ts`.
Standalone checker: pipe `{a,b,certificate}` to
`verify_profile_certificate verify-minkowski-3d` (bounded 2 MB JSON request).

## Resource boundaries

Each source admits at most 1024 vertices, 2048 triangles and 128 distinct facet
planes, with coordinates within 1,000,000 mm. The kernel limits arrangements
to 2048 cells, retained source cells to 256 per operand, convex cell pairs to
512, vertex pairs to 16384 per convex hull, union planes to 192, convex facets
to 512 and hull triangles to 4096. The output admits at most 8192 boundary
incidence vertices before face centers, 32768 exact vertices and 65536 triangles.
Generated exact coordinates have an 8192-bit numerator/denominator limit and
the main construction has a 4,000,000-predicate work budget. Exceeding a budget
or encountering an unsupported degeneracy refuses explicitly. The application
also enforces its existing document coordinate range and worker timeout.

## Qualification

Seven native Rust tests passed. The selected host regression suites passed
147 tests across 15 files, including the existing planar Minkowski/Bézier and
worker/client suites. Three targeted interface tests passed: new sum with one
Undo, canceled/failed work without a commit, and the existing subtraction path.
The remaining 261 cases in that interface file were not run. Application and
MCP TypeScript checks passed. This is focused qualification, not a full
repository test run.

The final raw WASM passed eight certificate fixtures and rejected 32 forged
witnesses (changed source, output boundary, cell census and schema). The
standalone Rust checker independently accepted those eight WASM-produced
certificates and rejected all 32 forgeries. Seven nonempty fixture meshes were
closed, consistently oriented, without degenerate triangles or nonmanifold
edges; the eighth fixture proved empty positive material. The corpus covers
convex boxes, a nonconvex L, a cavity, disconnected and merging components,
slanted faces and a rational-volume tetrahedral sum with nonzero output rounding.
Per-call timings are retained in `wasm-proofs.json`; the slowest call in this
small corpus was about 546 ms, not a general performance guarantee.

In the built application, the visible toolbar action created the nonconvex sum
from two selected source bodies. Undo restored the two sources; Redo restored
the third body. The browser-exported certificate passed the standalone checker
with exact volume `108/1 mm³`. A final reload and repeat checked the final
interface text and certificate export. Captured browser warnings/errors: zero.
See `browser.png`, `browser-history.json`, `browser-exported-proof.json` and
`browser-exported-proof-verified.json`.

Generated, public and dist geometry WASM all have SHA-256
`0aa68af8a0b41bfcb0a6f912d20ae07ec40c5cfff42b6dc4128603c488e5e28c`
and 11,798,459 bytes. Dist verification passed for 175 artifacts: 8,298,207 asset
bytes plus 14,826,740 raw WASM bytes. The packed geometry chunk is 3,995,167 bytes;
its bounded budget is 4,010,000. The final DirectModeler chunk is 391,718 bytes
within 393,000, and the aggregate asset budget is 8,310,000 (11,793-byte margin).
These integrated measurements include the shared workspace's concurrent work.

`manifest.json` records source and package identities, counts and explicit
boundaries. Build/test logs, the complete WASM corpus and standalone refusals
are saved alongside it. `scripts/check-minkowski3d.mts` exercises the raw WASM;
`scripts/check-minkowski3d-certificates.py` orchestrates the separate Rust
checker and performs no geometric computation itself. Work is left uncommitted
in the shared dirty workspace; this request did not include a push.
