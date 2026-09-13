# brep-core operation envelope

`brep-core` stores indexed manifold topology with NURBS curves, parameter
curves, and surfaces. Validation checks incidence and sampled geometry
agreement; it does not claim general solid-geometric certification.

## Solid operations

- `extrude_polygon(profile, z_min, z_max)` and
  `extrude_polygon_with_holes(outer, holes, z_min, z_max)` construct exact
  planar B-reps from simple counter-clockwise XY outlines. Concave outlines
  and multiple disjoint holes are supported together. Hole rings are clockwise
  and become true inner loops on the two cap faces, with shared side-wall edges.
  Self-intersecting, touching, nested, or exterior holes fail closed. Solid mode exposes the no-hole path
  as the authored B-rep Wedge primitive.
- `cylinder(radius, height)`, `frustum(bottom_radius, top_radius, height)`,
  and `tube(outer_radius, inner_radius, height)` construct **exact rational**
  round solids. Four quarter-circle NURBS patches per wall share authored
  arc/line edges; planar caps carry rational UV trims. A tube has two annular
  caps and eight curved wall patches. Radii/height are 0.00001–1000000 mm;
  tube wall thickness is at least 0.00001 mm. Frustums accept one zero radius
  for a real apex cone: one pole vertex and an explicitly collapsed boundary,
  algebraically checked against its constant curve and surface control row.
- `sphere(radius)` uses eight regular rational stereographic patches, including
  regular pole charts. `torus(major_radius, minor_radius)` uses sixteen rational
  tensor patches and requires major radius greater than minor radius.
- `revolve(profile)` constructs an exact full-turn solid from a simple CCW
  `(radius, z)` outline with 3–64 vertices, including concave turning profiles.
  Radii are nonnegative; axis-touching profiles use explicit pole topology. Each profile span creates four
  rational ruled patches; this supports cylindrical/conical stepped parts
  without replacing the authored surfaces with facets. `revolve_angle(profile, angle_degrees)` supports signed partial turns up to
  ±360°, exact circular arcs and planar end caps, including concave profiles.
  Profile holes and general curved-profile revolves remain unsupported.
- `faceted_cylinder` and `faceted_sphere` construct manifold planar-faced
  B-reps. They are explicitly labeled as faceted approximations in Solid mode;
  they do not claim analytic cylindrical or spherical NURBS surfaces.
- `faceted_loft` joins 2–64 strictly convex CCW horizontal sections with
  planar triangulated side faces. `faceted_sweep` transports a strictly convex
  CCW profile along a 2–64 point polyline and triangulates each authored span.
  `faceted_revolve` makes a full-turn planar-faceted B-rep from a closed convex
  `(radius, z)` profile, including radius-zero poles. These are construction
  B-reps with shared manifold topology, not smooth or analytic NURBS claims.
- `boolean(a, b, "union" | "difference" | "intersection" | "xor")` supports one or
  more closed orthogonal planar bodies per operand. Faces must be axis-aligned.
  Union, difference, and intersection additionally support two convex planar
  single-body operands at arbitrary orientation by clipping exact planar
  boundaries against their support half-spaces. A bounded planar boundary
  arrangement also supports rotated non-convex solids, concave individual face
  loops, and trimmed face holes. Concave regions are partitioned without moving
  their boundaries. Both operands use the same support-plane/coplanar-edge
  arrangement, and containment tests the actual trimmed planar regions.
  The path fails on resource limits, ambiguity, and non-manifold output.
  The result may be concave or disconnected; disconnected components are
  represented as separate B-rep bodies and can be used by later booleans.
  A fully enclosed orthogonal difference is represented as an inner shell.
  Orthogonal inner-shell operands remain supported in later booleans. Regularized
  empty results contain no entities and remain valid later operands. Strictly
  separated positive-weight control hulls authorize independent curved bodies
  without intersections. Non-manifold or unresolved cases fail explicitly.
