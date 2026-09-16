# B-rep full-close QualificationPlan seeds (F0–F9 + deepenings)

Full matrix: [`../qualification/brep-full-closed-matrix-v1.json`](../qualification/brep-full-closed-matrix-v1.json)
G8 index: [`../qualification/plans/g8-full-matrix-index-v1.json`](../qualification/plans/g8-full-matrix-index-v1.json)
Canonical registry: [`../qualification/brep-capability-registry-release-full-v1.json`](../qualification/brep-capability-registry-release-full-v1.json) (`unresolvedInShippedMatrix=[]`)

## Capability maturity

| Capability | Maturity |
|------------|----------|
| planar-csg/1 | Qualified |
| analytic-boolean/1 | Qualified |
| nurbs-boolean-bezier-le3/1 | AnalyticComplete (topology change permitted; supersedes transverse-bicubic/2) |
| analytic-chamfer/1 | AnalyticComplete |
| step-interchange/1 | AnalyticComplete (graph-only + CIRCLE + cuboid/tube honesty + product seam) |
| nurbs-step-bicubic-face/1 | AnalyticComplete (bicubic open-face B_SPLINE STEP) |
| nurbs-step-trimmed-bicubic/1 | AnalyticComplete (bicubic open face + FACE_BOUND holes) |
| nurbs-step-solid/1 | AnalyticComplete (closed freeform MANIFOLD solid + Boolean RT) |
| iges-interchange/1 | AnalyticComplete (186/514/510/144 solid topology) |
| analytic-fillet/1 | AnalyticComplete (multi-edge chain remapping) |
| analytic-shell/1 | AnalyticComplete (closed cuboid + cylinder offset) |
| analytic-solid-loft/1 | AnalyticComplete (FrameLaw Frenet/RMF/fixed) |
| nurbs-ss-bezier-le3/1 | AnalyticComplete (Phase B Bezier deg≤3 elevated to bicubic; supersedes transverse-bicubic/2) |

## Invariants

Fail-closed; no prism on curved Boolean success; no Manifold cross-route ([ADR](manifold-keep-as-peer-adr.md)); false-Complete freezes Unavailable. **STEP+NURBS product-matrix closed** for the admitted freeform STEP + bezier-le3 cells. Parasolid parity remains an explicit refuse.
