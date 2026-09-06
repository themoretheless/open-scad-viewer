# CAD operation extension

Implemented in ModelGraph/1, using the existing Manifold execution route:

- `hull`: convex envelope of profiles or solids.
- `mirror`: reflection through an origin plane, nonzero normal.
- `affine`: invertible 3x4 transform; includes shear and combined transforms.
- `offset`: signed rounded profile offset using engine default corner discretization.
- `projection`: XY silhouette of a solid.
- `section`: XY profile at a specified Z coordinate.
- `advanced_extrude`: positive-height extrusion with twist and positive independent top scales, up to 64 slices.
- `cone`: cylinder frustum, including one zero end radius.
- `torus`: ring torus with major radius greater than minor radius.
- `linear_pattern`: up to 256 translated copies, unioned.
- `circular_pattern`: up to 256 copies rotated about global Z, unioned.

Existing union/intersection/difference, translation/rotation/scaling, extrusion/revolution and bounded loft remain available. Every scalar parameter in the new nodes participates in the existing functional expression and unit system. Profile/solid checks prevent accidentally exporting planar results as solids.

`modelgraph_modify` is registered for both stdio and HTTP MCP. It measures current geometry and returns complete documents plus actual geometry analyses:

- `resize`: target XYZ sizes with origin/minimum/center anchor. Uses an affine transform.
- `split`: axis-aligned plane strictly inside measured bounds; returns negative and positive parts without kerf.

Resize transforms and split cutters are calculated at current parameter values, not persistent geometric constraints. Re-run modification after parameter changes. Assembly roots are rejected. Existing graph-size and geometry runtime limits still apply. All returned documents work with the common report/export tools.

## Verification and remaining scope

Production-worker tests measure volumes, bounds and closed/nonmanifold topology for every new node family and both modifications. MCP transport tests call modification and check returned parts. Generated language schema and prompt expose the new nodes automatically.

This is an initial extension, **not completion of the requested 100 operations**. General BRep fillets, chamfers, shells, face editing and exact surface intersections are not implemented here. Symmetric difference was tested but excluded because nested boolean provenance can exceed the existing worker protocol's identifier limits. Deep transform trees can hit the same existing limitation. This extension does not modify the frozen execution protocol or replace it with an unbounded in-process fallback.
