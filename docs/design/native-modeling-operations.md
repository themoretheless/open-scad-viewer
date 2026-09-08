# Native modeling operations

Operations belong to the kernel that owns their geometry. `geometry-ops` shares
coordinate transforms, brush falloff and planar region math. `sketch-kernel`
solves bounded 2D constraints independently of any surface representation.
The bridge serializes calls; it does not silently convert native operations to meshes.

| Kernel | Added construction | Added editing | Limits |
| --- | --- | --- | --- |
| Polygon | Extrude with holes; full/partial revolve including axis; capped loft; polyline sweep | Selected triangle extrusion, twist/bend/lattice, radial grab | Loft needs corresponding planar simple rings; sweep uses transported frames; self intersections not certified |
| NURBS | Rational translation sweep; loft with domain, degree and knot alignment | Control-point twist/bend/lattice and radial grab | Sweep keeps profile orientation; loft is piecewise linear in section direction; control deformation is not exact pointwise deformation |
| Subdivision | Quad cage extrude, full revolve, loft and sweep | Cage deformation and radial grab | Revolve requires positive radii; no geometric solid certification; smoothing changes the constructed cage shape |
| SDF | Profile extrusion and revolution | Inverse-coordinate twist; spherical add/remove brush | Twist is a field, not a certified distance; bend/lattice explicitly unsupported |
| Sketch | Points, circles, dimensions, coincidence, horizontal/vertical, parallel/perpendicular, equal lengths and tangencies | Bounded numerical constraint solve | 24 points, 12 circles, 96 constraints; convergence not guaranteed; status and residuals returned |

Public TypeScript entry points: `polygonKernel.ts`, `nurbsConstructors.ts`,
`subdivisionKernel.ts`, `sdfKernel.ts`, `sketchKernel.ts` under `src/services`.
Rust implementations live in the corresponding crates. WASM integration tests
are in `tests/nativeModeling.test.ts`.

Compact ModelGraph currently exposes polygon profiles/extrude/revolve/sweep/loft
and NURBS sweep/aligned loft. Editing, native sketch constraints and new SDF and
subdivision constructors are public APIs; they are not yet all language commands
or interactive viewer tools.

## Still missing

General native NURBS/B-rep booleans require surface intersections, UV trimming,
classification, sewing and tolerance handling. Existing polygon booleans do not
satisfy this requirement. NURBS extrude/revolve currently construct surfaces,
not arbitrary capped B-rep solids. Subdivision booleans need an explicit policy
for trimming/remeshing rather than pretending the result is an unchanged cage.
Native SDF sweep/loft, advanced sculpt brushes, interactive constraints and
control/edge selection remain unfinished. No complete CAD-kernel claim is made.
