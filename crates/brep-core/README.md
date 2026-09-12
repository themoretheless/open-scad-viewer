# brep-core operation envelope

`brep-core` stores indexed manifold topology with NURBS curves, parameter
curves, and surfaces. Validation checks incidence and sampled geometry
agreement; it does not claim general solid-geometric certification.

## Solid operations

- `extrude_polygon(profile, z_min, z_max)` constructs an exact planar B-rep
  from a strictly convex counter-clockwise XY profile. Solid mode exposes this
  path as the authored B-rep Wedge primitive. Concave, collinear, and invalid
  profiles fail closed.
- `boolean(a, b, "union" | "difference" | "intersection")` supports one or
  more closed orthogonal planar bodies per operand. Faces must be axis-aligned.
  The result may be concave or disconnected; disconnected components are
  represented as separate B-rep bodies and can be used by later booleans.
  Empty, cavity, curved, rotated, and non-manifold cases return a typed error
  instead of silently falling back to a mesh.
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
`brep_nurbs_boolean`, `brep_nurbs_chamfer_edges`, and
`brep_nurbs_fillet_edges`. Solid mode creates authored B-rep Box and Wedge
primitives. Select two B-rep bodies with Shift and use the B-rep
Union/A − B/Intersection buttons. Switch to Edges and choose Chamfer 3D or
Fillet 3D to run the B-rep edge path; Shift-selects build a connected edge
chain. Fillet facet count is configurable from 2–32. Every authored B-rep body carries a welded display mesh; Solid exposes a
1–32 tessellation-detail control and explicit Retessellate action. Any
mesh-only body continues to use the separate polygon tools and is labeled as
such in the operation card.
