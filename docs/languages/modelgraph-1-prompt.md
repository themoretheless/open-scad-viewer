# ModelGraph/1 — инструкция для модели

Создавай параметрические 3D-модели в JSON по приложенной схеме. Используй чистые функции и именованные параметры для повторно используемых деталей. Не вставляй программный код в строки.

ModelGraph/1 is a declarative JSON modeling language for MCP. Produce documents matching the accompanying JSON Schema. Units: millimeters; rotations: degrees, OpenSCAD Euler order (X then Y then Z); right-handed Z-up. IDs match [A-Za-z][A-Za-z0-9_]{0,31}. Parameters and nodes each have unique IDs. Scalar fields accept numbers or {"param":"id"}; strings of code are forbidden; structured functional expressions are supported. box uses positive size[3], sphere positive radius, cylinder positive radius and height. Box/cylinder center defaults false. translate/rotate/scale take vector[3] and input node ID; scale factors cannot be zero. union/intersection take inputs; difference takes base and subtract. Transforms wrap the referenced geometry. Every node must be reachable from root. Shared references instantiate geometry at each use; cycles are forbidden. Limits: 64 parameters, 128 nodes, depth 32, 4096 expanded uses, segments 12..128 (default 48). Assembly nodes: {id,op:"assembly",components:[{id,input:solidNodeId,anchors:[{id,origin:[length,length,length],rotation:[angle,angle,angle]}],placement?:{origin,rotation},mate?:{component:targetComponentId,anchor:targetAnchorId,own_anchor:localAnchorId,gap:length,rotation:[angle,angle,angle]}}]}. An assembly must be the document root or a direct component of another assembly. Assemblies inside boolean operations, transforms or functions are rejected. There are at most 64 expanded components across all nesting levels. Each component must declare exactly one placement or mate. At most 32 components and 32 anchors per component. Component and local anchor IDs must be unique. Components remain separate top-level geometry objects; they are not implicitly unioned. References to the same solid instantiate independent components.
Placement is a fixed rigid frame using degrees with X then Y then Z rotation order. Fixed mates are acyclic dependencies, not a numerical joint solver. The transform is targetComponent * targetAnchor * localGapAndRotation * inverse(ownAnchor). Gap is a signed displacement along target anchor Z; rotation is an additional local Euler rotation. Axes align by default; opposing faces require an explicit rotation. Gaps are declared frame offsets, not measured surface clearances. Unknown components/anchors, cycles and ambiguous placements are errors. assembly_components in compile/check/report returns each component's id,input,world matrix and named world anchor matrices. Matrices are row-major. Combined STL/OBJ export does not preserve assembly identities; keep the ModelGraph JSON as the assembly source of truth. Nested assembly instances expose instance_path, parent_path and is_assembly; leaf components also expose standalone source in world coordinates. Optional mate.joint is {kind:"revolute"|"slider",position:scalar,min:scalar,max:scalar}. Revolute positions/limits are angles; slider positions/limits are lengths. Position must be within inclusive limits. The joint rotates around local Z or translates along local Z after gap/rotation and before inverse(ownAnchor). Parameters and expressions can drive positions. These are acyclic kinematic placements, not physics simulation or a closed-loop assembly constraint solver. Call modelgraph_interference to measure volume overlap between at most 8 leaf components at current positions. It returns pair paths, intersection_volume_mm3 and status overlap/no_volume_overlap/unknown. Kernel failures are unknown; never interpret them as clearance. A tolerance of 1e-6 mm3 applies. Contact, clearance and swept motion are not checked.
Call modelgraph_report for actual geometry measurements, operation provenance and front/top/isometric PNG views. Images are bounded to 20000 triangles and 8000000 raster candidates and may be unavailable; consult images_status. Reports do not certify printability or persistent CAD face identity. First call modelgraph_compile, fix errors using their JSON path, then modelgraph_check for actual geometry validation. Edit parameters through modelgraph_set_parameters using the returned document_sha256. Keep the returned document as the source of truth. Generated SCAD is an execution artifact usable with existing export tools. No tool here persists the graph. Stable IDs identify document nodes, not stable CAD faces. This frontend currently targets the existing Manifold mesh engine, not a new geometry kernel, exact B-rep, CUDA or a full OpenSCAD replacement.
Profiles and solid features: rectangle(size:[length,length],center=false), circle(radius:length), polygon(points:[[length,length],...]) create XY profiles. Polygon points implicitly close the boundary; do not repeat the first point. A polygon has 3..256 points and must be simple with nonzero area. Holes are modeled with profile difference; union/intersection/difference must combine the same geometry dimension. extrude(input:profileId,height:positiveLength,center=false) and revolve(input:profileId,angle:positiveAngle<=360deg) create solids. Revolve interprets profile X as radius and Y as height around Z; place the profile on one side of the axis and validate the actual result using modelgraph_check. Transforms of profiles must preserve XY: translate Z=0, rotate X=Y=0, scale Z=1. Root must be solid. Functions and geometry values can carry profiles; expected profile/solid dimension is checked when emitted. Node references use input as elsewhere. Polygon-validation work is capped at 262144 point-pair budget units across expanded uses. A sketch node adds a bounded straight-segment constraint solver; arc constraints and edge fillets are not implemented.

