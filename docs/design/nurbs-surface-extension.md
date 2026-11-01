# Native tensor-product surface extension

`nurbs_core::editing::surface_extension::extend_surface` continues all control curves along a selected U/V axis using the curve extension contract. It supports either free end of that axis. The resulting curves must have exactly equal degrees, knot vectors and control counts; incompatible parameter advances are explicitly refused rather than assembled into an inconsistent surface.

This is useful for planes and compatible tensor-product surfaces such as translated copies of the same rational profile. A common requested physical length does not generally produce a common parameter advance on unrelated control curves. This API therefore does not claim to extend every surface by that physical length. It does not extend B-rep trims or sew topology.

Terminal-span extraction and existing control budgets are inherited from curve extension. Binary64 knot insertion/extrapolation is numerical; the `achieved_continuity` compatibility field echoes the requested class and does not certify actual G1/G2/C2 continuity. `stable` and `max_extrapolation_growth` summarize control-curve growth heuristics, not global error or injectivity bounds. A surface may have singularities or self-intersections even if these heuristics pass.

Public integration tests check the analytic plane parameterization throughout the entire enlarged domain for all four axis/end choices, preservation of the original UV rectangle on a multispan rational extrusion, refusal of incompatible parameter advances, and malformed/nonfinite requests. Native only; JSON/WASM/Rush/editor/STEP qualification remains open.
