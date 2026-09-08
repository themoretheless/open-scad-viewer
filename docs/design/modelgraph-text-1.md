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

## SKADIS dovetail example

`examples/skadis-box/skadis-dovetail.modelgraph.scad` is the editable Text/1
translation of the original `skadis-dovetail.scad`. It retains dimension and fit
parameters, part selection (0 assembly, 1 box, 2 hook oriented for printing),
optional friction ridge (0/1), and construction constraints. Source colors are
not represented; assembly parts remain separate meshes.

The legacy frontend also exposes `polygon(points)` and `hull(parts...)`.
`offset(distance)` keeps round joins; `offset(delta: distance)` selects sharp
joins and lowers to an offset node with `mode: "delta"`. Omitted JSON mode keeps
existing radius behavior. `segments 40` preserves the source curve resolution;
this optional declaration accepts one integer from 12 to 128, defaults to 48,
and is rejected for the own-geometry backend, which has explicit tessellation
arguments. ModelGraph still generates SCAD internally on this legacy route;
the authored source no longer contains OpenSCAD modules.

## Conditional expressions

Use `condition ? thenValue : elseValue`. Conditions are dimensionless numbers:
zero is false, nonzero is true. Comparisons produce suitable conditions.
The operator associates right and binds less tightly than arithmetic and
comparisons. Parenthesize conditional values before applying a pipeline or
method. Branches may span lines after `?` and `:`. Example:

```text
param rounded = 1 range 0..1
radius = rounded ? 10mm : 20mm
body = rounded ? sphere(radius) : box([20mm,20mm,20mm])
show body
```

Scalar choices lower to canonical `if` expressions; geometry choices lower to
canonical `if` nodes. Only the selected branch is evaluated/built, including
inside generators. Both branches are parsed and lowered, so unknown names,
unsupported calls and syntax errors remain errors even in an unselected branch.
Branches must both be geometry or both be values. Selected geometry still
undergoes normal profile/solid checks. The own-geometry backend resolves choices
to its numeric snapshot; changing source parameters and recompiling selects a
new branch. Canonical legacy choices retain parameters and local bindings.

## Block functions, generics and named records

The runnable `examples/modelgraph-text/generic-functions.scad` demonstrates:

```text
struct Point3D<T> {
    x: f32,
    y: f32,
    z: T,
}
makePoint = fn<T>
    x: f32,
    y: f32,
    z: T,
->
    point: Point3D<T>,
    count: int,
{
    ret {
        point: Point3D<T> { x, y, z },
        count: 1,
    }
}
{ point, count } = makePoint(10, 20, 12)
show box([point.x, point.y, point.z])
```

Parameters and named results are separated by newlines or commas; trailing commas
are accepted. Bodies contain immutable local bindings and one final `ret` record.
The result fields must exactly match the declaration (order does not matter).
Access fields with dots, or destructure a subset by name. Struct constructors accept
named fields and shorthand; generic constructors require explicit type arguments.
Records are immutable frontend values, lowered into canonical scalar expressions
and geometry nodes. They do not add mutable host objects or executable code to JSON.

Generic function arguments are inferred from input values, or supplied explicitly
as `makePoint<f32>(10, 20, 12)`. Repeated inferred type variables must agree.
Calls accept positional arguments or an exact set of named arguments, never a mix.
Functions capture lexical bindings; forward references and recursive calls are not
supported. Generics have no constraints or default arguments in this version.

Supported types are `int`, `f32`, `f64`, `str`, `length`, `angle`, `Geometry`,
`Vec<T>` (literal vectors), and previously declared nominal structs. `str` values
can be carried in records/functions but are not numeric geometry expressions.
Generic inference distinguishes integral literals from floating literals/expressions;
parameter references without an integer constraint infer `f64`. Explicit generic
arguments can provide context when inference is insufficient. `length` and `angle`
require explicit compatible units; numeric annotations require dimensionless input.
`f32` rounds at annotated boundaries; intermediate ModelGraph arithmetic remains
JavaScript-number arithmetic, not a native f32 execution engine. Existing +/-1e6
numeric limits remain in force. `int` validates integrality and signed 32-bit range.

Canonical `typed` expressions preserve numeric validation when parameters change.
`checked` expressions evaluate argument/result checks before the used result, so an
invalid extra numeric result is not silently skipped when only geometry is consumed.
An unused function call does not execute canonical numeric checks. Own-geometry
compilation evaluates the same checks while producing its numeric snapshot.

Bounds: 32 fields/inputs/outputs, 16 generic parameters and type nesting levels,
64 local statements per function, 256 block-call expansions, existing expression
and geometry budgets. Unsupported or incomplete types, duplicate names, absent/extra
results, incompatible records and ambiguous inference produce compiler errors.

### Single results and expression bodies

A single unnamed return type yields the value directly, without a result record:

```text
identity = fn<T> value: T -> T => value
make = fn size: f64 -> Geometry {
    base = rectangle([size, size])
    ret base.extrude(3)
}
show make(2).translate([1, 0, 0])
```

An expression body can start on the next line. Named results remain supported with either a block returning a record or `=> { first: x, second: y }`. Generic inference, declared return-type checks and canonical parameter checks apply to both forms.

### Indented named functions

```text
fn get_first[T]
    a: int,
    b: T
->
    x1: T,
    x2: int,
    x3: int,
    x4: int

    count: a + 1
    ret
        x1: b, x2: count,
        x3: 55, x4: 6546
```

Signature lists and named returns require commas between items, including across lines. The final item has no trailing comma. `ret x1: b, x2: count,` can continue on a deeper-indented line. Body statements share one indentation; return field continuation lines share a deeper indentation. Blank lines do not delimit lists. Local `:` bindings are immutable, just like `=`. A single unnamed return uses `ret value`. Existing brace-based functions remain supported. Explicit generic calls continue to use `get_first<int>(1, 2)`.
