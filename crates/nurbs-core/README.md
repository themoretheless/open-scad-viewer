# nurbs-core

Binary64 rational B-spline curves and surfaces, editing, interval-bounded distances
and intersections. Native types and algorithms do not depend on polygon-core,
the viewer or WASM. `Curve` and `Surface` retain rational control data.

```rust
use nurbs_core::{curve::Curve, curve_distance};
let a = Curve {
    degree: 1, knots: vec![0.,0.,1.,1.],
    control_points: vec![vec![0.,0.,0.],vec![1.,0.,0.]],
    weights: vec![1.,1.], periodic: false,
};
let mut b = a.clone();
for p in &mut b.control_points { p[1] = 2.; }
let distance = curve_distance::distance(&a, &b, 1e-6, 1000)?;
assert!(distance.distance_interval_mm[0] <= 2.);
assert!(distance.distance_interval_mm[1] >= 2.);
# Ok::<(), nurbs_core::Error>(())
```

The default `transport` feature exposes `dispatch(Value)` and `execute(&str)`
through a separate JSON boundary and enables `codec`. Use
`default-features = false` for native curves, surfaces, computations and typed
reports without `value-codec`. Enable `codec` separately when only serialization
is needed. Native runtime dependencies are `osv-math`, `geometry-ops`,
`brep-topology` and `cad-predicates`.

`Curve::bounds()` and `Surface::bounds()` return native `bounds::Bounds` with
`min` and `max` coordinate vectors. `bounds::from_points` rejects empty,
nonfinite or dimensionally inconsistent point lists.

Formula calculations accept `formula::Token` instructions; `formula::parse`
converts bounded infix text without JSON. Host strings and numeric token lists
are decoded separately. `framed_sweep::checked_sweep` returns `CheckedSweep`
and `SweepReport`; rejected refinement results contain no surface. Its sampled
control deviation is a diagnostic, not a continuous error certificate.

`foundation::project_surface_report` returns typed parameter boxes, boundary
reductions and affine, distance-separation or Krawczyk uniqueness evidence.
Surface projection uses the native curve reports for boundary reductions.

`foundation::project_curve_report` returns typed candidates, parameter and distance
intervals, projection status, and the winning candidate index. Point trimming
uses that report directly through `trim_point::trim_at_point_report` and
`trim_at_screen_point_report`. The latter records CSS pixels as the projection
space; the retained curve and cut point remain in original coordinates.
Compatibility adapters preserve the existing JSON response fields.
Run `cargo run -p nurbs-core --no-default-features --example typed_projection`
from this workspace to use the native report and trimming APIs directly.

`intersection::intersect_curve_surface_report` returns native `Point` and
`Overlap` events, typed unresolved parameter regions and coverage counts.
Deduplication and parameter ordering use native fields; serialization preserves
the existing host response.

`intersection::intersect_curve_curve_report` returns native intersection
components and unresolved parameter boxes. Trim intervals use the shared
`brep-topology::CoedgeTrim` definition. The existing `intersect_curve_curve`
entry point serializes the same report for host compatibility.
Run `cargo run -p nurbs-core --no-default-features --example typed_intersection`
from the workspace for a direct native example.

Direct APIs include `curve::Curve`, `surface::Surface`, `curve_distance::distance`,
`surface_distance::distance`, curve/surface and surface/surface intersection,
trim-domain checks and exact-edit evidence. Interval results expose convergence,
resource budgets and unresolved outcomes; callers must preserve those statuses.
B-rep topology, solid certification and display tessellation belong to consumers.

`primitives` constructs rational ellipse arcs (including oblique 3D axes),
axis-aligned ellipsoids and ring tori with circular or elliptical tubes.
These are rational constructions, without sampled fitting; binary64 roundoff
remains. Full turns use clamped bases with identical endpoint controls, rather
than claiming a periodic basis. Ellipsoid poles are parameter singularities.
Surfaces alone do not certify a closed solid.

```rust
use nurbs_core::primitives::{ellipse_arc, ellipsoid, torus};
let arc = ellipse_arc([0.;3], [10.,0.,0.], [0.,5.,0.], 0., 270.)?;
let shell = ellipsoid([0.;3], [10.,5.,8.])?;
let ring = torus([0.;3], 20., 3., 2.)?;
# Ok::<(), nurbs_core::Error>(())
```

JSON operations: `curve_ellipse_arc` (`center`, `axisU`, `axisV`,
`startDegrees`, `sweepDegrees`), `surface_ellipsoid` (`center`, `radii`),
`surface_torus` (`center`, `majorRadius`, `radialRadius`, `axialRadius`).
Angles use degrees; curve U domains are [0,1], revolution V domains [0,4].
Tori require majorRadius > radialRadius; zero radii, dependent ellipse axes,
nonfinite data and zero or over-full arc sweeps return errors.

`primitives::circle_quadrants(radius)` returns four XY rational quadratic arcs
with normalized domains. Signed radii preserve the authored phase; radius zero
returns four constant curves for legacy graph recording.

Additional rational primitives:

- `primitives::parabola(center, axis_u, axis_v, start, end)` represents
  `center + axis_u*t + axis_v*t*t`; its [0,1] parameter maps affinely to t.
- `primitives::hyperbola(...)` represents one finite branch
  `center + axis_u*cosh(t) + axis_v*sinh(t)`. Its rational parameter is
  not affine in t; unrepresentable data returns an error.
- `primitives::elliptic_cylinder(center, radius_x, radius_y, height)` returns
  an uncapped side surface with domains U=[0,1], V=[0,1].
- `primitives::cone_frustum(center, bottom_radius, top_radius, height)` returns
  an uncapped circular side surface with U=[0,1], V=[0,4]. One radius may
  be zero, producing an intentional singular apex; both may not be zero.

JSON operations are `curve_parabola`/`curve_hyperbola` (`center`, `axisU`,
`axisV`, `start`, `end`), `surface_elliptic_cylinder` (`center`, `radiusX`,
`radiusY`, `height`) and `surface_cone_frustum` (`center`, `bottomRadius`,
`topRadius`, `height`). These additions currently have native/JSON coverage;
their frontend integration is tracked separately in `docs/design/nurbs-catalog.json`.

`primitives::quadratic_patch([xMin,xMax,yMin,yMax], [a,b,c,d,e,f])`
constructs the polynomial surface `z=a*x*x+b*x*y+c*y*y+d*x+e*y+f` with
a biquadratic tensor basis. JSON operation `surface_quadratic_patch` accepts
`bounds` and `coefficients`. Both domains are [0,1]. It covers elliptic and
hyperbolic paraboloid graphs, with mixed terms and linear offsets. Increasing
finite bounds and finite coefficients are required. Construction overflow is
rejected; no solid or periodic closure is implied.

Finite elliptic hyperboloid surfaces are available as
`primitives::hyperboloid_one_sheet(center, radii, start, end)` and
`primitives::hyperboloid_two_sheet(center, radii, start, end, lower)`.
The former satisfies `X²/a²+Y²/b²-Z²/c²=1`; the latter satisfies
`Z²/c²-X²/a²-Y²/b²=1`, selecting one connected sheet. Coordinates are
relative to center. Both use rational conics and revolution without fitting.
Two-sheet intervals require `0 <= start < end`, with a pole at start=0.
JSON names are `surface_hyperboloid_one_sheet`/`surface_hyperboloid_two_sheet`,
with `center`, `radii`, `start`, `end` and optional `lower` (default false).
U=[0,1], V=[0,4]; U is not affine in the analytic hyperbolic parameter.
These are uncapped surfaces with native/JSON coverage, not closed solids.

`polynomial::graph(bounds, coefficients)` constructs a polynomial graph
`z = sum(coefficients[i][j] * x^i * y^j)` over `[xMin,xMax,yMin,yMax]`.
It uses affine substitution and conversion from powers to Bernstein controls,
without sampling or fitting. The rectangular coefficient grid is limited to
13x13 (degree 12 per axis). Constant and linear cases are supported. Output
domains are [0,1]x[0,1], weights are one. Binary64 rounding and conditioning
remain; this API does not claim an interval error certificate. Nonfinite inputs,
ragged/oversized grids, reversed bounds and construction overflow return errors.
JSON operation `surface_polynomial_graph` takes `bounds` and `coefficients`.
This is a graph in mm coordinates, with coefficient units determined by their
powers, not an arbitrary parameterized surface or implicit-surface extractor.

`polynomial::parametric_curve([tMin,tMax], coefficients)` represents
`P(t)=sum(coefficients[i]*t^i)` with XYZ coefficient vectors.
`polynomial::parametric_surface([uMin,uMax,vMin,vMax], coefficients)`
represents `S(u,v)=sum(coefficients[i][j]*u^i*v^j)` with XYZ vector grids.
These polynomial conversions use the same degree-12 limit and return clamped
definitions over normalized [0,1] parameters. Domain scaling is affine.
They support arbitrary spatial polynomial curves and overhanging patches;
constant and singular results are allowed, with no claim of regularity,
injectivity, closure or solid topology. JSON operations are
`curve_polynomial_parametric`/`surface_polynomial_parametric`, taking `domain`
and `coefficients`. Native/JSON coverage is separate from frontend integration.

`polynomial::rational_curve(domain, coefficients)` and
`polynomial::rational_surface(domain, coefficients)` take homogeneous power
coefficients `[X,Y,Z,W]` and represent `[X/W,Y/W,Z/W]`. Conversion normalizes
the homogeneous scale and accepts either a common positive or negative sign
of all Bernstein denominator controls, then materializes positive weights.
Mixed signs and zero controls are rejected: this conservative gate can refuse
a denominator that is analytically positive but lacks a positive Bernstein
representation on the requested interval. It does not silently fit or sample
around possible poles. Existing coordinate/weight conditioning limits apply.
JSON operations are `curve_rational_polynomial`/`surface_rational_polynomial`,
with `domain` and `coefficients`. Degree limits and normalized domains match
the polynomial constructors. Tests cover a rational circle chart, negative
homogeneous scaling, mixed-sign refusal and a nonseparable rational patch.

