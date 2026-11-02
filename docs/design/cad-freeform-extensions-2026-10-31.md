# Freeform CAD continuation

Implementation belongs to the native Rust geometry owners. No Plasticity code
or proprietary geometry runtime is copied or linked. The clean comparison
library remains frozen (11 files checked by `verify_clean.py`).

Further native topology work and the active parity ledger are recorded in
[the library gap plan](plasticity-library-gap-plan.md). It adds automatic
boundary-moving region replacement, closed curved-profile prism blends and
topological quad cycle/strip edits; these do not imply general solid parity.

## Retained quad geometry

`subdivision_core::quad_surface::QuadSurface` owns an immutable, oriented
manifold quad cage. It accepts irregular vertex valences, closed topology and
open boundaries. `with_vertex` returns a new retained definition.

`evaluate(face, uv, max_depth)` evaluates the Catmull–Clark limit geometry:
original vertices use the existing limit stencil, open boundary edges use their
cubic limit curve, and regular descendants resolve to actual bicubic patches.
`locate` returns that patch, local UV and the derivative of the local UV clock
with respect to original face coordinates. `polyhedral_spline::from_uniform_patch`
retains the extracted uniform basis as NURBS; it does not reinterpret uniform
control coefficients as clamped Bezier coefficients.

The current shared topology owner allows at most 5 refinement levels and
25,000 faces. Queries too close to extraordinary points can exhaust that
budget; they refuse. At the extraordinary vertex the position stencil remains
available, but no finite regular NURBS chart or G2 curvature is fabricated.
Nonmanifold cages and T-junction repair are outside this contract.

`quad_surface::editing` now additionally joins complete original boundary cycles
and deletes connected quad strips/rings, with full result manifold validation
and original-to-result maps. These are shape-changing edits; old-limit shape
preservation and extraordinary CAD patch coverage remain separate.

## Complete G2 boundary constraints

`constrained_surface::boundary::solve_preserving_g2` fixes three coefficient
layers on each selected clamped rational boundary. This retains complete
Cartesian two-jets in real arithmetic, including mixed and tangential
derivatives; it does not replace a boundary by point samples. Solver roundoff
in locked controls is removed and every requested interior constraint is
remeasured on the actual output. Fresh continuous G2/regularity checks must pass.

`solve_from_mapped_supports` constructs fields from four independent immutable
original support pcurves, solves interior constraints, then uses the existing
mapped-corner owner to freshly qualify the actual solved surface DIRECTLY on
the original support curves. Interior injectivity is required. The certificate
owns the solved surface privately; callers cannot mutate it while retaining the
certificate. Immutable copies of original supports/pcurves are retained and
`reverify` reruns admission against those exact saved originals.
Incompatible corner jets, insufficient work, or unresolved proofs
return errors. This extends the solver beyond preserving its own old boundary.

Fixed rational basis/weights and <=256 controls / <=384 scalar-vector input
constraints remain explicit solver limits. Automatic support/trim discovery
and unconstrained global shape optimization are separate problems.

## Freeform offsets

`offset::freeform::NormalOffset` retains the original surface and the expression
`S(u,v) + d*n(u,v)`. Position and first derivatives use the original analytic
first/second jets, including rational weights. No control-net offset or finite
difference normal substitutes for this retained geometry.

`qualify` adaptively bounds the original world curvature tensor over a UV
rectangle. The sufficient outward bound `|d| * ||shape||_F < 1` proves a positive
lower tangent stretch throughout that region. It proves local regularity of
the ideal offset map, not global injectivity, minimum wall thickness, or
absence of intersections between distant parts of the surface.

`approximate` creates a separate same-UV NURBS proposal through ordered 5/9/11
site grids and measures error at an independent 65x65 grid. Authored nonunit
UV domains are retained. This is sampled approximation evidence, not a
continuous NURBS offset-distance certificate. A focal crossing cannot receive
a regular-region certificate.

## Fillets and chamfers

`offset::freeform_blend::RollingBall` retains two original support charts and a
constant radius. A selected original coordinate drives the local offset/offset
intersection. Each query corrects the other three coordinates using analytic
offset Jacobians, pivoted Newton and bounded descent inside original domains.
The selected driver must be nonsingular even if the initial seed already solves
the residual. Explicit seeds select a local branch; global branch uniqueness is
not inferred.

The returned section retains original contact parameters, ball center and solve
residual. It evaluates the circle and creates a rational circular section.
`ContactChamfer` instead evaluates a ruled surface between arbitrary original
rational UV contact rails, with analytic first derivatives and normals. Rail
convex hulls must remain in their original charts. Contact selection/setback
rules are explicit inputs.

## B-rep entry points

| `brep_core::feature_ops::freeform` API | Result and evidence |
| --- | --- |
| `replace_constrained` | Actual solid face replacement with fixed original edges/vertices. Low-degree seed charts are elevated for interior freedom. Every actual outer/hole trim use needs fresh original-support G2, continuous boundary agreement, candidate chart injectivity and the existing solid incidence/ownership audit. |
| `sheet` | One validated open B-rep face from a nonperiodic natural chart. No volume body. |
| `offset_sheet` | Locally regular retained offset plus a validated open NURBS approximation accepted by sampled error. |
| `chamfer_sheet` | Ruled NURBS sheet between explicit original isoparametric contact traces; whole original-support boundary lift checks and chart injectivity are mandatory. |
| `fillet_sheet_proposal` | Open NURBS skin through retained rolling-ball sections. Its type preserves proposal status: radius/contact between stations are not certified. |

Separate closed-body entry points now exist in `feature_ops::retrim` and
`feature_ops::profile_blend`. The former moves a region boundary by automatically
trimming adjacent supports inside their original charts. The latter authors
convex rounds/chamfers on curved rational XY-prism profiles, including both end
caps, with actual full-side radius and whole-join G1 admission for rounds.

**Still open:** automatically discover all contact branches on arbitrary solid
faces, partition/retrim neighboring supports, build end caps and multi-edge
corners, self-trim offsets, and sew/admit a closed solid after a general radius
fillet or chamfer. Arbitrary boundary-moving face replacement also needs that
topology pipeline. Open sheets and sampled fits do not close this scope.
STEP/WASM/UI and full application qualification are not claimed.

## Verification

```sh
cargo test --manifest-path crates/Cargo.toml \
  -p nurbs-core --test freeform_geometry --test boundary_constraints \
  --test constrained_surface --test polyhedral_spline \
  --test corner_mapped --test surface_offset_assurance \
  -p subdivision-core --test quad_surface \
  -p brep-core --test freeform_features --test feature_ops --test face_replacement
python3 crates/independent-modeling/verify_clean.py
node scripts/audit-nurbs-catalog.mjs
```

Cases include valence 3/5, two charts of shared quad edges, open boundary limit
curves, singular driver refusal, rational offset derivatives, nonunit UV clocks,
focal crossing refusal, full original noniso support G2 with an interior edit,
actual solid replacement, proven chamfer contact and unchanged source models.

The combined native regression run passed **37 tests, 0 failures**. The final
ownership/replay adjustment is additionally covered by the focused boundary
and B-rep freeform suites. Formatting checks passed for all added Rust files.

The catalog audit passes. The full architecture/API-map gates remain open due
to existing concurrent source changes outside these feature modules; their
baselines/ownership assignments have not been relaxed. New public root modules
are assigned to the surfaces API domain.