## Functional programming

Functional extension (backward compatible with ModelGraph/1):
Values are numbers, immutable lists, lexical closures and immutable geometry values. No mutation, IO, clock, random or eval. All numeric intermediates are finite and within +/-1000000. Predicates return 0 or 1; conditions treat zero as false. Trigonometry uses degrees.
Expressions: {param:id} reads a document parameter; {local:id} reads a lexical binding. Binary {op,args:[a,b]}: add, subtract, multiply, divide, mod, pow, min, max, lt, le, eq, and, or. Unary {op,value}: negate, abs, sqrt, sin, cos, floor, ceil, not. if uses condition, then, else and evaluates only the selected branch; and/or short circuit. let uses name,value,body; the new binding exists only in body. lambda uses parameters:[ids],body and captures its definition scope. apply uses function:expression,args:[expressions]; arguments are positional. Duplicate binders are errors; lexical shadowing is allowed.
Lists: list uses items; range uses count,start,step (count 0..256); at uses input,index (zero based); length uses input. map and filter use input and function (one argument: element). reduce uses input,function,initial and folds left with callback(accumulator,element). Empty reduce returns initial. Lists may hold closures or lists; a geometric numeric field must resolve to a number.
Document functions: {id,kind:"scalar"|"value",parameters:[ids],body:expression} or {id,kind:"geometry",parameters:[ids],nodes:[...],root:id}. scalar must return a number; value can return a list or closure. Named call uses {op:"call",function:id,args:{parameter:expression}}. Geometry call is a node with an id and the same fields. Exact named argument matching is required. Named functions see document parameters and their own arguments, never caller locals. Pass closures as arguments to write higher-order value functions. The expression {op:"geometry",function:id,args:{name:expression}} constructs an immutable geometry value with bound arguments. A geometry node {id,op:"evaluate",value:expression} emits a geometry value. Closures may return geometry; lists and higher-order functions may carry geometry values. Geometry functions can accept geometry values and use evaluate nodes to compose them.
Geometry if nodes have condition,then:nodeId,else:nodeId. Geometry map nodes have count (1..256),index:localId,input:nodeId; each instance receives an immutable zero-based index and results are unioned. Node IDs are local to each geometry body. Document assertions:[{condition:expression,message:string}] run before geometry emission. source_map includes instance_path identifying function calls and map instances.
Bounds: 32 functions, 32 arguments per function, expression/call depth 32, 100000 expression steps, 16384 allocated list slots, 4096 expanded geometry nodes, geometry depth 32, 20000 input values and nesting depth 64. Recursion is permitted within these budgets; no tail-call optimization. Functions and inactive branches are schema checked; value-dependent errors are detected when evaluated. This is a bounded functional modeling DSL, not general-purpose JavaScript or a new CAD kernel.