The TypeScript constructors in `src/services/nurbsConstructors.ts` expose these
seven constructors through the packaged geometry WASM. RushGraph/Rush names
are `hyperboloid_one_sheet`, `hyperboloid_two_sheet`, `polynomial_graph`,
`polynomial_curve`, `polynomial_surface`, `rational_polynomial_curve`, and
`rational_polynomial_surface`; runnable sources are under `examples/rush/`.
Formula domains and coefficients use scalar literals. XYZ coefficients follow
the millimetre coordinate convention; homogeneous coefficients are specified
directly as XYZ/W, rather than interpreted as uniformly dimensioned vectors.
Graph bounds, hyperboloid centres and radii accept length units. Hyperbolic
start/end values are dimensionless. Formula constructors do not infer units,
fit data, cap side surfaces, or certify regularity of arbitrary supplied nets.

`primitives::line(start,end)` and `primitives::polyline(points,closed)` provide
degree-one 3D paths on [0,1], using equal parameter intervals per segment.
They reject zero-length adjacent segments. Closed paths need at least three
vertices, explicitly repeat the first point and remain clamped and nonperiodic.
The existing 256-control-point limit includes the repeated closure point.

`primitives::circle(center,normal,radius)` and `circle_arc(...,start,sweep)`
construct rational circles in arbitrary planes. Normal magnitude is irrelevant;
zero/nonfinite normals and nonpositive radii are rejected. The reference start
direction projects the least-aligned Cartesian axis into the plane (X before Y
before Z on ties); positive angles follow the normal. This reference can change
when the least-aligned axis changes: supply ellipse axes when a continuously
controlled frame is needed. JSON names are `curve_line`, `curve_polyline`,
`curve_circle`, `curve_circle_arc`; Rush names are `line_curve`, `polyline_curve`,
`circle_curve`, `circle_arc`. Normals are scalar vectors, endpoints and points
accept length units, arc angles accept degrees.

`paths::bezier(points,weights)` constructs a degree-1..25 single Bezier span.
Weights default to one and must satisfy the core positive-weight constraints.
`paths::compose(curves)` joins 2..32 clamped, nonperiodic curves whose endpoint
controls match exactly, elevating them to a common degree. It rescales each input's weights
by a common factor to share the seam weight, retaining each rational piece.
The composite domain is [0,1], with equal intervals per input. Only C0 at joins
is guaranteed; there is no automatic snapping, reversal or
G1/G2 matching. The result has at most 256 controls. Unrepresentable weight
scales or parameter mappings that collapse knots are rejected. Native tests
compare rational Bezier evaluation with independent homogeneous de Casteljau
and composite evaluation with each original piece on its own domain.

`patches::bezier(points,weights)` provides a rational tensor-product Bezier
surface with 2..26 rows and columns, degrees inferred from the net dimensions,
normalized domains and optional positive weights (default one). Rectangular
net and weight dimensions are validated. `patches::bilinear(corners)` uses four
corners indexed [u][v], including nonplanar and degenerate patches; regularity
is not automatically certified. `patches::plane(origin,axis_u,axis_v)` builds an
affine patch and rejects dependent axes and spans that collapse when added to
the supplied origin at binary64 coordinate precision. JSON names are
`surface_bezier`, `surface_bilinear`, `surface_plane`; Rush names are
`bezier_surface`, `bilinear_patch`, `plane_patch`. Coordinates accept lengths;
rational weights remain dimensionless. Tessellation is derived display data.

### Circular surface constructors

`primitives::sphere`, `cylinder` and `cone` expose the circular specializations of the rational ellipsoid, elliptic cylinder and frustum. Sphere poles and the cone apex are intentional parameter singularities. Cylinder and cone produce uncapped side surfaces; none of these constructors certifies closed solid topology. Coordinates, radius and height use model length units; radius and height must be finite and positive. JSON operations are `surface_sphere`, `surface_cylinder`, `surface_cone`; Rush operations are `sphere_surface`, `cylinder_surface`, `cone_surface`.

### Section constructions

`surface::extrude` translates a retained 3D curve along a finite nonzero vector. `surface::revolve` rotates it around an origin/axis by a signed nonzero angle of at most 360 degrees; rational circular arcs preserve the rotation, while the arc parameter is not affine in angle. `surface::loft` requires equal degree, knots and control counts. Its transverse degree is one, with rational section interpolation at integer stations; equal section weights give affine generators. `surface::loft_aligned` normalizes section domains, elevates degrees and unifies knots before the same interpolation. These APIs do not supply smooth transverse curvature, closed end caps or guide curves. Rush exposes `surface_extrude`, `surface_revolve`, `ruled_surface` and `surface_loft`.

### Coons in graph documents

Rush `coons_patch(bottom,top,left,right)` retains four boundary curve references. Bottom and top follow +u; left and right follow +v. Exact corner coordinates must agree, the homogeneous corner-weight cycle must be compatible, and the resulting control weights must be positive and finite. Invalid boundaries fail without snapping or automatic weight preparation. The native homogeneous construction reproduces the supplied boundary parameterizations in real arithmetic; binary64 construction and evaluation still round. It does not certify surface injectivity or regularity.

### Coordinate formulas without fitting

`formula::curve(domain, expressions)` accepts three bounded reverse Polish expressions for x/y/z. Tokens are finite numbers, `t`, unary `neg` and binary `+`, `-`, `*`, `/`. For example `["t","t","*",2,"+"]` means t²+2. Each coordinate has at most 64 tokens and must leave exactly one stack result. Intermediate and final power-basis numerator/denominator degrees are limited to 12. The constructor shares identical coordinate denominators, converts the common homogeneous polynomial to Bernstein form and returns rational NURBS on [0,1]; no samples or fit tolerance enter this construction. Domain endpoints and `t` are dimensionless; constants and output coordinates are interpreted in model millimeters, without symbolic physical-unit inference. Overflow, invalid token stacks, degree budgets and nonpositive/mixed Bernstein denominator weights fail explicitly. Algebraic factor cancellation is not automatic. A conservative denominator check may reject a nonsingular expression; splitting its domain or simplifying it is the caller's choice. Real-arithmetic equivalence is distinct from binary64 rounding. Trigonometric/exponential expressions remain separate future work requiring approximation with a declared tolerance. JSON: `curve_formula`; Rush: `formula_curve(domain: [...], expressions: [[...],[...],[...]])`.

`formula::surface([u0,u1,v0,v1], expressions)` extends the same bounded algebra to two variables, using `u`/`v` instead of `t`. Tensor powers are limited to degree 12 independently in each axis, including intermediate/common-denominator products. Shared coordinate denominators are reused; distinct denominators are multiplied explicitly. The output patch has normalized [0,1]² domains. Mixed terms remain algebraic, and no sampled fitting or regularity/injectivity certificate is inferred. JSON operation: `surface_formula`; Rush: `formula_surface`.

Coordinate formulas also accept infix strings such as `"8*t"`, `"3*t^2"`, `"u^3-3*u*v^2"` or `"u/(1+u*v)"`. The grammar includes finite decimal/scientific constants, explicit `+ - * /`, parentheses, unary signs and literal integer powers of magnitude 1..12 (including negative reciprocals). `-t^2` means `-(t^2)`. Power zero and chained powers are deliberately refused; write their intended expression explicitly. Limits: 1024 ASCII bytes per string, nesting 24 and at most 64 expanded reverse Polish tokens. Text identifiers are only `t` for curves or `u/v` for surfaces; token-list notation still supports scalar graph parameter references. Strings are compiled to the same rational algebra, with the same overflow, degree and denominator refusals. Mixed string/token-list coordinates are supported.

### Sweep arithmetic and diagnostics

The translational sweep forms controls as `profile + (pathControl - pathStart)` so a distant path origin does not erase a small local profile. Tensor weight products retain their original scale when admitted; otherwise only common source scales are removed before multiplication. Excessive relative conditioning still fails. Rotation-minimizing transport is a separate discrete construction, exposed in Rush as `framed_sweep(profile,path,normal:[...],sections:...,max_deviation:...mm)`. Its fourfold-refinement comparison is sampled, `continuousBound:false`; graph reports retain this diagnostic and do not mark its construction error certified. This constructor does not certify a continuous ideal frame sweep, self-intersections, wall thickness or a closed solid. Closed sampled frames have a C0 seam; tangent discontinuities and ambiguous initial frames are refused.

### Hermite positions and tangents

`hermite::interpolate(points,tangents,parameters)` constructs piecewise cubic spans through 2..86 authored 3D positions with dP/dt tangents. Parameters are finite and strictly increasing; normalized output u maps affinely across their full span, so dP/du scales by that span. Stationary tangents and repeated positions are allowed, without regularity certification. Positions are materialized as shared span endpoint controls. Interior knots have multiplicity three; use trimmed one-sided jets at joins, since the evaluator conservatively reports insufficient continuity there. Finite-control limits, nonrepresentable parameter normalization and collapsed nonzero tangent controls refuse the construction. Multiplying span by tangent before division preserves compensating extreme scales when their product is representable. JSON: `curve_hermite`; Rush: `hermite_curve(points:[...],tangents:[...],parameters:[...])`. Tangents use model length units per dimensionless t. The 256-control curve budget permits 86 sites, while derived surface constructors have their own stricter row limits.

### Natural cubic interpolation

`natural_spline::interpolate(points,parameters)` computes cubic moments through 2..86 sites with zero endpoint second derivatives. Tangents are inferred by the natural moment system; authored parameters are strictly increasing and output is normalized to [0,1]. Real-arithmetic C2 behavior is distinct from binary64 rounding and is not a continuity certificate. Interior knots retain C0 multiplicity; evaluate one-sided trimmed jets at joins. Overshoot, stationary points and self-intersections are possible. Invalid counts, nonfinite input, collapsed parameter intervals and overflow are refused. Cancellation noise in inferred tangents may round away during control construction; this is not a continuity error certificate. JSON: `curve_natural_spline`; Rush: `natural_spline_curve`.

