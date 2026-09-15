# R1 — Transverse bicubic NURBS SS spike (research-only)

Status: filed  
Date: 2026-09-15  
Decision: **narrow** (not go; not stop)

## Scope (frozen before results)

Research-only matrix for transverse bicubic × bicubic contacts:

| ID | Pair | Contact | Expected |
|----|------|---------|----------|
| R1-01 | bicubic bump × plane-like bicubic | transverse curve | Complete or typed refuse |
| R1-02 | identical bicubics | coincident | typed refuse (no false Complete) |
| R1-03 | nearly tangent bumps | tangency | Incomplete / TangencyOrMultipleRoot |
| R1-04 | disjoint patches | empty | Complete empty admitted |
| R1-05 | fitted-vs-procedural same shape | correspondence | ADR note only |

Out of scope for R1: product Boolean imprint, signed-weight NURBS, general G2.

## Fitted vs procedural ADR

- **Procedural** (constructor → exact rational carriers) remains the only path that may later publish `Coverage::Complete`.
- **Fitted** (mesh/sample → NURBS) may produce diagnostic `NumericallyResolved` only; never `Complete`; never topology change.
- Kill criterion: any fitted path emitting `Complete` → **stop**.

## Tangent / coincident counterexamples

- Near-tangent (separation < `distance_tolerance`) must not round into transverse Complete.
- Coincident patches must refuse with `NearCoincidence` / `CoincidentTrim`, never a welded curve.

## Failure taxonomy

| Class | Action |
|-------|--------|
| BudgetExceeded | Incomplete; raise boxes only in a new qualification version |
| TangencyOrMultipleRoot | Incomplete; no seed marching rescue |
| NearCoincidence | Incomplete |
| UnsupportedSurface | Incomplete; matrix expansion needs new plan |
| False Complete | Kill / stop productization |

## Verdict: narrow

- Keep analytic B-rep (Phases A–D) as the **production ceiling**.
- Allow a **narrow G6** product path only for transverse bicubic pairs that:
  1. Pass an independent coverage verifier with zero false-Complete on the frozen fuzz work units.
  2. Refuse every out-of-matrix pair.
  3. Never authorize Boolean imprint until a dedicated G5e-NURBS gate (future) with its own QualificationPlan.
- Do **not** claim general NURBS Boolean.

## G6 gate seeds (frozen if narrow proceeds)

- Capability: `nurbs-ss-transverse-bicubic/1`
- Positive: R1-01, R1-04 with Complete + verifier
- Negative: R1-02, R1-03, degree>3, rational weights ≠ 1, trimmed boundaries
- Work units: 3 clean CI runs; fuzz budget fixed before first run
- Rollback: any false Complete resets counter and freezes capability at Unavailable

## Stop clause

If narrow G6 cannot hold zero false-Complete under the frozen seeds, leave analytic B-rep as the explicit production success mode and mark NURBS-SS `Unavailable`.
