# OpenSCAD native migration

Source audit: 2026-10-01. This is a remaining-work inventory, not a completion certificate.

## Ownership

| Responsibility | Owner | Current boundary |
| --- | --- | --- |
| Evaluation, scopes, argument binding and language normalization | `openscad-core` | Stable primitive and transform plans are native; execution coverage still grows |
| Circle and sweep fragment counts | `geometry-ops::fragment_resolution` | Shared numerical rules; OpenSCAD facade preserves the public API |
| Legacy rounding | `openscad-core` | Language-specific rounding rules |
| Stable degree trigonometry | `openscad-core::degree_math` | One implementation for native built-ins and production TS through ABI 35 |
| Aggregate resize bounds and axis scales | `geometry-ops::resize` | Shared geometry rules; OpenSCAD binds values and reports language diagnostics |
| Normalized face polygon packing | `geometry-ops::polygon_mesh` | Reversed triangle fans and float32 range admission; topology validation remains in the mesh kernel |
| Extrusion subdivision | `geometry-ops::extrude_slices` | Shared by profile execution and the OpenSCAD facade |
| Neutral, normalized geometry programs | `geometry-ops::{solid_program,profile_program}` | Primitive, transform, Boolean and extrusion nodes; no language dependency |
| Mesh construction and execution | `polygon-core::solid::program` | Consumes the typed program; bounded retained triangles |
| Source and value transport | `languages-bridge` | Produces the program; does not construct meshes |
| Geometry transport | `geometry-bridge` | Executes the program; does not compile source |
| Browser integration | `openscadNativeGeometry.ts` | Opt-in API; production parser still evaluates in TS |

## Stable profile: prerequisites

Stable sin/cos/tan and inverse degree functions use `degree_math` in both
evaluators. ABI 35 accepts up to 1,024 calls, validates names and arity, and uses
explicit tokens for signed zero and nonfinite results. TS retains argument binding
and type diagnostics. WASM tests cover common exact angles, inverse results,
negative zero, tangent poles, invalid domains and batch admission. Production
sin/cos built-ins share a bounded 256-angle cache and batch paired computations
when observed calls alternate across angles. Sin-only sequences request one value.
The shared host adapter chunks larger explicit batches at 1,024 calls. A warm Node
microbenchmark records 16.65 ms for 4,096 individual values, 10.76 ms through the
production pair adapter and 2.64 ms through explicit batches. Unique sin-only calls
still have adapter/cache overhead (18.51 ms versus 16.45 ms for raw ABI requests).
See `docs/qualification/degree-math-batching-2026-10-03.json`; these timings exclude
initialization and do not qualify whole-model or browser performance. Batching
whole expression evaluation remains unfinished.

Production polyhedron plans include Rust-packed vertices and indices from ABI 28.
The parser passes these buffers to the mesh kernel and retains topology diagnostics.
Packing preserves authored face order, reverses fan winding and checks float32
range as the previous TS implementation did. This preserves the existing fan
behavior for concave faces. Native polyhedron recording binds arguments once,
reuses the indexed face expansion and packing, and emits a neutral mesh node.
Its closed-solid admission policy turns cleanup failures, empty meshes and open
shells into empty geometry plus a topology diagnostic, matching the production
adapter. WASM tests compare native and production volumes and warnings for
valid tetrahedra, the triangles alias, skipped indices, malformed referenced
points, float32 overflow and repeated invalid shells.

Solid programs accept the existing neutral `Triangles` format through `Mesh` nodes.
Admission checks complete position/index triples, finite referenced geometry,
index bounds and aggregate authored mesh budgets of 300,000 vertices and 100,000
triangles, including meshes nested inside projection programs. Execution uses the
same mesh cleaning path as the production mesh adapter and the existing retained
triangle budget. Native tests cover transport, invalid buffers, extraction of
unrelated arena data and translated closed-mesh volume. Mesh nodes default to
strict kernel error handling; the optional emptyOnFailure policy requires a closed
solid and records a diagnostic instead of aborting sibling roots. These diagnostics
propagate through projection and extrusion. The integration keeps compiler
messages and appends distinct executor warnings in first occurrence order.

