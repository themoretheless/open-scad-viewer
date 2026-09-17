# Native modeling operations

Operations belong to the kernel that owns their geometry. `geometry-ops` shares
coordinate transforms, brush falloff and planar region math. `sketch-core`
solves bounded 2D constraints independently of any surface representation.
The bridge serializes calls; it does not silently convert native operations to meshes.

| Kernel | Added construction | Added editing | Limits |
| --- | --- | --- | --- |
| Polygon | Extrude with holes; full/partial revolve including axis; capped loft; polyline sweep | Selected triangle extrusion, twist/bend/lattice, radial grab, sculpt brushes | Loft needs corresponding planar simple rings; sweep uses transported frames; self intersections not certified |
| NURBS | Rational translation sweep; loft with domain, degree and knot alignment | Control-point twist/bend/lattice, radial grab and sculpt brushes | Sweep keeps profile orientation; loft is piecewise linear in section direction; control deformation is not exact pointwise deformation |
| Subdivision | Quad cage extrude, full revolve, loft and sweep | Cage deformation, radial grab and sculpt brushes | Revolve requires positive radii; no geometric solid certification; smoothing changes the constructed cage shape |
| SDF | Profile extrusion and revolution | Inverse-coordinate twist; sphere/box/capsule add/remove strokes with optional smooth blend | Twist is a field, not a certified distance; bend/lattice explicitly unsupported |
| Sketch | Points, circles, dimensions, coincidence, horizontal/vertical, parallel/perpendicular, equal lengths and tangencies | Bounded numerical constraint solve | 24 points, 12 circles, 96 constraints; convergence not guaranteed; status and residuals returned |

Public TypeScript entry points: `polygonKernel.ts`, `nurbsConstructors.ts`,
`subdivisionKernel.ts`, `sdfKernel.ts`, `sketchKernel.ts` under `src/services`.
Rust implementations live in the corresponding crates. WASM integration tests
are in `tests/nativeModeling.test.ts`.

Compact ModelGraph currently exposes polygon profiles/extrude/revolve/sweep/loft
and NURBS sweep/aligned loft. Editing, native sketch constraints and new SDF and
subdivision constructors are public APIs; they are not yet all language commands
or interactive viewer tools.

## Sculpting

`geometry-ops::sculpt` implements one brush engine shared by polygon meshes,
subdivision cages and NURBS curves/surfaces. Brush kinds: `grab` (translate),
`draw` (displace along the normal), `inflate` (per-vertex normal), `smooth`
(one-ring Laplacian), `flatten` (project onto the average tangent plane) and
`pinch` (pull toward the center). Falloffs: `smooth`, `linear`, `sharp`,
`root`, `sphere`, `constant`. Optional X/Y/Z symmetry mirrors the stroke
across the world planes. Each kernel supplies positions, normals and adjacency
for its own control data; NURBS normals are derived from the control net.

SDF sculpting uses `SdfStroke`: a `sphere`, `box` or `capsule` tool, `add` or
`remove` mode and an optional smooth-blend radius (`smooth_union` /
`smooth_difference`). Strokes compose fields and respect the existing node budget.

TypeScript entry points: `sculptPolygonMesh`, `sculptSubdivision`,
`sculptNurbsCurve`/`sculptNurbsSurface`, `sculptSdf`, `sculptMesh`; interactive
brushes live in the mesh modeler's Sculpt panel. Tests: `tests/sculpting.test.ts`.

## Still missing

General native NURBS/B-rep booleans require surface intersections, UV trimming,
classification, sewing and tolerance handling. Existing polygon booleans do not
satisfy this requirement. NURBS extrude/revolve currently construct surfaces,
not arbitrary capped B-rep solids. Subdivision booleans need an explicit policy
for trimming/remeshing rather than pretending the result is an unchanged cage.
Native SDF sweep/loft, interactive constraints and control/edge selection
remain unfinished. Sculpt brushes deform control data (vertices, cage points,
control points) and are not an adaptive remeshing sculpt. No complete CAD-kernel claim is made.