## Пример: функция детали и массив экземпляров

```json
{
  "language": "modelgraph/1",
  "units": "mm",
  "parameters": [
    {
      "id": "count",
      "value": 3
    }
  ],
  "functions": [
    {
      "id": "block",
      "kind": "geometry",
      "parameters": [
        "width"
      ],
      "nodes": [
        {
          "id": "solid",
          "op": "box",
          "size": [
            {
              "local": "width"
            },
            2,
            2
          ]
        }
      ],
      "root": "solid"
    }
  ],
  "assertions": [
    {
      "condition": {
        "op": "le",
        "args": [
          {
            "param": "count"
          },
          20
        ]
      },
      "message": "Use at most 20 blocks."
    }
  ],
  "nodes": [
    {
      "id": "block",
      "op": "call",
      "function": "block",
      "args": {
        "width": {
          "op": "apply",
          "function": {
            "op": "lambda",
            "parameters": [
              "x"
            ],
            "body": {
              "op": "multiply",
              "args": [
                {
                  "local": "x"
                },
                2
              ]
            }
          },
          "args": [
            1
          ]
        }
      }
    },
    {
      "id": "placed",
      "op": "translate",
      "vector": [
        {
          "op": "multiply",
          "args": [
            {
              "local": "i"
            },
            3
          ]
        },
        0,
        0
      ],
      "input": "block"
    },
    {
      "id": "parts",
      "op": "map",
      "count": {
        "param": "count"
      },
      "index": "i",
      "input": "placed"
    }
  ],
  "root": "parts"
}
```

## Types, units and declared constraints

Types, units and declared constraints:
For new documents use type_policy:"strict". Expressions {op:"quantity",value:number,unit:"mm"|"cm"|"m"|"in"|"deg"|"rad"} create typed lengths/angles. Values normalize internally to mm and degrees; returned documents retain the original units. Bare numbers are dimensionless. Default/legacy policy still interprets bare numbers as mm/degrees at geometry fields; strict policy requires correct dimensions there, except bare zero is accepted for any geometry dimension. Explicitly typed values are always checked, including in legacy mode.
Box sizes, radius, height and translation require length; rotation requires angle; scale, predicates, indices and counts require dimensionless numbers. Function arguments, closures, lists, geometry values and scalar-function returns preserve quantities. Checks occur during evaluation, not by whole-program static inference. Inactive branches retain lazy semantics.
Add/subtract/min/max/mod/comparisons require identical dimensions; implicit number-to-length promotion in arithmetic is forbidden. Multiply/divide combine dimensions; length/length is dimensionless. Powers require a dimensionless exponent; resulting length/angle exponents must be integers within +/-8. sqrt halves exponents. sin/cos accept angles, returning dimensionless values (legacy mode also accepts bare degrees). abs/negate preserve dimensions; floor/ceil round canonical mm/degrees, preserving dimensions. Typed ranges require matching start/step dimensions; count remains dimensionless. Numeric limits apply after conversion as well as during arithmetic.
Parameters may declare unit,min,max,integer. Bounds and update values are expressed in the parameter's declared unit; modelgraph_set_parameters preserves that unit and revalidates all bounds/constraints atomically. An omitted unit denotes a dimensionless parameter.
Document constraints:[{id,left:expression,relation:"le"|"ge"|"eq",right:expression,tolerance?:expression,message:string}] compare numeric values with identical dimensions. Tolerance must have the same dimensions and be nonnegative; omitted means exact comparison. le means left <= right+tolerance; ge means left >= right-tolerance; eq means abs(left-right) <= tolerance. IDs must be unique. Maximum 64 constraints. constraint_report contains id,path,passed,actual,expected,relation,tolerance,dimension:[lengthExponent,angleExponent],message; measurements use canonical mm/degrees. If any fail, compilation returns constraint_failed with the entire report in error.details and emits no geometry. Legacy assertions still work.
These are checks of declared expressions, not a constraint solver or measured mesh-wall analysis. A minimumWall constraint checks the expression you provide; it does not prove every wall in the finished mesh meets that minimum. No automatic parameter repair, sketches or geometric constraint solving is introduced.

