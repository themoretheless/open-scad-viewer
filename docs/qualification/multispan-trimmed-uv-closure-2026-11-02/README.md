# Multi-span trimmed UV endpoint closure

2026-11-02, native qualification.

exact_trimmed_loop_joins now dispatches non-Bezier original curves to the existing exact rational B-spline UV point predicate. Original poles, weights, expanded knots and physical trim parameters remain the authored input leaves. No sampled/evaluated Cartesian point or rounded extracted span supplies authority. At an internal knot, both adjacent positive-width spans must agree. Disconnected curves with interior multiplicity above degree are rejected by Curve validation before point evaluation.

Bezier endpoints retain the existing fast predicate. Literal original clamped endpoint controls retain structural equality without an algebraic arena or work charge. Thus large fully literal loops are not refused by an irrelevant algebraic census cap. When algebra is required, one shared context covers the loop and individual unknown joins remain explicit. Algebraic source census is capped at 32768 controls; the existing public Curve validation limits degree to 25 and controls to 256 per curve. Larger internal predicate layout ceilings are not exposed model capacity.

## Evidence

31 native tests passed: exact_trimmed_loop_joins 7, exact_bspline_root_restriction 6, exact_trimmed_surface_lift 4, exact_trimmed_boundary 4, continuous_validation 6, validation_sampler 4. Logs attached. git diff --check passed.

New cases cover rational interior trim endpoints across C1 spans, a cut exactly at a C1 knot, a C0 shared knot, reversed declared intervals, one-ULP gap, insufficient work, invalid discontinuous source and a 129-curve literal loop exceeding the algebraic census but requiring no algebra. The B-rep case proves multi-span UV endpoint closure, refutes a tiny altered knot endpoint and explicitly keeps the unsupported multi-span surface lift UNPROVED. Closure alone does not raise whole-boundary/solid admission.

Four earlier qualification README files were clarified against current core/curve.rs: the wrapper limits are degree 25 / 256 controls, not the larger raw predicate ceilings. This correction does not expand the tested geometry scope.

## Open scope

Nonperiodic endpoint closure only. It does not prove loop simplicity/winding, multi-span edge/surface lift, valid removed face domains, G2/curvature<=10, radius/thickness, embedding or volume. Periodic chart transport, arbitrary original NURBS face trimming/sewing and mixed-radius junctions remain open. Native only; these latest changes have not been packaged into WASM or browser-qualified. STEP/part/history acceptance and remaining P0/P2/P3 remain unfinished.
