# Native sampled finite-biarc fitting

`nurbs_core::biarc::fit_biares_report` numerically fits pairs of tangent circular arcs to a planar source curve. The legacy spelling `biares` is preserved for compatibility. Output includes arc records, authored rational quadratic NURBS and the maximum computed sampled one-way source-to-output deviation.

Distances are measured to **finite** arcs and finite line segments, including endpoint and out-of-plane distances. A supporting circle is insufficient: a repeated circle traversal can lie on the supporting circle while the fitted short arc omits most source geometry. Straight pieces similarly require clamped projection onto a segment. Each source sample is compared with the union of both finite arcs; the joint need not correspond to the midpoint of the source parameter interval.

The existing limits remain unchanged: 32 sampling intervals per candidate pair, recursion depth 16 and at most 256 output arcs. Curvature-inflection cuts and 3D least-squares planarity checks are sampled. Nonplanar sources beyond tolerance/64, undefined endpoint tangents, failed construction, insufficient arc budget or recursion exhaustion are refused. Nearly planar 3D handling is numerical and does not prove exact planarity.

`max_error` and `sagitta_error` remain sampled diagnostics. Outward rounding of a floating-point result is not an exact distance certificate. There is no continuous Hausdorff bound, reverse output-to-source bound, multiplicity/winding preservation, source-parameter correspondence, self-intersection guarantee, certified G1/G2 or topology authority. This is approximate optional geometry, not exact simplification. A feature that requires these stronger guarantees must use another certified path or refuse.

Public contracts include an independent missing-quarter-circle regression, finite collinear overshoot/budget refusal, analytic semicircle and line equations, endpoint and tangent agreement, and invalid/nonplanar inputs. JSON/WASM/Rush/editor/STEP qualification remains open.
