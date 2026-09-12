# brep-core operation envelope

`brep-core` stores indexed manifold topology with NURBS curves, parameter
curves, and surfaces. Validation checks incidence and sampled geometry
agreement; it does not claim general solid-geometric certification.

## Solid operations

- `boolean(a, b, "union" | "difference" | "intersection")` supports one
  closed orthogonal planar body per operand. Faces must be axis-aligned. The
  result may be concave, but must have one connected boundary and no enclosed
  cavity. Empty, disconnected, cavity, curved, rotated, and non-manifold cases
  return a typed error instead of silently falling back to a mesh.
- `chamfer(model, edge, size)` supports one convex planar body without
  cavities and one authored convex edge.
- `fillet(model, edge, radius, segments)` has the same convex-planar envelope.
  It creates a manifold B-rep with `segments - 1` planar tangent faces
  (`segments` is 2–32). This is a declared faceted circular approximation, not
  an analytic cylindrical blend. Oversized radii and consumed adjacent faces
  are rejected.

Common error codes are `BREP_UNSUPPORTED_OPERATION`,
`BREP_OPERATION_FAILED`, `BREP_INVALID_OPERATION`, `BREP_INVALID_SELECTION`,
`BREP_INVALID_SIZE`, and `BREP_RESOURCE_LIMIT`.

The geometry bridge exposes these as `brep_nurbs_boolean`,
`brep_nurbs_chamfer`, and `brep_nurbs_fillet`. Solid mode creates authored
B-rep Box primitives. Select two B-rep boxes with Shift and use the B-rep
Union/A − B/Intersection buttons. Switch to Edges and choose Chamfer 3D or
Fillet 3D to run the B-rep edge path. Any mesh-only body continues to use the
separate polygon tools and is labeled as such in the operation card.