`natural_spline::clamped(points,parameters,start_tangent,end_tangent)` instead specifies endpoint dP/dt tangents. Interior cubic moments solve the clamped boundary system. Output endpoint dP/du scales by the full authored parameter span. Nonfinite, overflowing or underflowing normalized endpoint tangents are refused. Interior C2 is a real-arithmetic property, not a binary64 continuity certificate; overshoot, stationary points and self-intersections remain possible. JSON: `curve_clamped_spline`; Rush: `clamped_spline_curve`.

### Closed cubic interpolation

`closed_spline::interpolate(points,parameters)` solves cyclic cubic moments for 4..86 sites including an exactly repeated endpoint. Real-arithmetic C2 includes the closing seam. The returned [0,1] curve has clamped knots and `periodic:false`; geometric closure does not imply periodic knot encoding. No snapping or regularity/injectivity certificate is supplied. Cancellation roundoff in internally inferred tangents may round away during control construction; authored Hermite precision guards remain strict. One-sided trimmed jets are required at joins and seam. JSON: `curve_closed_spline`; Rush: `closed_spline_curve`.

### Bicubic corner Hermite patches

`hermite_patch::patch(corners,tangent_u,tangent_v,twist)` constructs a bicubic patch from corner positions, u/v first derivatives and uv mixed derivatives. Each 2×2 array is indexed [u][v] on [0,1]², with model length units for all jets. Construction forms 16 unit-weight Bezier controls algebraically; no sampled fit is used. Nonfinite or inadmissible controls and some rounded-away nonzero authored conditions are refused. Precision guards are conservative checks, not a complete derivative error bound. Degenerate patches are permitted; regularity/injectivity are not certified. Native `Evaluation::second_derivatives()` exposes (uu,uv,vv) when defined. JSON: `surface_hermite_patch`; Rush: `hermite_patch`.

### Natural tensor cubic grid interpolation

`grid_spline::interpolate(points,parameters_u,parameters_v)` interpolates rectangular 2..11 site grids per axis, indexed [u][v]. Parameters are dimensionless and strictly increasing; output domains are normalized [0,1]². Natural cubics are constructed along U and then along V for each intermediate control, producing degree3×3 with at most31×31 unit-weight controls. Real-arithmetic C2 and zero boundary second derivatives are distinct from binary64 certification. Natural curve precision refusals apply; overshoot, degeneracy and self-intersection are possible. Interior C0-multiplicity knots require one-sided trimmed jets. This handles ordered rectangular sites, not an unstructured cloud or guided loft. JSON: `surface_grid_spline`; Rush: `grid_spline_surface`.

Natural interpolation uses internally inferred Hermite tangents: tiny cancellation noise may round away while forming controls. Clamped interpolation applies strict authored-tangent checks at the actual first/last span intervals before materializing the inferred interior. Direct authored Hermite constraints retain all precision guards. None of these checks certifies the continuous rounding error or regularity of an inferred spline.

### Guided rational loft

`guided_loft::interpolate(sections,parameters,guides,guide_parameters,tangents)` retains 2..86 nonperiodic 3D sections and 1..86 complete guides through a homogeneous base+guide-loft−base-isocurve-loft correction. U guide stations are authored in [0,1]. A single guide may be interior; multiple interior guides are supported by retaining missing U=0/U=1 base isocurves. The effective count including retained boundaries must not exceed 86. Section stations and curve domains normalize V. Homogeneous crossings, including weight scales, must match exactly. Optional paired tangent controls use the aligned section U basis; guides must match base endpoint homogeneous derivatives exactly. This conservative binary64 equality can refuse mathematically compatible constraints after different rounding. Intermediate/final weights must be positive; aligned surfaces have at most256 controls per axis. No intersection search, snapping, automatic orientation, continuous error, regularity or solid certificate. JSON: `surface_guided_loft` with `curves`, `parameters`, `guides`, `guide_parameters`, optional paired `start_tangents`/`end_tangents`. WASM/TypeScript/Rush integration is verified; guided, natural, clamped and closed examples have visual CPU-mesh checks; five loft fixtures pass independent OpenCascade STEP import, topology validity and sampled position comparisons. These are bounded fixture checks, not continuous certification.

### Natural cubic rational loft

`natural_loft::interpolate(sections,parameters)` aligns 2..86 3D rational sections and interpolates their homogeneous controls with natural cubics along normalized V. Section parameters are finite and strictly increasing. U uses the aligned profile domain; the surface budget is 256 controls per axis; 86 sections produce exactly 256 V controls (3n-2), and a 87th section is refused. Real-arithmetic C2 and natural endpoint conditions apply to homogeneous controls, not necessarily to Cartesian second derivatives with varying weights. Independent section weight scales affect the intermediate surface. Nonpositive interpolated control weights and unrepresentable homogeneous conversions are refused. Overshoot and self-intersections are possible; neither continuous rounding error nor regularity is certified. JSON: `surface_natural_loft` with `curves` and `parameters`. TypeScript: `naturalLoftNurbsCurves`; Rush: `natural_loft_surface` with positional section references and named dimensionless `parameters`. Visual CPU-mesh and sampled independent STEP checks for this loft are recorded in docs/design/progressive-loft-2026-10-01.md.

`natural_loft::clamped(sections,parameters,start_tangent,end_tangent)` instead constrains Cartesian endpoint dP/dt to one constant vector across each end section. It sets endpoint homogeneous W'=0 and H'=W*tangent, using clamped cubic solvers and strict authored-condition precision guards. Normalized V derivatives scale by the authored parameter span. It supports the same section limits and positive-weight restrictions. This is not an arbitrary U-dependent tangent field or automatic surface continuity matching. JSON: `surface_clamped_loft`; TypeScript: `clampedLoftNurbsCurves`; Rush: `clamped_loft_surface` with positional sections and named parameters/tangents. Visual CPU-mesh and sampled independent STEP checks for this loft are recorded in docs/design/progressive-loft-2026-10-01.md.

`natural_loft::clamped_control_tangents(sections,parameters,start,end)` accepts a tangent control for each aligned U control at each endpoint. With endpoint weights `w_i` and basis functions `N_i`, Cartesian dP/dt is `sum(N_i*w_i*t_i)/sum(N_i*w_i)` and homogeneous W'=0. Normalized V derivatives scale by the station span. Tangent arrays must match the basis after section alignment, and finite/representability and positive-weight guards apply. This provides a varying endpoint derivative field, not automatic G1/G2 matching. JSON: `surface_control_tangent_loft` with `curves`, `parameters`, `start_tangents`, `end_tangents`. TypeScript/Rush integration and WASM publication are verified by tests/nurbsGuidedLoft.test.ts.

`natural_loft::closed(sections,parameters)` interpolates a cyclic list of 4..86 rational sections including the repeated endpoint. First and last aligned homogeneous controls must match exactly, including weight scales. Cyclic cubic interpolation gives real-arithmetic C2 across the seam with the same positive-weight and control budgets. Output uses clamped knots and `periodic_v:false`; parameter wrapping is not supported. Closure does not certify regularity, injectivity or solid topology. JSON: `surface_closed_loft`; TypeScript: `closedLoftNurbsCurves`; Rush: `closed_loft_surface` with positional cyclic sections and named parameters. Visual CPU-mesh and sampled independent STEP checks for this loft are recorded in docs/design/progressive-loft-2026-10-01.md.

### Rational Gordon networks

`gordon::patch(u_curves,v_curves,parameters_u,parameters_v)` combines two natural homogeneous lofts minus a natural homogeneous intersection grid. Each family has 2..11 nonperiodic 3D curves; U curves occupy V stations and V curves occupy U stations. Curves and finite increasing stations normalize to [0,1]. Computed homogeneous crossings must match exactly, including their weight functions; no snapping, reorientation, weight normalization or intersection-parameter search is performed. Real-arithmetic construction preserves the complete curve network. Intermediate and final control weights must be positive, and aligned surfaces remain within 32 controls per axis. Rounding error, regularity and injectivity are not certified. JSON: `surface_gordon`; TypeScript: `gordonNurbsSurface`; Rush: `gordon_surface` with named curve families and station arrays. Visual and STEP qualification remain pending.

### Three-boundary rational patches

`triangular_patch::patch(base,side_a,side_b)` accepts nonperiodic 3D curves oriented A->B, A->C, B->C with exactly coincident endpoints. It constructs a weighted collapsed C->C boundary compatible with the side endpoint weights, then builds a homogeneous Coons patch. Normalized V=1 is an intentional singular apex with undefined normal. Coons control budgets and positive-weight restrictions apply. Boundary rounding error, regularity, injectivity and solid topology are not certified. JSON: `surface_triangular_patch`; TypeScript: `triangularNurbsPatch`; Rush: `triangular_patch` with three positional curves. Visual and STEP qualification remain pending.

`boundary_fill::fan(boundaries,center)` constructs one rational triangular patch per edge of an exact oriented cycle of 3..32 nonperiodic 3D curves. Unit-weight spokes preserve identical Cartesian parameterization on neighboring patches and real-arithmetic G0. The supplied finite center may be nonplanar; V=1 is a singular apex for every patch. This does not certify G1/G2 seams, a simple contour, interior center placement, nonoverlap or solid topology. Hole loops and multiple components are outside this contract. JSON: `surface_boundary_fill`, returning every surface in an array; TypeScript: `boundaryFillNurbsSurfaces`; Rush: `boundary_fill` with positional edges and named center. Generic patch sets retain patches and faceIds without fabricated reconstruction metrics. Visual and STEP qualification remain pending.

### Rational scale sweep

