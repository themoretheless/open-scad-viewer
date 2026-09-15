# B-rep / NURBS QualificationPlan seeds (Phase G + Full coverage P0–P6)

Frozen evidence artifacts live under `docs/qualification/*-evidence-v1.json`.

Per-capability seed plans:

- [`qualification/analytic-fillet-1.md`](qualification/analytic-fillet-1.md)
- [`qualification/analytic-shell-1.md`](qualification/analytic-shell-1.md)
- [`qualification/analytic-solid-loft-1.md`](qualification/analytic-solid-loft-1.md)
- [`qualification/step-interchange-1.md`](qualification/step-interchange-1.md)

## Shipped registry (`brepCapability.ts`)

| Capability | Maturity | Evidence |
|------------|----------|----------|
| planar-csg/1 | Qualified | existing planar corpus |
| analytic-boolean/1 | Qualified | [analytic-boolean-1-evidence-v1.json](../qualification/analytic-boolean-1-evidence-v1.json) |
| intersection-queries/1 | AnalyticComplete | `analytic_ss` + verifier |
| nurbs-ss-transverse-bicubic/1 | AnalyticComplete | [nurbs-ss-g6-evidence-v1.json](../qualification/nurbs-ss-g6-evidence-v1.json) |
| analytic-fillet/1 | AnalyticComplete | [analytic-fillet-1-evidence-v1.json](../qualification/analytic-fillet-1-evidence-v1.json) |
| analytic-shell/1 | AnalyticComplete | [analytic-shell-1-evidence-v1.json](../qualification/analytic-shell-1-evidence-v1.json) |
| analytic-solid-loft/1 | AnalyticComplete | [analytic-solid-loft-1-evidence-v1.json](../qualification/analytic-solid-loft-1-evidence-v1.json) |
| step-interchange/1 | AnalyticComplete | [step-interchange-1-evidence-v1.json](../qualification/step-interchange-1-evidence-v1.json) |

Release ledger: [brep-capability-registry-release-v1.json](../qualification/brep-capability-registry-release-v1.json).

## Release policy

- One walking slice per merge: falsifiable certificate + tests + envelope honesty.
- False `Complete` resets the capability counter and freezes it at Unavailable.
- Peer engines: B-rep failure never routes to Manifold.
