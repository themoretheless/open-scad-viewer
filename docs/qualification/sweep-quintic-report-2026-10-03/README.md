# Bounded quintic station reconstruction — 2026-10-03

Native `smooth_station_walls` constructs canonical rational degree-2/degree-5
walls with shared lattice jets on soft stations, including closed wrap, and
independent one-sided jets at sharp stations. Either copy of the closed endpoint
marks the same sharp station. Outward interval control differences bound the
whole wall displacement against its source linear span. Incompatible bases,
source decomposition, lattice ranges, displacement tolerance and work exhaustion
refuse a complete candidate rather than expose partial walls.

Native `section_loft_surfaces` now retains supplied nonlinear walls across all
spans and extracts their actual longitudinal edges. Both operations have JSON
and TypeScript adapters. `smoothCertifiedMiterBody` accepts only unchanged owned
full-bound source evidence and exactly reproduced source sections. It composes
wall errors, verifies unchanged filled caps and cap topology, and reruns actual
smoothness, injectivity and material audits before accepting Solid.

Frozen native snapshot `/private/tmp/open-scad-viewer-sweep-quintic-native-2026-10-03`
passed 1,503 tests across brep-core/nurbs-core/cad-predicates; two measurement
tests were ignored. Its subsequent closed-sharp-endpoint adjustment also passed
the two targeted construction tests. Optimized WASM is
`ad0c428caa4858f1357fa680e0533346d6f1285429af7d0d59d249980f3d4b02`,
10,702,146 bytes, installed after checking the previous artifact identity.
The optimized snapshot passed 27 tests across seven frontend suites, followed
by the two final new tests with explicit material-refusal assertions. Vue and
MCP type checks and scoped whitespace checks passed. These claims qualify this
snapshot and listed source hashes, not all concurrent primary native changes.

The curved canonical fixture has full natural-boundary extraction and native
station G2, eight injective wall charts (292 cells), exact boundary identity,
valid trims and certified caps. Its outward displacement upper is
0.4000000000000013 with 405 construction work units. Global embedding still
refuses four adjacent longitudinal face pairs: [0,1], [2,3], [4,5], [6,7].
Consequently material orientation/nesting are not promoted and Solid remains
false. The separate straight owned-source fixture passes the complete boundary
budget and repeated Solid audit after quintic conversion.

Remaining: prove separation for these curved shared-boundary pairs, qualify
reconstruction in moving-frame/guide/affine and hollow/closed combinations,
wire explicit reconstruction options through Rush and viewport, then repeat
independent STEP and UI matrices on the new artifact and finish scoped
publication/new CI. Previous 33-case STEP and 56-scenario UI evidence belongs
to artifact 851ff22f, not this new build. Goal remains active and unpublished.

Native follow-up: an exact synchronized-monotone-coordinate shared-boundary
certificate now resolves those four curved longitudinal pairs. It requires
matching strictly monotone coordinate controls, V-independent positive row
weights and strict opposite offsets from the identical shared edge; missing
prerequisites refuse. The eight shared-boundary regressions passed, including
damaged synchronization, weights and zero-side controls. The curved fixture now
passes the native full boundary, nesting and outward material orientation audit
in its two construction tests. This follow-up is not present in installed WASM
ad0c428c; its next frozen build is in progress. The refusal above records the
installed artifact's behavior, not the newer native source result.