`scaled_sweep::sweep(profile,path,scale,origin)` constructs `origin+C(v)-C(0)+r(v)*(P(u)-origin)` with fixed profile orientation. Scale controls are positive dimensionless `[r,0,0]`; path/scale domains normalize independently to [0,1] and use clamped nonperiodic encoding. Bernstein products on the union of path/scale spans construct the rational surface algebraically, without sampled fitting. Degree V is the sum of input degrees (at most25), with at most32 controls per surface axis. Profile U retains its domain. Positive-weight, finite-control and conservative displacement/underflow checks apply, including exact binary64 shared-span endpoints. Interior V knots encode C0; higher actual smoothness and continuous rounding error are not certified. Twist, RMF orientation, V closure, injectivity and solid topology are outside this contract. JSON: `surface_scaled_sweep`; packaged graph and editor entrypoint tested; visual/STEP qualification pending.

`two_guide_sweep::sweep(profile,guide_a,guide_b,width,axis_y,axis_z)` constructs
`A(v)+(x(u)/width)*(B(v)-A(v))+y(u)*axis_y+z(u)*axis_z` from local 3D profile
coordinates. Width is positive and uses model lengths; the authored transverse
axes are finite independent dimensionless vectors, without automatic normalization.
Guides normalize independently to [0,1] and use nonperiodic clamped encoding;
profile U domain and periodic flag are retained. Bernstein products on the union
of guide spans preserve rational inputs algebraically, with degree V at most25
and at most32 controls per axis. Interior V knots encode C0 and shared span
controls must match in binary64. Positive weights and finite controls are checked;
continuous rounding, orthogonal/RMF orientation, profile length preservation,
regularity, nonintersection and solid topology are not certified. JSON operation:
`surface_two_guide_sweep`; packaged geometry, graph, Rush and editor entrypoint tested; visual/STEP qualification pending.

`helix::approximate(center,radius,height,turns,phase_degrees,max_deviation)`
returns a cubic Hermite curve and its approximation report for a Z-axis
cylindrical helix. Radius/budget are positive; turns is nonzero and may be
negative. Up to85 uniform angular spans use the ideal remainder estimate
`sqrt(2)*radius*(abs(2*pi*turns)/spans)^4/384`. This estimate excludes binary64
trigonometric, control and evaluation rounding: the report explicitly declares
`continuousBound:false` and `roundingCertified:false`. Denser requests beyond
the256-control curve budget are refused; downstream single surfaces retain
their32-control axis limit. JSON: `curve_helix`; packaged geometry, Rush,
editor entrypoint and JSON/OBJ/PLY tested. Variable pitch, arbitrary axis and a rounding-inclusive
error certificate remain separate work.

`helix::approximate_elliptic` uses positive XY radii;
`helix::approximate_conical` uses a radius linear in normalized t with
nonnegative endpoints, at least one positive. Both share the cubic Hermite
generator and explicit uncertified approximation report above. The conical
fourth-derivative estimate includes the radius derivative; a zero endpoint
is allowed without a regularity certificate. JSON operations:
`curve_elliptic_helix`, `curve_conical_helix`. Packaged WASM/TypeScript/Rush,
editor entrypoint and JSON/OBJ/PLY tested; visual/STEP qualification pending.

`helix::approximate_variable_pitch` adds a constant-radius helix with a cubic
axial law constrained by total height and two signed endpoint pitches (length
per signed revolution). Its endpoint axial derivatives in normalized t equal
turns times pitch. Cubic Z uses polynomial blossom controls in the existing
unit-weight spline basis; the XY ideal remainder estimate is unchanged.
Interior reversal is allowed. JSON: `curve_variable_pitch_helix`. Native
sample/endpoint/report and packaged WASM/TypeScript/Rush/editor entrypoint
plus JSON/OBJ/PLY tests passed. Visual/STEP qualification remains pending. Rounding-inclusive error and monotonicity are not certified.

`ss_intersection::intersect_surface_surface_report` returns a native
`SurfaceSurfaceIntersection`: exact branches, continuation branches, tangencies,
overlap regions and unresolved parameter boxes. Junction detection and coverage
classification operate on these structures. The existing
`intersect_surface_surface` adapter preserves the JSON report schema.
Run `cargo run -p nurbs-core --no-default-features --example typed_surface_intersection`
from the workspace for a direct example of affine intersections and overlaps.

The `continuity::match_surface_jets_report`, `match_surface_jets_oriented_report`
and `match_surface_jets_checked_report` APIs return native `SurfaceJetMatch`
results. Regularity certificates, continuous error bounds, orientation mapping
and acceptance decisions use typed fields. The original API names remain JSON
adapters. Run `cargo run -p nurbs-core --no-default-features --example typed_surface_jets`
from the workspace for checked matching and reversed seam matching.

`continuity::preparation::{certify_report, checked_report, checked_with_conversion_report}`
exposes native seam preparation certificates and accepted surface pairs.
`continuity::curve_match::checked_report` exposes native G1 endpoint matching.
All continuity computations use Rust data; JSON compatibility is implemented
in their serialization modules.

`foundation::{certify_curve_report, certify_surface_report}` returns native
convex hull bounds, denominator bounds, regularity classifications and recursive
singularity localization. The existing certificate APIs encode these results
through `foundation::certificates::serialization`. Run
`cargo run -p nurbs-core --no-default-features --example typed_certificates`
from the workspace for direct curve and surface certification.

`foundation::ParameterMapping` and `MapPiece` represent rational parameter maps
without JSON. `certify_reparameterization_report`,
`evaluate_reparameterized_curve_report` and `materialize_reparameterized_curve_report`
return native monotonicity proofs, evaluations and materialized curves.
Legacy decoding and report encoding live in `foundation::parameter_mapping::serialization`.
Run `cargo run -p nurbs-core --no-default-features --example typed_parameter_mapping`
from the workspace for a nonlinear map and its materialized curve.

`foundation::fitting` contains native point fitting and cloud fitting.
`fit_curve_points_report`, `interpolate_surface_grid_report`,
`fit_curve_cloud_certified_report` and `fit_surface_cloud_certified_report`
return typed geometry and certificates; the original names remain JSON adapters.
Run `cargo run -p nurbs-core --no-default-features --example typed_fitting`
from the workspace for direct curve and surface fitting.

`foundation::periodic_edits` exposes native seam continuity evidence and periodic
curve/surface edits. `edit_periodic_curve_report`, `edit_periodic_surface_report`
and `split_periodic_curve_report` use `PeriodicEditOperation` and typed results.
The original API names remain compatibility adapters.

`foundation::approximate_edits` exposes native decisions and rollback results for
knot removal, degree reduction and curve/surface rebuilds. The `_report` APIs
return geometry and an `EditCertificate`; JSON reports are compatibility adapters.
`foundation.rs` now contains native helpers and module exports; projection adapters
and host evidence live in its serialization module, and tests live under `foundation/tests`.

Trim-domain classification uses native `ClassificationReason` values. Curve,
surface and trimmed-surface distance reports share `DistanceStopReason`; their
JSON encoders live in separate serialization modules and preserve the host labels.
BRep shell distance checks the native `EmptyDomain` reason directly.

`involute::approximate` constructs a cubic Hermite XY base-circle involute
from increasing radian angles, with up to85 spans/256 controls. The analytical
normalized tangent vanishes at zero angle; regularity is not certified there.
The ideal fourth-derivative remainder excludes binary64 rounding. JSON
`curve_involute` retains the uncertified report. Native and packaged WASM/TypeScript/Rush/editor entrypoint plus JSON/OBJ/PLY
tests passed; visual/STEP qualification remains pending. A full tooth profile and
qualified gear solid remain separate work.

`logarithmic_spiral::approximate(center,radius,growth,start,end,budget)`
uses radius*exp(growth*(theta-start)) in XY over increasing radian angles.
Growth is signed per radian; zero gives the circular limit. Cubic Hermite
uses analytical derivatives and up to85 spans/256 controls, with an explicit
ideal fourth-derivative remainder excluding binary64 rounding. Radius
overflow/underflow and unattainable budgets are refused. JSON operation:
`curve_logarithmic_spiral`. Packaged WASM/TypeScript/Rush/editor entrypoint and JSON/OBJ/PLY tests
passed; visual/STEP and rounding-inclusive error qualification remain pending.

`lissajous::approximate(center,amplitudes,frequencies,phases,budget)` creates
three-axis harmonic trajectories with analytical derivatives. Amplitudes
are nonnegative lengths, frequencies signed cycles over normalized t, phases
degrees. At least one axis must vary. Up to85 cubic spans/256 controls; the
ideal fourth-derivative estimate excludes binary64 rounding and no exact
closure/periodicity is asserted. JSON: `curve_lissajous`. Packaged WASM/TypeScript/Rush/editor entrypoint and JSON/OBJ/PLY tests
passed; visual/STEP and rounding-inclusive error qualification remain pending.

`trochoid::approximate(center,rolling_radius,tracing_radius,start,end,budget)`
uses [r*theta-d*sin(theta),r-d*cos(theta),0] in XY;
`approximate_cycloid` sets d=r. Angles are increasing radians; r is positive
and d nonnegative. Curtate, cusp-bearing cycloid, prolate loops and the d=0
linear limit share analytical derivatives and the ideal cubic Hermite
fourth-derivative estimate, excluding binary64 rounding. No regularity at
cusps or topology certificate is asserted. JSON: `curve_trochoid` and
`curve_cycloid`. Packaged WASM/TypeScript/Rush/editor entrypoint and JSON/OBJ/PLY tests
passed; visual/STEP and rounding-inclusive error qualification remain pending.

`circular_rolling::approximate_epicycloid` and `approximate_hypocycloid`
construct outside/inside rolling-circle trajectories in XY from fixed and
rolling radii plus increasing radian angles. Both radii are positive; inside
rolling requires the fixed radius to exceed the rolling radius. Analytical
derivatives and up to85 cubic spans/256 controls use an explicit ideal
fourth-derivative estimate, excluding binary64 rounding. Cusps and exact
closure/periodicity are not certified. JSON: `curve_epicycloid` and
`curve_hypocycloid`. Packaged WASM/TypeScript/Rush/editor entrypoint and JSON/OBJ/PLY tests
passed; visual/STEP and rounding-inclusive error qualification remain pending.