- `planar_trim::boolean` and `prism::extrude` add a finite curved Boolean path:
  parallel extrusions of lines and exact rational circular arcs, including
  holes and separate components. Equal height intervals support all four
  operations; intersection supports any overlapping heights; difference supports
  through cutters; equal footprints support interval union/XOR/difference along
  the extrusion axis. A cutter whose footprint covers the source can remove a
  partial axial interval, leaving zero, one or two separate bodies.
  Curve/curve intersections and winding are analytic binary64 computations,
  with explicit ambiguity and work limits. Output retains source rational
  curve spans and ruled surfaces. This is not a certified general NURBS
  surface/surface Boolean algorithm. `stepped_prism` additionally partitions
  unequal-height footprints into retained planar cells and axial slabs. It
  supports all four operations, blind pockets, enclosed cavity shells and
  cap contacts. Complete opposite cell faces cancel before shared edges are
  sewn; exterior coplanar face pieces may remain adjacent. Serialized results
  are recognized from complete side definitions and independent cap-region
  checks, including the signed roles of outer and inner shells. Bounds are
  8 slabs, 16 profiles, 32 cells, 256 partition queries, 2048 partition
  fragments and the existing 256-face/4096-entity model limits; occupied slabs
  smaller than 1e-5 mm are refused. Noncircular profiles, sphere/torus
  intersections and curved blends remain open.
  `prism_frame` handles arbitrary common axis orientation via rigid coordinates
  and bounded coordinate-roundoff normalization; its adjustment report is an
  estimate, not an exact-arithmetic certificate. No mesh authors this topology.
  Separate exterior components may meet at isolated profile endpoints; they
  retain separate vertex/edge ownership. Hole-boundary contacts are refused.
- `prism::extrude(loops, z_min, z_max)` accepts material-left 2D NURBS loops:
  CCW outer boundaries, CW holes, and nested islands. Active knots split retained
  curves into individual edges so a closed multi-span curve does not collapse
  at low display detail. Geometry-bridge exposes `brep_nurbs_extrude_curves`.
- `chamfer_edges(model, edges, size)` supports one convex planar body without
  cavities and one connected chain of authored convex edges. `chamfer` is the
  single-edge convenience form.
- `fillet_edges(model, edges, radius, segments)` has the same connected
  convex-planar envelope. `fillet` is the single-edge convenience form.
  It creates a manifold B-rep with `segments - 1` planar tangent faces
  (`segments` is 2–32). This is a declared faceted circular approximation, not
  an analytic cylindrical blend. Oversized radii and consumed adjacent faces
  are rejected.

Common error codes are `BREP_UNSUPPORTED_OPERATION`,
`BREP_OPERATION_FAILED`, `BREP_INVALID_OPERATION`, `BREP_INVALID_SELECTION`,
`BREP_INVALID_SIZE`, and `BREP_RESOURCE_LIMIT`.

Every NURBS B-rep serializes `topologyIds` for vertices, edges, loops, faces,
shells, and bodies. IDs are independent of compact array order. Constructors
derive deterministic IDs; transforms preserve them; booleans and edge
operations inherit IDs for geometrically unchanged vertices/edges/faces and
assign deterministic IDs to generated or split entities. Serialized
`topologyIds.lineage` records explicit `persist`, one-to-many `split`, and
many-to-one `merge` parent/child relations for geometric Boolean entities.
Blend records also relate each selected authored edge to its generated
parallel replacement rails. A split remains intentionally ambiguous rather
than selecting one child by proximity. Tessellation publishes
`topologyFaceIds` alongside compact numeric `faceIds`, so display LOD does not
change authored face identity.

The geometry bridge exposes exact round construction as `brep_nurbs_cylinder`,
`brep_nurbs_frustum`, `brep_nurbs_tube`, and `brep_nurbs_revolve`, alongside
`brep_nurbs_extrude_polygon`,
`brep_nurbs_faceted_loft`, `brep_nurbs_faceted_sweep`,
`brep_nurbs_faceted_revolve`,
`brep_nurbs_faceted_cylinder`, `brep_nurbs_faceted_sphere`,
`brep_nurbs_boolean`, `brep_nurbs_chamfer_edges`, and
`brep_nurbs_fillet_edges`. Solid mode creates authored B-rep Box, Wedge,
Cylinder, Frustum, Tube, Cone, Sphere and Torus primitives. Cylinder and Sphere offer an explicit
exact/faceted choice; the faceted option retains planar Boolean compatibility.
New sketch extrusions retain authored B-reps (analytic circles remain rational
cylinders), including their sketch workplanes. Select two supported B-rep bodies
with Shift and use the B-rep
Union/A − B/Intersection/XOR buttons. Switch to Edges and choose Chamfer 3D or
Fillet 3D to run the B-rep edge path; Shift-selects build a connected edge
chain. Full 360° New revolves in Solid use a faceted B-rep by default. The
explicit exact option supports signed partial turns and axis poles. Exact combine mode uses the native Boolean envelope and requires an authored B-rep target. Fillet facet count is configurable from 2–32.
Every authored B-rep body carries a display mesh with topology-owned edge indices; Solid exposes a
1–32 tessellation-detail control and explicit Retessellate action. Any
mesh-only body continues to use the separate polygon tools and is labeled as
such in the operation card.

## Direct edits and analysis

