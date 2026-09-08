# ModelGraph Text/1

Implemented compact authoring syntax, lowered to the existing ModelGraph/1 document and compiler. Begin `.scad` files with `// @modelgraph-text/1`; the supported viewer parser detects this marker. Official OpenSCAD tools do not accept this frontend.

See `examples/modelgraph-text/ring-pattern.scad` for a runnable example.

- Newlines or semicolons separate declarations. Continue a pipeline on the next line with `|>`.
- Arithmetic: `+`, `-`, `*`, `/`, unary minus, parentheses and vectors.
- Units: `mm`, `cm`, `m`, `in`, `deg`, `rad`.
- `param radius = 24mm range 15mm..40mm` produces a parameter and editor slider. Defaults and endpoints are literals using the same unit. Slider edits replace only the numeric span.
- `name = expression` binds a value once. `show name` selects the result; otherwise the last declared geometry is used. Duplicate declarations are errors.
- Unary functions use `name = x => expression` and capture lexical bindings. Scalar and geometry results are supported. Recursive and forward references are not supported.
- `repeat(count, i => geometry)` lowers to bounded ModelGraph map semantics, which union the results. A direct parameter used as repeat count gets an integer slider.
- Positional/named calls: `circle(radius)`, `rectangle(size)`, `box(size)`, `sphere(radius)`, `cylinder(radius,height)`; piped modifiers: `extrude(height)`, `revolve(angle)`, `translate(vector)`, `rotate(vector)`, `scale(vector)`, `offset(distance)`, `mirror(normal)`.
- `union(parts...)`, `intersection(parts...)`, and piped `subtract(cutters...)` combine geometry. Piping into a primitive is rejected.

The initial compact operation catalog is deliberately bounded; the full JSON language remains available for operations not listed above. No JavaScript eval or host access is used. Source length is capped at 256 KiB, nesting/function expansion at 64, and geometry nodes at 128, in addition to ModelGraph budgets.

MCP: call `modelgraph_language` for `text_guide`, then `modelgraph_text_compile`. It returns canonical `document`, generated SCAD and parameter spans. Pass `document` to `modelgraph_check`, `modelgraph_report` and `modelgraph_export`.

Generated SCAD source locations are not valid compact-source locations. Browser triangle source references are cleared for compact builds; geometry selection remains available but reverse highlighting to compact declarations is not implemented.

Validation covers actual Manifold construction, the repeat example, parameter/unit span preservation, syntax rejection, HTTP MCP compilation/build, and browser sliders. The worker includes the compiler, adding approximately 140 KiB to the uncompressed distribution; the total budget is updated from 2.0 to 2.2 MB, while per-file limits remain unchanged.

## Own Rust NURBS and mesh operations

See `examples/modelgraph-text/nurbs-boolean.scad`. Use `nurbs_surface(degree_u,degree_v,knots_u,knots_v,control_points,weights)` or `nurbs_curve(degree,knots,control_points,weights)`, with positional or named arguments. Pipe surfaces into `tessellate(segments_u,segments_v)` and meshes into `thicken(vector)`. NURBS constructors are `surface_extrude(vector)` and `surface_revolve(origin,axis,angle)`; `transform(matrix)` takes an affine 4x4 matrix. Both piped and dotted call forms work.

`mesh_union(a,b)`, `mesh_intersection(a,b)` and `a |> mesh_subtract(b)` lower to own Rust mesh CSG. They require exactly two closed oriented meshes. This branch produces `modelgraph/nurbs-1` with `execution_target: own-nurbs`; the viewer publishes the resulting mesh directly, without compiling generated SCAD or invoking Manifold. Empty results clear the scene. Use `modelgraph_nurbs_build/evaluate/export` with the returned document over MCP.

The compiled NURBS JSON is a numeric snapshot: source parameter defaults, units and arithmetic are resolved at compilation. Source text and Customizer sliders remain editable and recompile all dependent expressions. Snapshot `parameters` is empty to avoid implying that changing a JSON parameter recomputes an already resolved expression. Bare lengths use millimeters, angles degrees; incompatible explicit units fail. The existing NURBS graph and Rust geometry budgets apply.

This branch currently rejects legacy primitives, groups, repeat and assertions in the same graph. Curve/surface roots can be compiled for numerical queries; the viewer requires a tessellated mesh. The existing `planetary_spinner` program remains on its original route, with its separate 20 meshes and six parameters unchanged.

`brep_box(min,max) |> brep_tessellate(segments)` uses the shared B-rep topology and publishes authored face IDs, so selecting a tessellated face selects its whole source surface. The native/API B-rep adapters support both NURBS and polygon face geometry; this text constructor currently exposes the NURBS box.