`archimedean_spiral::approximate(center,start_radius,end_radius,
start_degrees,end_degrees,budget)` constructs a planar spiral with radius
linear in angle, sharing the conical helix generator with zero height.
Angles strictly increase, endpoint radii are nonnegative and at least one
is positive. Up to85 cubic spans/256 controls with explicit ideal remainder
excluding binary64 rounding. JSON: `curve_archimedean_spiral`. Packaged WASM/TypeScript/Rush/editor entrypoint and JSON/OBJ/PLY tests
passed; visual/STEP and rounding-inclusive error qualification remain pending.

`catenary::approximate(center,scale,start_x,end_x,budget)` creates an XY
catenary with its vertex at center. Scale is positive and local x bounds
increase. Stable height uses `2*scale*(sinh(x/(2*scale)))^2`; analytical
derivatives and up to85 cubic spans/256 controls use a uniform-abscissa
ideal remainder excluding binary64 rounding. It does not solve chain
length/tension/support constraints. JSON: `curve_catenary`; packaged WASM/TypeScript/Rush/editor entrypoint
and JSON/OBJ/PLY tests passed. Visual/STEP and rounding-inclusive error
qualification remain pending.

`catenoid::approximate(center,scale,start_z,end_z,budget)` revolves a cubic
Hermite catenary radial profile about Z. Radius is scale*cosh(local_z/scale);
positive scale, increasing local Z bounds and positive radial controls are
required. Circular sections use rational revolution and matching seam
controls with periodic_v=false; the ideal profile approximation report
remains uncertified. A single surface retains32-control axis budgets;
denser requests refuse without relaxing precision. JSON: `surface_catenoid`.
Packaged geometry, graph/Rush, editor entrypoint and JSON/OBJ/PLY tests passed.
Visual/STEP and rounding-inclusive error qualification remain pending.

`helicoid::approximate(center,inner_radius,outer_radius,height,turns,phase_degrees,budget)`
constructs a radial ruled Hermite helicoid about Z with linear angular and
axial advance. Radii obey 0 <= inner < outer, height and turns are nonzero.
The outer helix provides common knots and the real-arithmetic remainder;
inner XY controls scale down while Z is shared. Domains are [0,1], no
periodic wrapping. A single surface allows 32 angular controls; dense
requests refuse without relaxing the budget. Rounding-inclusive error and
solid topology are not certified. JSON: `surface_helicoid`. Packaged
geometry/language/export tests passed; visual/STEP and rounding-inclusive
error qualification remain pending.

`toroidal_spiral::approximate` follows signed fractional major/minor turns on
an authored ring torus R>r>0, with separate angular phases. A product-to-sum
harmonic bound chooses up to 85 cubic Hermite spans without relaxing the
budget. `approximate_knot` accepts coprime positive integers p,q in 2..32
and shares closing endpoint samples; clamped encoding remains nonperiodic.
The ideal remainder excludes binary64 rounding; neither continuous error
nor knot topology is certified. JSON: `curve_toroidal_spiral`,
`curve_torus_knot`. Packaged WASM/TypeScript/Rush/JSON/OBJ/PLY integration tests passed.
Dense full-knot multipatch extrusion is integrated without relaxing the fit budget.
Visual/STEP and rounding-inclusive error qualification remain pending.

`extrusion_patches::extrude(curve,vector)` extends dense curve extrusion
without resampling or changing the authored fit budget. Small profiles
retain one surface; dense nonperiodic 3D curves split at existing knot
spans through knot insertion and retain their original U subdomains.
Returns up to256 surfaces, each within the existing32-control surface
budget. Patches are not sewn into a B-rep; continuous binary64 rounding
and topology are uncertified. JSON: `surface_extrude_patches`.
WASM/TypeScript/Rush/editor entrypoint/JSON/OBJ/PLY integration tests passed.
Visual/STEP, sewn topology and continuous rounding-inclusive error remain unqualified.

`spherical_spiral::approximate(center,radius,longitude_turns,latitude_turns,
longitude_phase_degrees,latitude_phase_degrees,budget)` approximates the
Z-axis longitude/latitude trajectory on a sphere. Both angular rates are
nonzero signed turns; pole crossings and fractional turns are supported.
An analytic harmonic fourth-derivative estimate selects at most85 cubic
Hermite spans. Binary64 rounding and exact sphere membership between fit
stations are uncertified. This is not a geodesic/equal-area spacing solver.
JSON: `curve_spherical_spiral`; TS/Rush `spherical_spiral_curve` integration
is complete; packaged geometry/Rush/editor entrypoint/JSON/OBJ/PLY tests passed.
Visual/STEP and continuous rounding-inclusive error qualification remain pending.

`clothoid::approximate(center,length,start_curvature,end_curvature,
phase_degrees,budget)` constructs a planar linear-arc-length-curvature
trajectory. Curvatures use inverse model lengths, phase degrees. Composite
Simpson integration and cubic Hermite fitting share the authored budget;
both ideal error contributions are reported separately and summed.
Limits:4096 integration intervals,85 cubic spans; precision requests
exceeding these limits refuse. Straight/circle limits are supported.
Binary64 rounding is excluded; continuousBound/roundingCertified remain
false. JSON: `curve_clothoid`. WASM/TypeScript/Rush/editor entrypoint/JSON/OBJ/PLY
integration tests passed. Visual/STEP and continuous rounding-inclusive
qualification remain pending.

`screw_surface::approximate(profile,origin,axis,height,turns,phase_degrees,budget)`
constructs approximate constant-pitch screw motion of a retained rational
profile about an arbitrary axis. Both angular and axial advance are linear
in normalized V. Signed height/turns and zero-height rotation are supported.
The positive-weight profile radial hull chooses a common helix Hermite fit;
tensor weights retain profile rationality. Dense profiles/motions split at
existing knots into at most2048 patches with original U/V subdomains.
Rounding-inclusive error and solid sewing are uncertified. JSON: `surface_screw`.
WASM/TypeScript/Rush/editor entrypoint/JSON/OBJ/PLY tests passed.
Visual/STEP and continuous rounding-inclusive qualification remain pending.

`catenoid::approximate_patches` extends dense catenoid construction without
refitting or relaxing the budget. It shares the single-surface radial
profile builder, decomposes dense profiles at existing knots and revolves
each retained span through a full rational circle. Up to85 patches preserve
original U subdomains and V=[0,4]. No sewn topology or rounding-inclusive
error certification. JSON: `surface_catenoid_patches`; WASM/TypeScript/
Rush/editor entrypoint/JSON/OBJ/PLY tests passed. Visual/STEP and continuous
rounding-inclusive error remain unqualified.

`pipe::checked(path,radius,initial_normal,sections,max_deviation)` creates
normal rational circular sections along a3D path through the shared
rotation-minimizing sampled sweep. Radius is constant at authored stations;
between them a linear rational loft does not preserve exact pipe geometry.
Shared refinement diagnostics are sampled, not a continuous tolerance/RMF
certificate. Rejected budgets return surface:null, never relax tolerance.
Open/closed regular paths are bounded by2..32 sections. JSON: `surface_pipe`.
WASM/TypeScript/Rush, programmatic editor entrypoint and JSON/OBJ/PLY regression passed (407 tests in23files,80fixtures). Continuous constant-radius/RMF, rounding-inclusive error, actual visual review and STEP qualification remain pending.

`pipe::checked_variable(path,radius,normal,sections,budget)` uses a positive
rational scalar radius law encoded as `[r,0,0]` controls. Path/law domains
normalize independently; this is parameter-based, not arc-length-based.
Circular authored sections use shared double-reflection frames; closed paths
require identical radius endpoints. Linear loft and fourfold sampled refinement
retain `continuousBound:false`; failed budgets return no surface. JSON:
`surface_variable_pipe`; TypeScript: `checkedVariablePipeNurbsSurface`; Rush:
`path.variable_pipe_surface(radius_law:curve,normal:...,sections:...,max_deviation:...)`.
Packaged graph, programmatic editor entrypoint,82fixtures and JSON/OBJ/PLY
regression passed (419tests in24files). Continuous variable-radius/RMF,
rounding-inclusive error, actual visual review and STEP remain unqualified.

`helicoid::approximate_patches` lifts the single-surface32-control limit by
splitting dense angular curves at existing knots into at most85 retained patches,
with original V subdomains and U=[0,1]. The shared Hermite fit and budget are
unchanged; small curves keep one surface. JSON `surface_helicoid_patches`;
TypeScript `approximateHelicoidNurbsPatches`; Rush `helicoid_patches` plus
`nurbs_patches_tessellate`. Full native363tests/2doctests, typecheck and packaged
425tests/24files passed, including83Rush fixtures and JSON/OBJ/PLY regressions.
Knot-insertion rounding, actual visual review and sewn STEP topology remain
unqualified; reports retain continuousBound:false and roundingCertified:false.

`ribbon::checked(path,width,normal,sections,budget)` transports a symmetric
unit-width line profile using the shared sampled frames and positive rational
width law `[width,0,0]`. Initial width direction is projected perpendicular to
the start tangent; both law/path domains normalize independently. Closed paths
require equal endpoint widths and retain a C0 cyclic seam. Authored section
widths are preserved, while intermediate linear loft geometry is approximate.
JSON `surface_ribbon`; sampled budget failure returns no surface. Full native
366tests/2doctests and3focused ribbon tests passed. WASM/TypeScript/Rush,
programmatic editor entrypoint and JSON/OBJ/PLY regressions passed:437tests
in25files with85Rush fixtures. Continuous width/RMF, rounding-inclusive error,
actual visual review and STEP qualification remain pending.

`circle_transition::ruled(start,end)` takes two `CircleSection` values with
center, normal, seam direction and positive radius. Seam directions project into
the circle planes and explicitly choose point correspondence. Shared rational
circle weights make the ruled transition linear in V, with degreeU2/degreeV1,
9x2 controls and domains [0,1]. Coincident control profiles are rejected.
This is a G0 ruled transition; no G1/G2 blend, regularity or sewn solid guarantee.
JSON `surface_circle_transition` uses start/end center,normal,seam,radius fields.
Independent quadrant equations and full native369tests/2doctests passed.
WASM/TypeScript/Rush, programmatic editor entrypoint and JSON/OBJ/PLY passed:
444tests/26files with86Rush fixtures. Rounding-inclusive error, actual
visual review and STEP qualification remain pending.