The neutral solid program includes `RevolveProfile` for rotation of a normalized profile by a signed angle and
explicit segment count. Revolution execution preserves holes, partial caps and
axis poles through `polygon-core::solid::modeling::revolve_rings`.
The node optionally stores a fragment policy; execution resolves the radius and
segment count after profile evaluation and reports fragment warnings. Native
recording of `rotate_extrude` uses this node. Shared profile-side classification
reflects negative-X profiles before execution and rotates their meshes by 180 degrees.
Host tests compare production volume for full and signed partial sweeps, negative
profiles, axis poles, holes, zero angles and empty circles, preserving child warnings.
It also includes extrusion of evaluated rings with height, slices, twist, scale
and center. The executor reuses `polygon-core::solid::modeling`;
the transport validates finite inputs and aggregate profile budgets. The evaluator
records square, circle and polygon profiles, planar Booleans and affine transforms, then emits profile
extrusion with deferred subdivision.
Each extrusion extracts its reachable profile subgraph, remapping references while
preserving shared inputs and operand order. Unrelated arena profiles are not copied
or counted against the extracted program's geometry budget.
Solid programs also expose `from_roots`. Both program types use one shared
dependency compactor, preserving shared operands and duplicate root order.
Selected geometry is validated after extraction; arena reference and graph budgets
are checked before any indexing. This supplies the solid subgraph extraction
used by projection nodes. A projection embeds one selected solid root and resolves
its silhouette or Z=0 section through `mesh-section`. One aggregate admission pass
limits alternating solid/profile programs to 25,000 nodes, 100,000 edges,
100,000 authored profile points and 32 nesting levels. Nested executor warnings
propagate through profiles to the outer solid report.
WASM host tests compare production and native volumes for silhouettes, zero-plane
sections, separated operands, empty solids and repeated projection/extrusion.
They also verify that a nested revolution's fragment-clamp warning and reduction
status survive projection and the outer extrusion.
Host tests compare production and native
volume for square extrusion, Boolean holes, XY transforms, twist and nonuniform
scale, and verify executor slice-clamp warnings and reduction status. Polygon tests
cover implicit contours, explicit paths, even-odd holes with equal winding,
out-of-bounds indices, malformed points, collinear contours and non-finite coordinates.

`eval.rs` records stable cubes, spheres and cylinders through shared parameter plans, including empty
results, user modules and union/intersection/difference. Stable translate, scale and
mirror, rotate and affine multmatrix record matrices from the shared transform plans; singular and non-finite
transforms produce explicit empty geometry with the existing warnings. Other stable
geometry modules still refuse recording, including asset imports, text and surface.
Minkowski recording emits shared profile or solid nodes. Planar products normalize
profile anchors, extrude, combine and project in Rust; the production CAD adapter
uses the same helper. WASM host tests compare production and native volume for
2D products with translated operands, circles and empty inputs, and for 3D cubes.
Execution preserves the existing convex-input and vertex-budget checks;
general nonconvex Minkowski remains unsupported.
Production resize sends the bounds of nonempty children to the Rust resize plan.
Rust aggregates group extents and resolves explicit and automatic axis scales;
TS retains child selection, diagnostics and application of the returned scales.
Host tests cover separated bounds, invalid axes and the nullable extents ABI.
Neutral profile and solid programs now also have unary deferred `Resize` nodes.
Their executors resolve bounds after geometry evaluation, scale about the origin,
retain the existing output budgets and propagate zero-width-axis diagnostics
through projection and extrusion. Native tests cover codec round trips, input
compaction, aspect axes and translated meshes. OpenSCAD recording binds newsize
and auto once, normalizes numeric targets and strict boolean automatic axes, and
records resize after the group union. The invalid-newsize diagnostic is deferred
until a nonempty geometry reaches the executor. WASM host tests compare native
and production volumes for 2D/3D resize, separated children, translated geometry,
automatic axes, ignored nonpositive targets and invalid newsize on empty/nonempty
inputs. Production evaluation still runs in TS; native recording remains opt-in.
Hull recording now
emits neutral 2D or 3D nodes using the existing hull kernels. WASM host tests
compare production volumes for cubes, spheres, extruded rectangles and circles,
including one operand and an empty group. Native projective
multmatrix execution preserves the full matrix and rejects triangles crossing W=0.
The stable production TS evaluator sends full 3D matrices through the CAD adapter
to the same typed projective mesh path. It still checks 2D affine classification;
projective profile execution remains unsupported. Unrepresentable matrices produce
empty geometry and a warning. Host tests compare native and production volume
for a finite projective chart and verify production refusal when triangles cross W=0.
Cylinder recording resolves
bound/global `$fn/$fa/$fs` through the shared fragment rules; descriptor-only
stable `segments` still returns zero. The neutral program has an explicit `Empty` node, so
empty operands retain their authored position in Booleans. The executor removes
empty root meshes from its returned geometry, after all consumers have run.

