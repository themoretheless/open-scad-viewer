# Whole-boundary routing of root-face holes

`QualifiedRootHoleAssignment` takes an actual `RootFaceChordSplit` candidate, examines every original hole coedge and produces a new split with automatically selected child assignments. Each complete original UV arc enclosure must be classified by winding as nonzero in exactly one child outer contour and zero in the other. All accepted cells for a hole must agree. Failed classification never counts as exclusion; it leads to bounded subdivision or an unresolved error.

Clock covers include endpoint uncertainty and use outward conversion. Subdivision covers the whole closed clock range. The original world curves, UV projections, global selectors, hole loop IDs and inserted chord remain intact. A hole boundary crossing child regions, touching an unresolved boundary or exhausting resources yields no assignment certificate.

Resource policy bounds cells, depth and winding calls. Each geometry query and winding call has its own bounded native policy; the aggregate is bounded by these explicit call ceilings. This does not claim a tighter single-operation shared geometry allowance.

The proof locates complete hole boundaries relative to the two outer winding regions. It does not prove Jordan embedding or hole interior nesting, chord/boundary nonintersection, retained material, shell validity or general Boolean completeness. Those remain required, alongside all unrestricted SS singular/tangent/periodic/pole cases and healing.

Tests use two original-root circular hole boundaries, one in each region of a circle split by an algebraic AC chord. Replayed assignment must preserve hole loop IDs and reject an insufficient query budget. The shared circle fixture now allows a scaled circle on the same original plane without changing the original-plane surface definition.

Validation: 12 tests passed across root_uv_closure, root_model and root_brep_subdivision. The extended UV test suite took 64.02 seconds. Architecture audit passed: 54 pinned oversized files, zero layer violations. Logs: docs/qualification/root-hole-assignment-2026-11-04.