Пример с единицами и именованным ограничением доступен в modelgraph-1.units.example.json и поле units_example MCP-ресурса.

## Sketch constraint solver

Sketch constraints (straight-segment profiles):
A profile node {id,op:"sketch",points:[{id,position:[x,y]}],boundary:[pointIds],constraints:[...],allow_underconstrained?:false} solves coordinates before profile emission. Positions are initial guesses, not fixed values. Point positions, fixed targets and distances accept length expressions, respecting strict units. Boundary is an ordered simple closed polygon; closure is implicit and IDs must be unique. Extra points may be construction points but their degrees of freedom still count.
Constraint forms: {id,kind:"fix",point:id,at:[x,y]}; {id,kind:"horizontal"|"vertical"|"coincident",a:pointId,b:pointId}; {id,kind:"distance",a,b,value:positiveLength}; {id,kind:"parallel"|"perpendicular"|"equal_length",a,b,c,d} relates segments a-b and c-d. Use coincident for zero distance. Parallel/perpendicular reject zero-length segments at the resulting solution.
The bounded numerical solver uses initial guesses to select a local solution. Limits: 3..16 points, at most 48 constraints, 64 iterations, 16 expanded sketch solves per compilation. Tolerance is 0.000001 mm. sketch_solutions contains solved coordinates, per-constraint residual_mm/satisfied, maximum residual, iterations, local degrees_of_freedom (Jacobian nullity) and redundant_equations (local equation-count minus rank). Rank is numerical, not a global uniqueness proof. Redundancy counts equations, not necessarily removable constraints.
Statuses: solved; underconstrained (satisfied but local freedom remains); inconsistent (unsatisfied linear system with augmented-rank conflict); not_converged (nonlinear solver exhausted its budget, singular start or degenerate segment). Unsatisfied residuals identify problematic constraints but are not a proven minimal conflict set. No global impossibility claim is made for nonlinear nonconvergence. By default only solved sketches emit geometry. allow_underconstrained:true explicitly accepts a locally underconstrained solution using the initial guess. Invalid or self-intersecting solved boundaries still fail profile validation. Compilation errors include the solver report in error.details; successful compilation and modelgraph_report include sketch_solutions.
No arc/circle/tangency solver, assembly solver or automatic repair is implemented by this node.

Example: modelgraph-1.sketch.example.json, also exposed as sketch_example in the MCP language resource.

## MCP workflow

1. Read openscad://language/modelgraph-1 for the current schema, guide and examples.
2. Call modelgraph_compile with document. This validates and evaluates expressions but does not build geometry.
3. Call modelgraph_check with document to build actual geometry and inspect measurements.
4. For assemblies, call modelgraph_interference to inspect current-pose volume overlap; unknown is not a clearance result.
5. To change parameters, call modelgraph_set_parameters with document, expected_document_sha256 and updates:[{id,value}]. Keep the returned document and validate geometry again.

The caller owns document storage. The hash checks the supplied document, not concurrent external storage. Generated source can be passed to existing OpenSCAD export tools. Error paths for evaluated instances and source_map.instance_path locate function calls and repetitions; they do not identify stable CAD faces. Runtime geometry errors may still refer to generated SCAD.

The browser editor still accepts OpenSCAD. Geometry is computed by Manifold; this language does not introduce B-rep, CUDA or guarantees of printability. Closures may return geometry values, which evaluate nodes insert into the geometric graph. No pattern matching or static type inference.
