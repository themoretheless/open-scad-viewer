# brep-core operation envelope

`brep-core` stores indexed manifold topology with NURBS curves, parameter
curves, and surfaces. Validation checks incidence and sampled geometry
agreement; it does not claim general solid-geometric certification.

## Solid operations

- `extrude_polygon(profile, z_min, z_max)` constructs an exact planar B-rep
  from a strictly convex counter-clockwise XY profile. Solid mode exposes this
  path as the authored B-rep Wedge primitive. Concave, collinear, and invalid
  profiles fail closed.
- `faceted_cylinder` and `faceted_sphere` construct manifold planar-faced
  B-reps. They are explicitly labeled as faceted approximations in Solid mode;
  they do not claim analytic cylindrical or spherical NURBS surfaces.
- `faceted_loft` joins 2–64 strictly convex CCW horizontal sections with
  planar triangulated side faces. `faceted_sweep` transports a strictly convex
  CCW profile along a 2–64 point polyline and triangulates each authored span.
  `faceted_revolve` makes a full-turn planar-faceted B-rep from a closed convex
  `(radius, z)` profile, including radius-zero poles. These are construction
  B-reps with shared manifold topology, not smooth or analytic NURBS claims.
- `boolean(a, b, "union" | "difference" | "intersection")` supports one or
  more closed orthogonal planar bodies per operand. Faces must be axis-aligned.
  Union, difference, and intersection additionally support two convex planar
  single-body operands at arbitrary orientation by clipping exact planar
  boundaries against their support half-spaces.
  The result may be concave or disconnected; disconnected components are
  represented as separate B-rep bodies and can be used by later booleans.
  A fully enclosed orthogonal difference is represented as an inner shell.
  Orthogonal inner-shell operands remain supported in later booleans. Empty,
  curved, unsupported non-convex rotated, and non-manifold cases fail closed;
  no operation silently falls back to a mesh.
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

The geometry bridge exposes these as `brep_nurbs_extrude_polygon`,
`brep_nurbs_faceted_loft`, `brep_nurbs_faceted_sweep`,
`brep_nurbs_faceted_revolve`,
`brep_nurbs_faceted_cylinder`, `brep_nurbs_faceted_sphere`,
`brep_nurbs_boolean`, `brep_nurbs_chamfer_edges`, and
`brep_nurbs_fillet_edges`. Solid mode creates authored B-rep Box, Wedge,
faceted Cylinder, and faceted Sphere primitives. Select two B-rep bodies with Shift and use the B-rep
Union/A − B/Intersection buttons. Switch to Edges and choose Chamfer 3D or
Fillet 3D to run the B-rep edge path; Shift-selects build a connected edge
chain. Full 360° New revolves in Solid are authored through the faceted B-rep
revolve path and labeled accordingly; partial revolves and combine modes keep
their existing polygon path. Fillet facet count is configurable from 2–32.
Every authored B-rep body carries a welded display mesh; Solid exposes a
1–32 tessellation-detail control and explicit Retessellate action. Any
mesh-only body continues to use the separate polygon tools and is labeled as
such in the operation card.
