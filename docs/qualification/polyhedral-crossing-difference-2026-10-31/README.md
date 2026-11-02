# Exact crossing polyhedral subtraction

Native Rust API: `minkowski3d::difference::{certify, verify}`.

This implements the regularized positive-winding material of the container
minus the positive-winding material of the tool. Both inputs are represented
binary64 triangle meshes; coordinates are converted to exact rational numbers.
No claim is made that an approximating mesh equals its original NURBS B-rep.

All planes of both sources partition the container bounding box. Each convex
full-dimensional cell is classified by exact source winding at its interior
centroid. No source triangle intersects a cell interior because its supporting
plane participates in the partition. Consequently the classification holds
throughout the cell. Keeping container-positive/tool-nonpositive cells computes
the regularized difference, including crossing tools and nonconvex containers
and tools. Internal shared faces cancel. Remaining tool-plane faces provide
caps; edge subdivision is synchronized globally before triangulation.

The output boundary must have two incident triangles on every edge and a single
cycle as every vertex link. Singular touching components are refused. Empty
material and disconnected manifold components are supported. The existing exact
coordinate-rounding homotopy checker is required for the display mesh. The exact
boundary and exact signed volume remain available separately.

The certificate binds both original sources, every rational output coordinate,
all oriented triangles and the retained cell count. Verification reconstructs
the complete arrangement and classification from the sources and compares the
certificate; it shares the Rust geometry implementation with the producer. It
is replayable verification, not a separately implemented solver or theorem
prover proof.

Bounds inherited from the existing kernel include 1024 source vertices, 2048
triangles and 128 distinct planes per source; 2048 arrangement cells, 512 facets
per convex cell, four million work units and 8192-bit coordinate guards. Exceeding
a bound or an unproved rounding homotopy returns an error, never an approximate
certified result.

## Remaining requested scope

This API does not construct a new Model B-rep, clip curved source NURBS faces,
select/delete arbitrary algebraic atlas cells, or construct curved caps across
those selected cells. The earlier enclosed curved subtraction API retains its
convex-container restriction. There is no UI/MCP/WASM/dist integration in this
change. A full solution of the user's request therefore remains open.

## Validation

See result.json and native-tests.log for the final native test result. Focused
regressions cover crossing caps, a nonconvex L-shaped container, a nonconvex tool,
a cavity, disjoint/touching tools, complete removal, split components, source and
boundary forgeries, and rejected singular vertex links. The source ledger is a
scoped file snapshot, not a frozen manifest of the entire shared workspace.
