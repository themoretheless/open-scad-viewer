# Exact trimmed rational surface lifts

2026-11-02, native Rust qualification.

New exact composition compares an original rational 3D Bezier curve restricted by its authored physical clock with an original rational UV Bezier curve restricted independently on an original rational Bezier surface. Homogeneous blossom restriction uses original clock differences, without rounded normalized parameters or extracted Euclidean poles. Reversed world traversal reflects the clock about the ORIGINAL domain exactly once. UV traversal may increase or decrease independently.

The restricted homogeneous UV control hull must lie inside the actual surface chart, and all original rational weights must be positive. Original UV poles may lie outside the chart if the proved restricted hull is inside. The complete cross-product composition identity then proves equality on the full requested interval.

B-rep boundary_agreement::verify_exact now processes supported explicitly trimmed coedges instead of always omitting them. The same shared work cap is charged, authored trim domains/periodic metadata are checked, and unsupported representations remain unknown. Explicit partial diagnostic windows prove only their requested relations, not whole-edge or whole-boundary coverage. Original untrimmed pcurve-loop join reporting is unchanged; trimmed-loop exact closure is not supplied by this proof.

## Evidence

27 native tests passed: exact_trimmed_surface_lift 4, exact_bspline_root_restriction 6, exact_rational_root_restriction 5, exact_trimmed_boundary 2, continuous_validation 6, validation_sampler 4. Logs attached. git diff --check passed.

Tests cover independently clocked rational curved traces, reflected/reversed traversal, rational surface weights, exact 1/3 and 2/3 normalized ends arising from authored physical clocks, outside original UV poles with a valid restricted hull, out-of-chart refusal, tiny interior perturbation with unchanged endpoints, exhausted work, and actual B-rep trim metadata. The B-rep case proves all 24 requested cuboid coedge relations without mutating the model, detects an altered world curve, and qualifies requested relations using exact evidence. The old test expecting all trimmed uses to remain unsupported was updated to require exact success with adequate work and refusal with insufficient work/invalid periodic lifts.

## Limits

Supported here: nonperiodic single Bezier source curves (world degree <=25 through current Curve validation, UV degree <=8) and rational Bezier surfaces (each axis degree <=8). General multi-span surface lift dispatch, periodic charts, exact trimmed-loop closure, arbitrary face-domain removal and complete fillet-body sewing remain open. This identity does not certify G2, curvature<=10, radius, thickness, injectivity, self-intersections or volume. Those independent whole-body gates remain required. Native only: no new packaged WASM or browser qualification is claimed. STEP/part/history acceptance and remaining P0/P2/P3 remain unfinished.

Capacity clarification (2026-11-02): wrapper admission follows core/curve.rs, not the larger internal predicate layout ceilings.
