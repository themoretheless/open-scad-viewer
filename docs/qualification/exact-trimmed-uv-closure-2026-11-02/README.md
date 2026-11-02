# Exact closure of declared UV trim intervals

2026-11-02. Native Rust qualification.

New cad-predicates endpoint identity compares original rational UV Bezier definitions at two authored physical parameters. Homogeneous de Casteljau evaluation omits common positive domain-span powers and compares cross-products. No rounded evaluated point or normalized quotient is admitted as an input leaf.

nurbs-core exact_trimmed_loop_joins checks each DECLARED interval end against the next declared start. Original clamped endpoint controls retain their structural proof, including multi-span curves when the selected parameter is an original clamped endpoint. Interior rational Bezier parameters (degree <=25 through current Curve validation; predicate ceiling 32) use one shared algebraic context/budget. Each join retains true/false/unknown; unknown is not closure. Curves and intervals are unchanged.

B-rep exact boundary verification now checks the requested trimmed UV intervals for explicitly trimmed loops, with algebraic work charged before use-lift verification. all_joins_exact no longer borrows closure from the full untrimmed source definitions. Partial diagnostic windows can have exact edge/lift agreement while their declared contour correctly fails closure. Periodic lifts remain unproved in this closure owner.

## Evidence

20 native tests passed: exact_trimmed_loop_joins 3, exact_trimmed_surface_lift 4, exact_trimmed_boundary 3, continuous_validation 6, validation_sampler 4. Logs attached. git diff --check passed.

Cases include an exactly closed square whose untrimmed definitions are not closed, directed/reversed UV intervals, rational 1/3 and 2/3 normalized source ends, one-ULP endpoint gap, unknown joins with exhausted work, invalid intervals and mismatched curve/range counts. Actual B-rep verification now proves the independent trimmed cuboid loops, refutes a UV gap below sampling tolerance, preserves the original model, and keeps partial diagnostic windows distinct from complete closed contours. Existing lift, reversal, metadata and resource regressions pass.

## Open scope

Endpoint closure does not prove simplicity, winding, injectivity, valid face regions, G2, curvature<=10, radius/thickness, self-intersections, sewing or solid volume. Interior endpoint evaluation on multi-span/periodic source curves needs further dispatch; full source endpoints retain their old literal proof. Arbitrary original NURBS face trimming and complete transition-body sewing remain unfinished. This milestone is native-only, absent from packaged WASM; no browser acceptance is claimed. STEP/part/history acceptance and remaining P0/P2/P3 are open.

Capacity clarification (2026-11-02): wrapper admission follows core/curve.rs, not the larger internal predicate layout ceilings.