`ellipse_transition::ruled(start,end)` takes two `EllipseSection` values with
center and length-valued axis_u/axis_v vectors. Independent skew axes are
supported under existing ellipse primitive numerical admission. Identical
rational angular weights preserve both complete ellipse boundaries and linear
Cartesian V interpolation with9x2 controls (degreeU2/degreeV1, domains[0,1]).
Coincident profiles are refused. JSON `surface_ellipse_transition`; TypeScript
`ellipseTransitionNurbsSurface`; Rush `ellipse_transition_surface`. Native
371tests/2doctests, typecheck and packaged450tests/27files passed, including
87Rush fixtures, programmatic editor entrypoint and JSON/OBJ/PLY regression.
G1/G2 blend, continuous rounding-inclusive error, regularity, actual visual
review and sewn STEP topology remain unqualified.

`circle_rectangle_transition::ruled(circle,rectangle)` constructs four retained
patches with degreeU3/degreeV1,4x2 controls each, preserving circle quarter U
subdomains and V=[0,1]. Rectangle orthogonal half-edge vectors choose corner
correspondence; the circle has explicit normal and seam direction. Shared
Bernstein weight multiplication preserves linear Cartesian V interpolation
without fitting. Adjacent G0 seams retain endpoint profiles; patches are not
sewn solids. JSON `surface_circle_rectangle_transition` returns all surfaces.
Full native373tests/2doctests passed. WASM/TypeScript/Rush, programmatic editor
entrypoint and JSON/OBJ/PLY passed:456tests/28files with88Rush fixtures.
Rounding-inclusive error, actual visual review and STEP remain pending.

`profile_sweep::checked(profile,path,scale,normal,sections,budget)` transports
arbitrary3D rational profiles with shared sampled RMF frames and a positive
dimensionless scale law encoded as [scale,0,0]. Profile is relative to the path
start; law/path domains normalize independently. Scale applies to the complete
profile, including tangent offsets; weights are retained. Closed paths require
equal scale endpoints and use C0 holonomy-corrected seams. Sampled fourfold
refinement retains continuousBound:false and rejects failed budgets. The shared
helper now lives in framed_sweep; pipe/ribbon behavior is preserved. JSON
`surface_profile_sweep`; native376tests/2doctests passed. WASM/TypeScript/Rush,
editor/examples/export, continuous RMF/scale, rounding-inclusive error, actual
visual and STEP qualification remain pending.

Native Rust usage does not require the optional codec or transport features.
`cargo run --manifest-path crates/Cargo.toml -p nurbs-core --no-default-features --example typed_nurbs_families`
checks seven constructor families and typed surface metadata directly.
`surface::Evaluation` exposes `curvatures`, `domains`, `derivative_status` and
`derivative_sides`, with public status/side enums. `affine::curve`, `surface`
and `patches` accept a native4x4 matrix and retain rational weights/basis.
Viewer control-coordinate transformations delegate to these Rust APIs.
App bindings are separate from native implementation completion; the1200-family
goal and rounding-inclusive geometric qualification remain incomplete.

### Native parameter and surface edits

`surface_edit::{insert, elevate, split, transpose, reverse, decompose}` provide typed tensor-product operations. `Curve::{insert, elevate, split, trim, reverse, decompose}`, `Surface::{trim, iso}` and `foundation::reparameterize_curve_report` work directly without JSON or viewer adapters. Split/trim/decomposition preserve original parameter intervals. Transposing or reversing one surface axis reverses the normal orientation. Distinct source knots may not collapse during affine parameter remapping.

Run `cargo run --manifest-path crates/Cargo.toml -p nurbs-core --no-default-features --example native_edits` from the repository root. Independent rational-equation tests are in `tests/native_edits.rs`. See [native editing contracts](../../docs/design/nurbs-native-edits.md) and [priority 200 registry](../../docs/design/nurbs-core-200.md). Finite sampled tests are not continuous binary64 error certificates.

### Native arc length and equal-distance stations

`curve_measure::{length, point_at_length, divide_by_length}` return typed length, inverse and station reports directly in Rust. Bounds use original-span interval jets and a trapezoid remainder with outward rounding; they include integration and binary64 arithmetic error for the stored rational definition. Check `within_tolerance`: work/precision exhaustion retains bounds and may return a partial station list. The work budget counts replaced cells and is global across inverse/division queries. Arc distance tolerance does not imply parameter precision near a stationary point.

Run `cargo run --manifest-path crates/Cargo.toml -p nurbs-core --no-default-features --example curve_measure`. See [measurement guarantees and limits](../../docs/design/nurbs-curve-measure.md). Tests use independent lengths/inverse equations for weighted lines, polylines, circles, a parabola, cusps and stationary geometry.

### Native surface area

`surface_measure::area(surface, tolerance, max_cells)` encloses parametrized area of the full original surface, including arithmetic/integration error. The absolute tolerance is in squared model units. Original homogeneous interval jets through total order three bound the Jacobian norm and a two-dimensional trapezoid remainder; singular cells retain general interval bounds. Inspect `within_tolerance` and `stop_reason`. Cell budgets include discarded subdivision trials. Folded or overlapping regions count with multiplicity; trim loops and union-of-images area are outside this contract.

Run `cargo run --manifest-path crates/Cargo.toml -p nurbs-core --no-default-features --example surface_measure`. See [area guarantees and independent checks](../../docs/design/nurbs-surface-area.md).

### Machining cross-sections

`engineering_profiles::{keyway, t_slot, dovetail}` construct closed degree-one XY cutter profiles, with depth toward -Y and the origin at the mouth center. They reject invalid dimensions and edges/areas lost at coordinate precision. These are authored shapes, without standards/fit claims. Use extrusion for a wall surface; end caps and solid subtraction are separate operations.

Run `cargo run --manifest-path crates/Cargo.toml -p nurbs-core --no-default-features --example machining_profiles`. [Contracts and independent geometric checks](../../docs/design/nurbs-machining-profiles.md).

### Native curve differential geometry

`curve_differential::at(curve, parameter, Side)` encloses curvature, signed torsion and Frenet-frame components directly from original interval jets. Select Left/Right at interior knots without proven C2/C3 continuity; Automatic natural endpoints use their inward side. Missing fields explicitly distinguish unproven continuity, speed not separated from zero and curvature not separated from zero. A line does not receive an invented torsion or normal. Normalizing before powers supports the tested tiny geometry scale 1e-110 without forming speed³.

Run `cargo run --manifest-path crates/Cargo.toml -p nurbs-core --no-default-features --example curve_differential`. [Contracts, units and independent tests](../../docs/design/nurbs-curve-differential.md).

`engineering_profiles::{square_fastener, hex_fastener}` construct wrench-sized
closed XY profiles. `poly_v_belt(center, ribs, pitch, back_thickness, rib_height)`
constructs a single closed curve for 1..126 triangular ribs and a flat back.
These are authored geometric dimensions, without manufacturing standard claims.
See [profile contracts](../../docs/design/nurbs-machining-profiles.md) and the
native `machining_profiles` example for measurement and extrusion.

`engineering_profiles::toothed_belt(center, ToothedBelt { ... })` constructs
a closed trapezoidal-tooth belt cross-section, with 1..62 teeth and explicit
pitch, base/tip widths, back thickness and tooth height. Feature collapse
at coordinate precision is rejected; belt-standard qualification is separate.

Use `Curve::bounds()` / `Surface::bounds()` for validated control-hull AABBs
and `bounds::union(&boxes)` for a validated union of 1..2048 boxes.
[Affine and bounds guarantees](../../docs/design/nurbs-affine-bounds.md)
include weighted, periodic and singular-map checks.

`engineering_profiles::o_ring_groove(mouth, width, depth, radius)` constructs
a closed cutter section with circular bottom corners and sharp mouth corners.
Run the native `o_ring_groove` example for certified measurement intervals
of the stored rational geometry. Seal sizing and fit qualification are separate.

`weight_edit::{curve,surface}` apply immutable absolute weight changes,
with periodic aliases updated together and duplicate logical edits rejected.
[Weight edit contracts](../../docs/design/nurbs-weight-edit.md) describe
conditioning limits and independent rational/periodic tests.

`sections::compatible(&curves)` normalizes active section domains, elevates
degrees and unifies exact knot multiplicities independently of loft.
`surface::loft_aligned` uses this shared native API.
[Compatibility contracts](../../docs/design/nurbs-section-compatibility.md)
cover resource limits and parameter/basis preservation.

`sections::orient(&curves, ambiguity)` matches endpoint directions relative
to the first section, reporting ambiguous matches without reversing them.
This is an endpoint heuristic; closed-section phase matching remains separate.


### Progressive profile sweep

`progressive_sweep::Sweep` is a resumable level iterator combining rational profile,
positive scalar scale and signed angular twist. Orientation supports RMF, fixed
world orientation, fixed normal projection and Frenet; undefined frames fail.
Laws use normalized parameter traversal or inverse arc length with an explicit
residual/work budget. `approximate` doubles intervals to the maximum section
count and returns final patches only when fourfold sampled control refinement
meets the unchanged budget. Unaccepted levels remain available as Rust previews.
Dense nonperiodic profiles retain knot-span patches, and long sweeps split V into
patches of at most32 controls with their original normalized traversal subdomains.
Closed RMF distributes holonomy by sampled chord length; endpoint scale and
whole-turn twist must match. Seam is C0; continuous RMF error, rounding-inclusive
error, injectivity, cap/solid topology and STEP are not certified. JSON operation:
`surface_progressive_sweep`; graph/Rush: `progressive_sweep`, followed by
`nurbs_patches_tessellate`. Native twist laws use radians, host/Rush values degrees.
See `docs/design/sweep-coverage-2026-10-01.md` for the full unfinished scope.

