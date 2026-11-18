# Sewn-corner curvature admission

Native check of the actual retained-face/transition B-rep, rather than the unsown proposal. `inspect_curvature` visits saved trimmed face domains with a shared cell/domain/shape budget. `qualify_g2` combines immutable constructed-seam G2 evidence, fresh curvature coverage at 10 mm⁻¹, and the existing embedding/material/volume admission. Original sharp stock edges remain sharp. An unresolved or omitted face cannot grant admission. Confirmed excess and unproven coverage have distinct error codes.

Ten focused tests pass. They cover closed incidence and exact boundaries, actual seam jets, fresh embedding/material/volume, rational curvature excess, full planar-face coverage, shared-budget exhaustion and source preservation. See `native-results.txt` and `review.json`.

The control corner is **not admitted under the curvature contract**. Face 6 has a certified inside witness of [11.962868376092837, 11.962868376095262] mm⁻¹. Faces 10 and 12 remain unresolved (`continuity_not_proven`). All 13 faces were inspected; this is not a successful whole-body curvature certificate. The shape still needs rebuilding, and the remaining internal continuity/degenerate-pole field coverage needs proof. No geometric tolerance or work limit was relaxed.

This increment does not add arbitrary original NURBS face trimming, general multi-edge corner construction, thickness qualification, STEP acceptance or WASM/worker/UI delivery. The production binary is unchanged by this increment.
