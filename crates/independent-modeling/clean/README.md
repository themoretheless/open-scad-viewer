# modeling-clean

Independent standard-library-only Rust geometry. This crate does not depend on
the host CAD crates, application artifacts, native libraries or a geometry service.
All angles are radians. Length units are consistent caller-selected units.

```rust
use modeling_clean::{curve::Curve, surface::Surface, mesh::Mesh};
let circle = Curve::circle([0.0; 3], 2.0)?;
let cylinder_side = Surface::extrude(&circle, [0.0, 0.0, 5.0])?;
let point = cylinder_side.evaluate(0.5, 0.25)?;
assert!((point[2] - 2.5).abs() < 1e-12);
let display: Mesh = cylinder_side.tessellate([8, 64])?;
// A side patch is not a closed solid. Its seam remains indexed separately.
assert!(!display.topology().is_closed_manifold());
# Ok::<(), modeling_clean::Error>(())
```

Curve definitions use positive rational weights, clamped knot vectors and
degrees 1–16. Conics are rational quadratic definitions; helices are explicitly
piecewise-linear approximations. Knot insertion, split and trim retain parameter
coordinates. Derivatives select the right branch at interior knots and the left
branch at the upper endpoint; this does not assert two-sided continuity.

Surface lofts require matching profile bases and are piecewise ruled in the
section direction. Revolution is rational, with at most one signed full turn.
Sweep transports section frames along a polyline; zero edges and reversals fail.
Neither sweep nor loft proves absence of self-intersections.

Meshes include box, cylinder, sphere, torus, polygon extrusion and section loft.
They are faceted definitions, not exact curved B-reps. Polygon extrusion accepts
simple polygons without holes; touching/crossing/collinear corners are refused.
Topology checks indexed incidence, orientation and connected vertex fans; it
does not certify geometric embedding. Uniform tessellation has no certified
deviation bound. Surface area, signed volume and center of mass are floating
point sums, not exact-arithmetic certificates.

`boolean::boxes` supports union/intersection/difference/XOR of axis-aligned box
sets. Reusable occupied cells are exposed by `box_cells`. Limits: 64 input boxes
combined, 64 retained result cells, 262144 coordinate cells. Nonmanifold contact
results fail. This is not a general mesh or NURBS Boolean. `shell_box` implements
only inward axis-aligned box shells with at most one selected open side.

OBJ input intentionally accepts only plain vertex and triangular face records;
OBJ and ASCII STL exports are provided. No STEP/IGES or proprietary file reader.

Development process and unimplemented requirements: [SPEC](../SPEC.md).
The baseline manifest is recorded before creating the informed package.