Multiple profile boundaries: `progressive_sweep::MultiSweep` and
`approximate_profiles` refine 1..64 ordered curves on common stations with one
aggregate sampled budget. Direct JSON `surface_progressive_sweep_profiles`
returns `profilePatchRanges` (half-open patch indices); no accepted wall set is
returned if any boundary misses budget. Aggregate patch budget is4096. Rational
outer/inner contours and authored shared vertices are retained; nesting, caps,
sewing and solid certification remain separate operations.

`Sweep::with_affine_laws(axis_scale,center)` and the equivalent `MultiSweep`
builder add positive rational3-vector axis scales and local-frame center offsets.
Local coordinates become `uniform_scale * axis_scale * q + center`, then twist
and guide frame transport apply. Laws normalize their independent domains to
selected traversal. Reconfiguration restarts refinement. Closed seams require
matching vector law endpoints. `approximate_affine_profiles` runs shared levels.
These laws are available through TS options and Rush `axis_scale/center_law`
records, including `brep_progressive_sweep` with retained caps and holes.

### Automatic and matched loft extensions

`loft_alignment::interpolate` discovers unique resolved section/guide intersections, reverses and sorts guides, trims exterior portions and remaps crossing stations piecewise. It refuses ambiguous/overlapping/unresolved intersections, inconsistent U stations and incompatible weights. Whole-curve outward retention bounds include remapping and weight-scaling error. Piecewise remapping can introduce C0 knots. JSON `surface_auto_guided_loft`; TS `autoGuidedLoftNurbsCurves`; Rush `auto_guided_loft_surface`.

`loft_continuity::match_ends` applies certified scaled C1/C2 end jets (stronger than G1/G2 on regular seams), preserving complete authored sections/guides within an explicit budget. Cubic-or-higher normalized V is required. Simple distinct knots localize the edited end strips; final seams are reinspected after both matches. Basis, regularity, jet and retention gates must all pass. JSON `surface_loft_match_ends`; TS `matchNurbsLoftEnds`; Rush `loft_match_surface`.

Capped natural and supplied-patch loft bodies live in `brep-core`, with planar audited caps and validated shared topology. Contracts and qualification: `docs/design/advanced-loft-2026-10-02.md`.

### Complete authored sweep frames

`progressive_sweep::Sweep::new_authored` accepts rational 3-vector longitudinal
and transverse laws. Normalization/projection constructs a right-handed frame
independent of guide tangent; twist and affine laws use that frame.
`with_frame_laws` on Fixed sweeps (also MultiSweep) restarts refinement.
`approximate_authored_profiles` retains aggregate admission. JSON orientation
`authored` requires both `frame_axis` and `frame_normal` Curve payloads.
Zero/parallel sampled directions refuse; closed normalized endpoint frames
must agree. Continuous regularity and embedding remain uncertified.
The TypeScript surface API accepts `AuthoredProgressiveSweepOptions` with
required `frameAxis` and `frameNormal` vector laws. Rush surface sweep uses
`orientation:"authored"`, `frame_axis` and `frame_normal` records; values are
dimensionless. B-rep uses the same authored contract via
`progressive_authored_profile_body`; the authored hollow example has been
viewed in Solid. Continuous regularity and full visual qualification remain pending.

### Progressive orientation rail

`Sweep::with_orientation_guide` and `MultiSweep::with_orientation_guide`
project the direction toward a second spatial rail into the main path tangent
normal plane. Correspondence uses independent normalized parameter fractions
or independent inverse arc-length fractions on both curves. The reported
length residual includes both rails. Replacing the guide restarts refinement.
Fixed/authored frames cannot be combined with this mode. Coincident or tangent
rail directions refuse at sampled stations; closed endpoint normals must agree.
This controls orientation only. Contact/correspondence anchors and transverse
scaling onto the rail, continuous regularity, TS/Rush/body transport and visual
qualification are still pending.

Surface JSON exposes `orientation_guide` and TS exposes `orientationGuide` via
`GuidedProgressiveSweepOptions`. Both use aggregate progressive refinement
and preserve simultaneous affine laws. Fixed/authored conflicts refuse.
Rush/body transport, positional contact constraints and visual qualification
remain pending.

`Sweep::with_contact_guide(rail,profile_parameter)` fits the transverse
coordinate so a selected rational profile point follows the rail at retained
stations. Initial coincidence and normal-plane correspondence are required.
Nonzero twist and side/longitudinal center offsets conflict with contact and
refuse. Closed endpoint widths must agree. Other scale axes remain intact.
This is sampled station contact; continuous rail contact, shared multi-profile
anchors and transport/body/UI integration remain pending.

`MultiSweep::with_contact_guide(rail,profile_index,parameter)` uses one
reference profile anchor to define the same transverse fit for every contour.
`approximate_contact_profiles` combines it with affine laws and aggregate
admission. Replacing the anchor restarts refinement; invalid index/parameter
and initial coincidence failures refuse. Native shared-fit tests pass; contact
transport/body/UI and continuous-contact qualification remain pending.

Surface JSON contact uses `orientation_guide`, `contact_parameter` and optional
`contact_profile` (default0). An anchor without a rail or an index without
a parameter refuses. TS uses `contactAnchor:{profileIndex,parameter}` on
GuidedProgressiveSweepOptions. Parameters use the selected profile domain.
Contact body/Rush/UI integration and continuous contact certification remain pending.

`Sweep::preview_at` and `MultiSweep::preview_at` compute a bounded level
without advancing the iterator. The multi-profile iterator reuses the same
aggregate implementation. JSON level serialization marks `preview:true`;
unaccepted previews must not become construction results. Host dispatch,
worker streaming/cancellation and editor preview integration remain pending.

JSON `surface_progressive_sweep_level` and TS
`previewProgressiveNurbsProfiles` request one bounded aggregate preview with
ordered profiles and explicit section count. All authored/affine/guide/contact
options are retained. Unaccepted preview patches are available for display,
while admission remains false. Worker/editor streaming is still pending.


Progressive orientation `CorrectedFrenet` / `corrected_frenet` keeps the sampled
principal normal in the hemisphere of its transported predecessor. Zero
curvature or a nonunique second derivative uses double-reflection transport;
zero initial curvature is seeded from the first available principal normal,
transported backwards, or the authored normal when no principal normal exists.
Strict `Frenet` still refuses an undefined principal normal. This is sampled
continuation, not a continuous frame/regularity certificate. Closed corrected
frames must still agree at their endpoints; incompatible seams refuse.

`paths::round_polyline(points, radius)` constructs an open spatial path with 2..17 points. Each corner becomes a rational circular arc of the requested positive radius, joined to retained straight segments with geometric G1 continuity. The binary64 composed curve retains C0 knot multiplicities; trimmed one-sided jets are required at joins. Reversals, coincident endpoints, zero-length edges, and neighboring fillets that consume or overlap a segment are refused. Radius is never automatically reduced. This constructor does not certify global path intersections or swept-profile regularity.
Corners with computed turn angle at most 1e-12 radians are treated as straight; this is a numerical admission tolerance, not certified exact collinearity.

`paths::transition_polyline(points, setback)` replaces open spatial polyline corners by polynomial quintic Bezier blends. Both edges are trimmed by the requested positive setback. Controls encode equal endpoint speeds and zero second derivatives; speed-scaled composition matches position, tangent and zero curvature in real arithmetic. C0 knot multiplicities remain in the binary64 encoding, so use one-sided trimmed jets at joins. Setbacks are not reduced to fit overlapping neighbors. Turn angles at most 1e-12 radians are treated as straight. This construction does not certify global intersections or swept-profile regularity.

`engineering_profiles::retaining_ring(center, inner, outer, gap_degrees)`
constructs a single closed outline of an open annular ring, with circular
inner/outer arcs and radial ends. Installation lugs and fit standards are separate.

`coordinate_frame` normalizes 2D/3D control geometry around a validated
box center, restores world coordinates and explicitly scales units.
[Coordinate frame contracts](../../docs/design/nurbs-coordinate-frame.md)
cover finite scales, numeric limits and rounding limitations.

`paths::closed_round_polyline` and `paths::closed_transition_polyline` accept 3..16 cyclic sites without a repeated endpoint. Every corner, including the last/first adjacency, is joined; output is clamped nonperiodic NURBS with exactly matching endpoint controls. The seam starts at the exit of the final corner, with matching tangents (and zero-curvature joins for transitions in real arithmetic). TS constructors take a third `closed` boolean; Rush nodes take `closed: true`. Retained edges must exceed a 64-epsilon relative length margin to refuse numerically collapsed intervals. Closed sweeps still report only sampled refinement and C0 swept-wall seam continuity.

`conditioning::{curve,surface}` report weight ratios, relative active knot
spacing and coordinate-offset indicators. These are scale diagnostics,
not an evaluator condition-number or regularity certificate.
[Diagnostic contracts](../../docs/design/nurbs-conditioning.md).

`trim_domain::exact_loop_joins(&curves)` proves exact authored UV joins
for clamped nonperiodic ends, returns None for unproved ends, and bounds
the contour to 1..256 curves. [Closure contract](../../docs/design/nurbs-uv-closure.md).

`TrimDomain::classify_point([u,v], max_cells)` conservatively classifies
UV points in authored winding domains, including holes and nested islands.
Boundary/uncertainty bands remain Unresolved. [UV domain contract](../../docs/design/nurbs-uv-point.md).

`trim_simplicity::Report::outcome()` explains why a sufficient UV simplicity
proof succeeded or remains unproven. [Proof scope](../../docs/design/nurbs-uv-simplicity.md).

