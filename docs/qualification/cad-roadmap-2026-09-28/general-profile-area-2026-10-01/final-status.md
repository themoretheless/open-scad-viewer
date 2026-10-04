# General rational profile signed area — 2026-10-01

Native foundation only. `nurbs_core::planar_area::measure` integrates the original closed XY rational curve chain over every nonempty knot span. Exact clamped joins and positive weights are required. It preserves the input definition and reports an outward signed area interval, convergence and evaluated-cell count.

The Green integrand uses interval rational jets and a trapezoid remainder bounded by its second derivative. Jets are normalized to each integration cell; source knot widths are not multiplied again. A heap refines the largest uncertainty. Accumulation and the final interval-width comparison round outward. Budget exhaustion or parameter precision limits remain explicit unproven results. An initial budget that cannot cover all source spans is rejected.

Five focused tests cover polynomial area, reversed traversal, translation, rational quarter-circle chains, inserted internal knots, changed knot domain, coordinate scaling, invalid/open chains, work limits and a self-intersecting chain whose signed integral is zero. The circle reference uses represented binary weights and tests enclosure at the stated tolerance.

This operation does not establish simplicity, hole roles or material area. It is not yet connected to retained-profile validation, document loading, preparation, extrusion, Boolean or offset. Those routes retain their current qualified analytic scope. Periodic/unclamped closure and single-curve loops are not admitted here. No browser, WASM or STEP qualification is claimed for this new operation.

Validation: full native NURBS library suite, 228 tests; see `native-tests.txt`. All P0–P3 roadmap items remain governed by their original acceptance criteria.