Port normalization before enabling recording:

1. `openScadStablePrimitiveSemantics.ts`: bounded transport preflight, warning
   presentation and frozen public objects. Resource limit admission and
   convexity normalization use `indexed_primitive::indexed_policy` through
   operation 30, preserving ordered field errors. Polygon conversion,
   implicit/explicit outlines and ordered stop events use
   `indexed_primitive::expand_polygon` through operation 29; 864 point/path/limit
   combinations match the frozen reference. Both indexed primitives share the
   typed index expansion routine. Polyhedron point conversion, unsigned indices,
   bounds skipping, partial-face expansion and stop events use
   `indexed_primitive::expand_faces` through operation 28; 720 source/alias/index
   limit combinations match the frozen reference. The adapter bounds face
   uploads at the first index-limit event. Owner:
   `openscad-core`, with typed plans independent of mesh construction.
   Cube/square size conversion and empty decisions already use
   `primitive_plan::box_plan` through language operation 17; warning presentation
   remains in TS. A frozen TS reference checks 320 input/context combinations.
   Sphere/circle/cylinder use the same typed module through operation 18 for
   radius precedence, empty decisions and warning events. TS supplies authored
   numeric values and presents those events.
2. `openScadStableTransformSemantics.ts`: authored-value transport, warning
   presentation and frozen public objects. Scalar rotation uses
   `transform_plan::scalar_rotation` through operation 27, preserving default
   axes and warning-field priority; 120 authored angle/axis combinations match
   the frozen reference. Translation, scale and reflection share
   `transform_plan::vector_transform` through operation 26, returning argument
   validity, range events, matrix and classification in one request. Authored matrices
   use `transform_plan::authored_matrix` through operation 25, retaining identity
   cells, bounded first-four rows/columns and homogeneous division, including
   nonfinite W. Euler vector normalization visits Z/Y/X in Rust and
   returns the matrix in the same request (operation 24); 743 argument vectors
   and warning sequences match the frozen reference. Shared vec2/vec3
   conversion uses `transform_plan::vector_with_default` through operation 23;
   2,058 partial-vector plans and warning sequences match the frozen reference.
   Axis-angle construction and scaled,
   compensated axis normalization use `transform_plan::axis_angle_matrix`
   through operation 22; 42 angle/axis combinations match the frozen reference,
   including zero, NaN, infinity and very small/large finite axes.
   Stable reflection uses
   `transform_plan::mirror_matrix` through operation 21, preserving zero-normal,
   overflow and underflow behavior; 64 extreme-normal combinations match the
   frozen reference. Stable Euler matrices and
   exact quadrant trigonometry use `transform_plan::euler_matrix` through
   operation 20; 54 quadrant combinations match the frozen TS reference.
   Singularity, affine classification, full XY matrix
   extraction and nonfinite child removal already use `transform_plan::analyze`
   through language operation 19. The determinant preserves the former
   elimination order and exact zero comparison. Owner: `openscad-core`
   for language rules; reusable affine arithmetic belongs to `osv-math`.
   Preserve column-major public plans and authored row-major `multmatrix`.
   Legacy XY extraction now uses `transform_plan::authored_affine2d` through
   operation 25, preserving finite-cell defaults, homogeneous division and W=0
   refusal. TS retains only bounded input adaptation and result decoding.
   Offset parameter precedence and joins use `offset_plan::resolve` through
   operation 33. Numeric `r` wins over `delta`, including nonfinite values;
   strict boolean chamfer applies only to delta mode. Geometry recording for
   Native offset recording emits the neutral profile `Offset` node with shared
   typed joins, codec support, reference remapping and the production
   `rings::offset_join` executor. Host tests compare expanded and contracted
   extrusion volumes, chamfer, holes, radius precedence, empty profiles and
   round-fragment clamp warnings. Delta mode skips fragment resolution.
   Stable projective rows cannot be discarded as in the viewer-subset path.
   Viewer axis-angle matrices now use operation 22 and
   `transform_plan::viewer_axis_angle_matrix`. Both profiles share Rodrigues
   assembly; viewer keeps ordinary trigonometry and refuses the zero axis.
   Host tests compare 28 matrices with the former TS formula, including axes
   scaled by 1e-200 and 1e200, and verify the zero-axis result.