`push_planar_face(model, face, distance)`, `shell_planar(model, openings, thickness)`,
and `split_planar(model, normal, offset)` operate on convex planar bodies through
support half-spaces and native Booleans. They preserve authored boundaries and
IDs where geometric correspondence supports inheritance. Curved, concave,
consumed and empty cases are explicit errors; curved shell/offset remains open.
Solid routes its Push/Pull, Shell and Split controls through these operations for
B-rep bodies. Body dragging preserves the B-rep and its IDs. ModelGraph affine
transforms retain authored carriers and IDs, reverse shell face uses for
reflections, and reject singular or numerically unresolved matrices.

`analysis::mass_properties` integrates authored NURBS surfaces and rational trims
with Green's and divergence theorems. It returns surface area, signed volume,
centroid, centroidal inertia, conservative control-hull bounds, work count and
quadrature error estimates. Results are independent of the display mesh. The
status is `converged_estimate`, with `solidGeometryStatus: not_certified`;
quadrature convergence is not a geometric validity/completeness certificate.
Solid exposes this through **B-rep properties**. Explicit budgets and convergence
failures refuse the estimate.

Geometry-only intersection APIs are documented in
[`nurbs-intersection-queries.md`](../../docs/design/nurbs-intersection-queries.md).
They retain parameter traces and unresolved regions without authorizing Boolean
topology decisions.

## ModelGraph / MCP

The `modelgraph/nurbs-1` schema exposes `brep_cylinder`, `brep_frustum`,
`brep_tube`, `brep_sphere`, `brep_torus`, `brep_extrude`, `brep_revolve`, `brep_boolean`, `brep_chamfer`,
`brep_fillet`, and `brep_tessellate`. Text calls support the same construction
path, with Boolean methods `brep_union`, `brep_subtract`, `brep_intersection`.
The normal NURBS MCP compile/build/export tools use these nodes too.

```text
// @modelgraph-text/1
show brep_tube(10mm, 7mm, 20mm).brep_tessellate(16)
```

Tessellation schedules each shared edge once, including straight edges shared
by planar caps and curved patches. Trimmed patches refine only interior edges,
preserving the sampled authored boundaries. Shared vertex/edge indices come from
topology ownership; nearby unrelated components are not proximity-welded. Each
face boundary is checked against its oriented authored coedge schedule. Empty
models produce empty meshes with `closed:false` and no fabricated shell.
Rectangular patches use two triangles per cell; the whole body remains bounded
by 20000 display triangles and segments 1–32. Higher detail may exceed this
budget for complex revolves and fails explicitly. Face IDs are independent of
this display detail. Mesh volume converges with detail; it is not an exact
analytic volume certificate.

General curved Boolean intersections, analytic blends, curved shell/offset
operations, STEP interchange, and geometric solid certification are still
open work. Exact primitives do not imply support for those operations.

## Native full-model transactions

`transactions::ModelSnapshot` owns an immutable complete Model, including
TopologyIds and lineage. Construction and edited candidates run the existing
Model::validate. `ModelStore` reuses the shared revision/history engine for
checkout, prepared edits, final publication checks and Undo/Redo. Operations
can replace the complete model, including topology-changing Boolean results;
identity tables are no longer captured separately from the editable state.

The integration test subtracts a through-hole from a cuboid, checks changed face
counts/identity tables, restores both through Undo/Redo, and rejects stale edits
and corrupted identities. The validator's sampled/not-certified limitations
still apply. These are native APIs, not yet the production UI/WASM transaction
backend, and they do not serialize pending transactions or source recipes.

## Full-model snapshot encoding

ModelSnapshot::to_json/from_json use a version-1 `brep-model-snapshot` envelope
and the existing model codec. Decoding requires explicit topology identity
tables and re-runs kernel validation. Unknown envelope fields/versions, duplicate
JSON keys, corrupt identities and invalid geometry are rejected. Encoded size
is limited to 8 MiB (checked before parsing; encoding checks after serialization).
The shared codec's parser nesting/item bounds also apply.

This stores current full geometry, identity and lineage only. In-memory store
identity, pending transactions, Undo/Redo and cad-predicates construction DAGs
are not encoded. Restored models begin a new store session. Four transaction
tests pass in debug/release, including deterministic Boolean-model roundtrip,
continued edits/Undo, corrupt payloads and oversize refusal.

## Current model plus history archive

`history_to_json` / `history_from_json` encode a version-1 `brep-model-history`
archive containing current, Undo and Redo snapshots. The combined history is
limited to 32 entries and the archive to 32 MiB; individual snapshot limits and
strict duplicate-key rejection still apply. Loading validates every snapshot
before returning a fresh revision-zero store. It does not restore in-process
store identity or pending transactions, so old prepared edits remain foreign.

The shared revision engine rejects mixed admission policies during typed history
restoration. Five kernel transaction tests pass in debug/release; eight topology
tests pass in debug and topology module-only Clippy passes. History archives do
not yet contain cad-predicates construction DAGs or connect to UI project saves.
