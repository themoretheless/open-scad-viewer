# Rust NURBS: 198/200

Frozen source: `/private/tmp/nurbs-native-200-chart-canonical-20261019` (133 selected files).

Fresh native correctness: 21 passed, 0 failed, 1 explicitly ignored paired benchmark, 356.30s. WASM and scoped formatting pass. This includes three original multispan Boolean regressions; exact touching-square and tangent-hole cycle/material queries; five actual native frame cases; prior material/winding checks; bounded operation-local replay identity, failure, lifetime, capacity and thread-isolation tests. Two previous prototypes failed because test queries encoded unreduced fractions. Only query fixtures were corrected through the existing canonical exact-ratio encoder, and explicit rejection of unreduced input remains.

The exact winding batch uses original restricted source traces and algebraic native frame cuts. It does not normalize chart queries periodically. Boundary contact, outside material and arithmetic/work refusal remain distinct. Full source replay occurs on construction; immutable prepared data is reused for queries.

Operation-local arrangement replay reuse has passed correctness. Its separately ignored eight-run alternating measurement passed (session27986 consumed0): disabled median 39.220s versus enabled 3.019s, ratio 12.99 on this actual constructor only. Disabled runs replay108 times; enabled runs replay once and reuse the same pinned immutable Arc107 times. All geometry and material outputs agree. Retain the optimization; no general CAD or whole-suite speedup claim. See benchmark.json and benchmark.log.

Next: exact source-bound left/right witnesses, outer/hole and local patch ownership, physical world-face publication and actual seam sewing. General complete surface intersection atlas discovery and branch/trim assembly remain open. No catalog completion or shared Rust integration is inferred from this isolated receipt.