3. `openScadStableGeometrySemantics.ts`: linear extrusion height, scale, twist and
   explicit slice normalization now use `extrusion_plan::parameters` through
   language operation 31. The same request now resolves automatic or explicit
   slices, applies the limit and returns the interior subdivision count. Host
   tests preserve warning order, defaults, empty height and explicit slices.
   Operation 12 remains available for standalone subdivision queries.
   Revolution angle normalization, profile-side classification and empty decisions
   use `extrusion_plan::revolution_parameters` through operation 32. Bounds are
   validated at the bridge; TS forwards fragment warnings and adapts the result.
   Child index selection now shares `children_selection::select` between native
   evaluation and language operation 34. Host tests preserve ordered warnings,
   truncation, negative-zero conversion, duplicates and indices above 32 bits.
   Range materialization shares `children_selection::expand_range` through the
   same operation. Host tests cover descending and fractional ranges, limits
   and stalled binary64 accumulation. Absent arguments still expand the already
   evaluated host child list in each adapter.
   Keep existing native slices/resize rules; remove the remaining
   TS numerical implementations after parity tests through language WASM.
4. Extend the neutral program for empty geometry, 2D, polygon/polyhedron and
   extrusions, then connect stable evaluator branches to normalized plans.

The TS adapters retain warning text, callbacks and frozen public objects.
Authored expressions must be evaluated once before normalization.

## Production switch requirements

- Compare normalized plans and ordered warning events against frozen TS oracles,
  including invalid, nonfinite, partial-vector and empty inputs.
- Verify packed WASM execution for both profiles, nested modules, loops,
  transforms and Booleans.
- Preserve materials, provenance, imports and the current B-rep result contract.
  Polygon mesh parity alone does not prove production parity.
- Measure whole-program transport and execution, including resource refusals.
  Matrix construction now returns classification in the same response: Euler
  vectors, scalar rotation, authored matrices, translation, scale and reflection
  require one request each. Tests assert these request counts. Whole-program
  timing is still required before claiming a performance improvement.
- Audit remaining parser algorithms before retiring the TS evaluator.

This inventory covers the OpenSCAD boundary only. The repository-wide TS owner
audit is recorded in [algorithm-owners-2026-10-03.md](algorithm-owners-2026-10-03.md);
its migration backlog remains open.
