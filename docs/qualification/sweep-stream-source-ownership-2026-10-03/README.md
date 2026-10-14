# One source snapshot per progressive stream — 2026-10-03

Progressive surface/miter profile streams and their BRep body streams now clone the numerical request when first advanced. Profiles, path/points, scalar/vector laws, guide/frame data, nested budgets and construction options stay attached to that snapshot across all yields and final cap/topology/certificate queries. AbortSignal and shouldAbort remain live cancellation controls and are not cloned. To change geometry, start a new stream.

Low-level previews are returned as separate deep copies. Editing an accepted preview's sections/patches or diagnostics cannot alter the final geometry, budget, accepted status or certificate history. BRep previews stay separate from the native final result. Native binary64 numerical data is preserved; there is no rounding or geometry approximation in cloning.

Before this fix five actual public-WASM regressions failed: mutated accepted miter preview, changed miter request during refinement, modified surface preview, changed miter request before final cap audits, and changed surface body request before topology construction. After the fix the ownership suite plus existing progressive miter/surface suites passed 56/56. The final six-case focused suite additionally checks surface source mutation during refinement and passes 6/6. Types and scoped whitespace checks passed.

This closes an input/preview ownership prerequisite for construction-certificate provenance. It does not promote full continuousBound, roundingCertified or global geometry/smoothness flags. The same raw native WASM is used as in the multispan-seam slice; the client stream ownership code changed.

The isolated UI source snapshot is `/private/tmp/open-scad-viewer-sweep-stream-2026-10-03`; full repeated wide/narrow UI qualification passed 34/34 scenarios and 302/302 assertions on the snapshot with source/preview isolation. Earlier 34-case/302-check qualification remains preserved separately in `docs/qualification/sweep-miter-lifecycle-2026-10-03/` with its own source/dist hashes.