`paths::miter_sections(profiles, points, normal, miter_limit)` constructs retained sections for an open 2..17-site spatial polyline and 1..64 rational profiles. Profiles must lie in the initial normal plane (relative numerical tolerance 1e-10). Minimum rotations transport the transverse frame. Interior profile offsets q are projected along the incoming tangent a into the bisector plane n by `q - a*(q·n)/(a·n)`. The authored miter limit bounds `sec(turn/2)`; reversals and nonpositive longitudinal control advances refuse. Degrees, weights and knots are retained without refitting. Sections can be assembled with the rational section-loft B-rep constructor. Closed miter frames, scale/twist laws and global embedding/rounding certification remain pending.

`curve_join::inspect` checks geometric G0/G1/G2 endpoint continuity
using original interval jets, unit tangents and curvature vectors.
[Geometric join contracts](../../docs/design/nurbs-curve-join.md).

`paths::closed_miter_sections` constructs 3..16 cyclic-site extrusion miters without repeating the first input site. The authored profile is expressed in the plane normal to the first outgoing edge, centered at the first site; its initial miter section is projected into the closing-corner bisector. All corners, including the first, enforce the stretch limit and positive longitudinal control advances. Minimum-rotation frames must return to their initial orientation within 1e-10; incompatible holonomy refuses rather than silently stitching a rotated profile. Only after that check is the final section replaced by an exact copy of the first for shared seam incidence. TS/Rush miter APIs take `closed: true`; bodies use periodic section loft with inner shells and no caps. Distributed twist correction for nonzero holonomy remains pending.

`periodic_seam::{curve_at_knot,surface_at_knot}` rotate periodic storage
at an existing authored knot. [Supported seam scope](../../docs/design/nurbs-periodic-seam.md).

`progressive_miter::Sweep` extends spatial polyline miters with rational positive scalar scale and signed twist laws, parameterized by normalized polyline length. Closed frame holonomy is distributed by that length; closed scale endpoints must agree and twist endpoints differ by whole turns. Every path vertex remains a station at every level. Between corners, the rotated/scaled transverse offset is sheared by a linear blend of the adjacent bisector-plane projections; constant laws and zero correction recover extrusion miters. Profiles keep degrees, knots and weights.

Levels double per-edge steps and compare retained controls against fourfold finer analytic section samples. At most1025 retained sections,4097 probe stations and one million retained controls are admitted; finer rows are streamed rather than retained together. Preview sections remain available after a failed sampled budget, but `approximate` promotes only an accepted final section set. Longitudinal control advances are checked at probe stations. Acceptance additionally requires certified frame transport, an outward-rounded retained section-interpolation bound, whole-domain profile tangent regularity and retained-wall Jacobian regularity. Global embedding and full B-rep conversion/cap error remain unproved. TS exposes full/preview/async stream APIs and an accepted-section B-rep constructor with the1024-face budget. Rush and viewport carry the error/regularity evidence and refusal reasons; the complete visual matrix remains open.

Progressive miter acceptance also requires `phaseResolved`: on each retained interval, the trimmed rational twist control range plus the holonomy increment must not exceed pi/2. This prevents integer whole-turn aliasing of uniform probes from prematurely admitting a level. Collapsed phase-guard parameter intervals refuse. The local control-hull guard is not a rounding or continuous surface-error certificate.

Run `cargo run --locked --manifest-path crates/Cargo.toml -p nurbs-core --example progressive-miter-holonomy` for a closed skew-path example combining one full twist turn, a nonconstant positive scale law and distributed holonomy correction. It prints each sampled level and asserts the retained seam equality after admission.


Progressive miter levels also report `continuous_error_upper`: a real-arithmetic
linear-interpolation remainder estimate from trimmed rational scale/twist
first/second derivative hulls and affine miter shear. Internal law knots use a
Lipschitz bound; discontinuous laws are refused. This estimate remains diagnostic. Acceptance requires sampled error and
`certified_error_upper` within budget, resolved twist phase, certified frame
transport and profile/wall regularity. The outward certificate includes
authored law domains, holonomy, frame rotation, miter shear, stored endpoint
error and interpolation remainder across all profile parameters via unchanged
positive weights. Exhaustion returns no upper; generic full B-rep rounding
and embedding flags remain false pending conversion/cap/global geometry audits.
Rush exposes this as `brep_progressive_miter_sweep`, with display-only streamed
wall patches and retained-section B-rep construction after acceptance.


`progressive_miter::scalar_certificate::certify(curve, [lo, hi], max_cells)`
returns outward-rounded enclosures of a scalar law's value and first two
one-sided derivatives with respect to its authored knot parameter. Restriction
uses interval homogeneous blossoms from the original curve, not `Curve::trim`.
Reports are `Certified` or `Unresolved`; exhaustion and numeric enclosure failure
return no partial bounds. `single_span` must hold before using a second-order
interpolation remainder across the requested range. Positive weights and
continuous scalar laws are required. This certificate covers the law only, not
frames, retained stations, surface regularity or global embedding. Progressive
miter composes these law enclosures into its certified retained interpolation
error; they do not independently certify the complete B-rep or embedding.


`sweep_seam_audit::inspect` and the public `inspectSweepSeams` WASM adapter
inspect boundary jets without editing input surfaces. Reports keep regularity,
tangential smoothness, the continuous jet deviation upper and per-seam refusal
reasons. The seam-count budget never promotes uninspected seams. Explicit
nonzero jet tolerance is qualification, not exact G1/G2. Translation round
first-order and transition second-order fixtures are qualified; moving-frame
seams and the complete smoothness contract remain open. See the
[sweep completion matrix](../../docs/design/sweep-contract-matrix.md).

The read-only `sweep_wall_audit` JSON action accepts `walls`, sorted distinct
index pairs in `sharedBoundaries`, nonnegative `clearance`, positive
`distanceTolerance`, and three explicit budgets: `maxInjectivityCells`,
`maxPairs` (outer BVH node-pair visits), and `maxPairCells` (distance refinement).
Each budget is bounded by100000; zero yields unresolved evidence when work is
needed. The response preserves per-chart projection/reason data, C0 boundary
evidence and unresolved pair IDs. `chartsAndPairsCertified` covers retained
charts and all pairs only; `globalEmbeddingCertified` stays false because cap
ownership and shell containment are independent obligations.

`curve_progressive_miter_wall_audit` optionally accepts `loopSizes` to preserve
explicit profile contour ownership. A partition contains1..16 positive sizes
covering all profiles. Each retained section must close by exact clamped
endpoint control equality; a single validated periodic curve also defines a
closed loop. Profile-neighbor declarations stay within their loop. The audit
separately checks whole-boundary C0 and interior separation, and never infers
containment from the partition. Shared coordinate-plane certificates support
clamped tensor boundaries on either parameter axis with exactly matching
rational boundary data and strictly opposite remaining control hulls.

`sweep_contour_audit` accepts `loops` (outer first, then holes), `tolerance`,
`maxPairs` and `maxCells`. The planar-domain audit retains separate
`capDomainCertified`, `capGeometryCertified:false` and
`globalEmbeddingCertified:false` fields, work counts and a refusal reason.
Currently it requires a represented coordinate plane and simple rational
Bezier contour segments; general decomposition and oblique planes remain
unproved. Pair and cell budgets are shared across contour simplicity,
separation and winding work, with256-cell internal per-call refinement caps.

`sweep_cap_boundary_audit` reads a stored unit-domain bilinear `surface`,
a spatial `world` curve and its rational `uv` curve, plus `tolerance` and
`maxProducts`. It returns a whole-boundary deviation bound only after all
basis products are enclosed with directed rounding. Exhaustion yields
`errorUpper:null`; `capGeometryCertified` and `globalEmbeddingCertified`
remain false because boundary agreement alone does not prove either. The
rational bases/weights must match and UV control hulls must stay in[0,1]².

`sweep_cap_wall_audit` accepts a cap surface, wall surfaces, one optional
`uMin`/`uMax`/`vMin`/`vMax` boundary per wall, and `maxWalls`. It proves
coordinate-plane interior exclusion and preserves separated, boundary-restricted
and unresolved wall IDs. Boundary restriction does not establish ownership
of a cap trim, so `boundaryOwnershipCertified` and
`globalEmbeddingCertified` remain false. Budget exhaustion retains every
remaining wall as unresolved.

`sweep_coedge_agreement_audit` exposes the continuous native curve/surface
composition verifier for stored world edges and pcurves. Inputs are `surface`,
`world`, `uv`, `reversed`, positive `tolerance`, and `maxCells` in 0..100000.
The result distinguishes `within-tolerance`, `mismatch` (with a certified witness
distance enclosure), and `unresolved`. Zero cells validates inputs and returns
unresolved. Positive acceptance covers the entire normalized traversal using
outward correlated polynomial bounds or interval refinement, not samples.
Tolerance agreement does not prove exact identity or global embedding; both
corresponding certification flags remain false.

`sweep_coedge_exact_audit` checks exact rational Bezier composition identity of
stored binary64 coefficients under normalized traversal. Inputs are `surface`,
`world`, `uv`, `reversed` and `maxWork`. Results distinguish `equal`, `different`,
`unresolved` and `unsupported`, retain consumed work, and set
`exactIdentityCertified` only for equality. The transport bounds the request at
10000000 work units; the predicate's current hard limit is 1000000 (requests
above it remain unresolved). Unsupported span layouts or an unproved UV chart
inclusion never become equal. This predicate does not certify global embedding.

The bilinear `sweep_cap_boundary_audit` accepts arbitrary clamped U/V knot
domains. UV control inclusion is checked against those actual domains; all
normalization arithmetic uses outward intervals before the correlated rational
product bound. A [0,1] chart is not required.

`sweep_boundary_coverage_audit` accepts a surface, a 2D pcurve and one boundary
label (`uMin`, `uMax`, `vMin`, `vMax`). It proves complete coverage of that side
when the fixed coordinate equals the clamped surface endpoint, every varying
control coordinate lies in the other axis domain, and the clamped continuous
pcurve attains the two domain endpoints. Positive rational weights imply hull
inclusion; continuity and the intermediate value theorem imply complete image
coverage. Traversal may reverse or backtrack. Injectivity, trim ownership and
global embedding are separate and stay uncertified.
