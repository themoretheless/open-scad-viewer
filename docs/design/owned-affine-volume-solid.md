# Original closed body and coefficient-owned smooth Solid

This Rust checkpoint adds a strict closed-body constructor and a supplied-model
auditor. Neither accepts caller proof flags. The source and retained body have
separate global certificates. TypeScript does not own any geometric decision.

## Proof chain

1. The original `MultiSweep`, filled material surface and UV trims are validated.
   Original UV/world trace identities, regular profiles, valid oriented material
   and hole regions, and the original whole-volume parameter-pair atlas are
   required. Every source knot interval and one-sided derivative is enclosed.
2. One common coefficient-owned piecewise quintic affine motion is constructed
   from the original station sections. The existing native error owner bounds
   original-wall error, exact decomposition error and reconstruction displacement.
   Their outward sum must fit the requested total error budget.
3. Every actual BRep face must equal its expected affine image of an original
   profile span, coefficient by coefficient. Exact arithmetic residuals must
   vanish. Weights, degrees, knots, periodic flags, shell membership and full
   rectangular face domains are checked. Small changes within the ordinary
   BRep tolerance still fail this binding.
4. The complete native boundary inspector checks all owned edges, exact
   correspondence, connected vertex links, all-face regularity and the requested
   G1 or G2 identities. There is no caller-provided ignored-edge list. A strict
   G2 request fails at a genuine G1-only curvature jump. Sharp corners stay sharp.
5. A second exhaustive global atlas proves the **actual retained filled volume**:

   `F(u,v,t) = T(t) + A(t) (Q(u,v) - origin)`.

   `Q` is the original filled carrier, not an extracted floating proxy. Each
   quintic pose stores three matrix columns and a translation. All station joins
   and the closed-time identification require literal full-pose equality.
   Every original temporal piece intersecting a query contributes both its value
   and its normalized derivative enclosure. Interval multiplication by the piece
   count selects pieces; rounded `i/count` values are never knot premises.
6. The valid original UV material is transported by this global embedding.
   A conditional world Jacobian sign and original UV winding determine all actual
   face-use orientations and the material outer/hole shell roles.

Only a fully successful chain returns an opaque immutable `SmoothClosedBody`.
Each incomplete stage returns a report and no model. The supplied-model verifier
repeats the entire chain for its actual geometry; STEP-imported flags grant no
credit. This proves a homeomorphic parameter/material topology for original and
retained bodies. It does not claim an ambient isotopy between their embeddings.

## Global atlas and shared arithmetic

`nurbs-core::affine_volume::certify` uses the same internal exhaustive pair-cover
engine as original source volumes. Every distinct parameter pair is covered by
strict world separation or an injectivity proof on its complete joint region.
Such a proof uses nonsingular averaged Jacobians over the whole convex hull, or
one fixed positive-definite left metric and coordinate scale throughout the
subcover. Closed-time head/tail pairs require a common lifted seam chart. A local
nonzero determinant or invertible `A(t)` alone grants no body credit.

First rational carrier derivatives use coefficient polynomials
`(H_u W - H W_u) / W²`, retaining cancellations lost in independent quotient
intervals. Original knot-cell extraction remains outward. Degrees above five use
the existing general stable-jet fallback. Shared bounded tensor-Bernstein
arithmetic also supplies the existing rational surface-normal verifier.

All exact comparisons and geometric queries have explicit finite ceilings.
Zero or one-short budgets refuse. Exhaustion is not a collision witness. The
literal double-cover negative is rejected; a precision-limit refusal there does
not locate an intersection. Temporal caching is scoped to one native ledger.

## Qualified cases and limits

Focused native cases cover spatial RMF, authored and guided frames, variable
scale/twist/affine laws, 40 nonbinary parameter intervals, holes, reversed UV
winding and a reversed carrier chart, a nonplanar filled carrier and nonplanar
original profiles, a genuine G1-only curvature jump, exact binding mutations,
and all owner-stage budget refusals. Native JSON construction and audit include
a STEP round-trip followed by a fresh imported-model audit.

The proof is deliberately conditional, not universal. Coefficient binding
currently requires the native face/chart ordering and exact polynomial motion.
It does not recognize arbitrary STEP gauge changes or arbitrary reindexed models.
Physical end caps are absent from this closed construction; the filled material
carrier is a reference cross-section, not a cap/wall G2 claim. The original
frame/boundary G1/G2 certificate is a separate unfinished obligation, explicitly
reported as `idealSourceSmoothnessCertified: false` in JSON. Pole quotients,
general physical cap joins and mixed C0/G1/G2 policies remain outside this API.

Full native qualification, public WASM rebuilding, external STEP qualification,
semantic G1 matrix, browser qualification and fresh CI are separate gates. A
focused success or this design note does not close them.
