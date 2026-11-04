# Winding on mixed original SS contours

`QualifiedRootLoopWinding` classifies a finite UV query box against one root BRep loop. Each coedge supplies its own original graph, projection support and traversal direction. Exact UV joint proofs establish a shared native point. Intersections of the two endpoint enclosures supply one common joint enclosure/sample, keeping the temporary winding polygon connected across different SS pairs.

Adaptive subdivision uses complete original UV arc enclosures. A chord is accepted only when the convex hull of the complete arc and endpoint boxes is separated from the entire query box along at least one coordinate. Convex homotopy therefore preserves winding. The temporary polygon is only a winding witness; it does not replace world or pcurve geometry. Chord orientation predicates use exact binary-rational arithmetic.

All geometry queries charge a shared remaining budget, and segment attempts have a fixed ceiling. Unresolved boundary contact or insufficient resources returns no classification. Root curves and algebraic clocks remain the original definitions.

This supports mixed-pair region queries. It does not certify Jordan embedding, hole nesting, arrangement completeness, material orientation, shell closure or Boolean validity. Those requirements and full singular/tangent/periodic/pole branch completeness remain active.

Validation: 8 tests passed across root_uv_closure and root_model. Architecture audit passed (54 pinned oversized files, zero layer violations). Logs: docs/qualification/mixed-ss-loop-winding-2026-11-04.
