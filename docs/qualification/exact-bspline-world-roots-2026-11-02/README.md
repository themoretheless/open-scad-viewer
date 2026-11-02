# Exact multi-span rational world roots

2026-11-02. Native Rust qualification.

The new nurbs-core wrapper uses the existing cad-predicates rational B-spline restriction predicate to compare stored 2D/3D child definitions against their original source. Every common knot cell is compared homogeneously; no floating knot insertion, rounded reclocking or point sample supplies identity authority. Source and candidate retain ORIGINAL parameter coordinates. Current native Curve validation admits nonperiodic rational curves of degrees 1..25 with at most 256 controls; the underlying predicate ceilings of degree 32 and 4096 controls are not exposed geometry capacity; unsupported representations return None and limits/missing proof remain explicit predicate decisions.

ExactBsplineRootPartition owns the original source, actual stored children and requested original interval. It checks complete ordered interval coverage with no gaps/overlaps, shares one algebraic budget among all children, and requires fresh full-cell identities. Original continuity and complete identities prove common functional endpoint values. Reverification recomputes the proof; this is not a topology-ID or solid-admission owner.

The real CAD edge_partition::Split exposes exact_world_root_partition on its actual original 3D edge and stored children. Existing tolerance-qualified numerical proposals remain distinct; an unproved exact partition returns None. Closed UV carrier preparation now uses the multi-span predicate under its existing shared budget.

## Evidence

22 native tests passed: exact_bspline_root_restriction 6, exact_rational_root_restriction 5, periodic_exact_partition 4, closed rational UV preparation 1, actual CAD shared root partition 1, shared edge trim regressions 5. Logs attached. git diff --check passed.

Cases include rational C1 multi-span 3D roots, an interior-to-interior interval crossing a C1 knot, degree/gauge variation, 2D UV curves, tiny interior perturbations, changed clocks, outside intervals, missing/reordered/overlapping children, exhausted work and fresh reverification. The CAD test partitions a real shared rational edge, reproves its complete source function, checks that both neighboring faces use each child, reruns existing boundary qualification and preserves the original model. Regression cases retain partial, nonunit/descending, rational arc and reflected trim relationships.

## Open scope

This extends exact FUNCTIONAL root agreement; it does not construct an arbitrary rational NURBS fillet body or certify its G2/curvature/thickness/radius. Periodic root correspondence and independent/reversed candidate reclocking need additional dispatch. Rational source-face trimming/sewing, mixed-radius complex nodes and full body qualification remain open. This milestone is native-only and not included in packaged WASM. STEP/part/history acceptance and broader P0/P2/P3 remain unfinished.

Capacity clarification (2026-11-02): wrapper admission follows core/curve.rs, not the larger internal predicate layout ceilings.
