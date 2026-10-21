# Retained-face graft: closed control body with independent STEP evidence

## Implemented

Four-sided rational G2 corner construction and owned hole closure are recorded
separately in [CORNER.md](CORNER.md), including a 30-face closed STEP control
validated independently by OpenCascade. That control does not qualify arbitrary
unequal-radius fillet corners or the remaining roadmap.

`contact_graft::attach` attaches one owned open transition face to explicitly
selected retained sectors of distinct original faces in one originally closed
shell. Target face and keep-vertex choices are plans, not supplied proofs.
Patch corners must map injectively to byte-identical, already-bound source
vertices. No tolerance weld or nearest-point merge occurs.

All original source surfaces remain unchanged. Original outer-loop arcs are
retained; new boundary pcurves are pulled back into those original charts.
Their endpoints use the existing exact chart vertex definitions, followed by
fresh full-interval verification of the actual saved, possibly reversed pcurves.
Each new boundary must lie in the original face closure/material. Source holes
remain preserved and must pass containment in the resulting region.

Shell orientation is inferred consistently from all actual opposite edge uses;
contradictory sector choices refuse. Unused corner vertices/edges are compacted
with an explicit source-vertex remap. Unchanged source face IDs and imported
transition edge IDs survive. Changed faces, shell and body have Modify lineage;
change history is relative to the persistent source, not a temporary patch.
The final closed candidate requires fresh trim and exact-aware continuous seam
closure checks, sharing 100,000 cells and 1,000,000 exact work. Failure returns
no modified source.

## Supporting proof changes

- Source-root binding now admits existing edge endpoints. The original vertex
  is reused unchanged, after a fresh root-position bound, each incident edge's
  endpoint agreement and full-interval lifts for every incident face use.
  Different roots cannot reuse one vertex. Automatic search groups incident
  edge endpoints by their actual source vertex identity and refuses distinct
  eligible owners. It does not weld separate vertices or assert exact algebraic
  root identity. Tests cover two existing corner ends (no topology changes),
  one existing end plus one interior split, and a degree-one rational edge with
  unequal weights and natural domain [2,5]. A deliberately defective degree-18
  pcurve passing the nine legacy samples is rejected without modifying source.

- `contact_bind::automatically_between_faces` searches original source-edge
  parameters from freshly recomputed contact-end root bounds. It subdivides
  original rational curves using interval exclusion, reserves 63/64 of the
  position tolerance for subsequent admission, and charges every search cell
  to the same binding ledger. Multiple eligible edges, periodic edge charts,
  exhausted work and unseparated intervals refuse. Search results are proposals:
  the existing original-parent, shared-vertex and both saved-support lift gates
  still run before returning a modified model. Each end cutter and root index
  still need explicit selection. Root representatives are bounded, not exact
  algebraic identities.
- `automatic_source_roots_and_target_faces_form_a_closed_transition` exercises
  automatic edge binding for both support contacts, owned end construction,
  automatic target-face selection and closed graft assembly. Fresh trim,
  continuous closure and whole-seam G2 checks on the two longitudinal seams
  pass. This is the plane-pair control; it does not qualify arbitrary corner
  valence, unequal-radius corner patches, cap G2 or whole-body embedding.

- `plan_targets` infers original target faces from exact owned endpoints and a
  retained outer vertex per patch edge. `attach_planned` runs this plan through
  the existing fresh trim, pullback and closure checks. Ambiguous ownership,
  repeated target faces and scans exceeding 100,000 coedge visits refuse.
  Root binding and retained-material seeds are still required; arbitrary B-rep
  automatic Boolean trimming and unequal-radius corner construction remain open.
- The diagnostic control now bounds all eight exact-unresolved face pairs to
  owned shared-edge tubes within the original 1e-7 mm tolerance. Six use strict
  plane sidedness; two use original-span coordinate-flow derivative bounds and
  quarter-tolerance continuous lift checks. This is a metric contact-locality
  result, not exact absence of intersections. Existing exact pair, embedding
  and volume flags are not promoted. Added work: 1,188 cells and 96,292 exact
  operations. The coordinate-flow tests cover signed derivatives on a
  nonnormalized chart, inverse displacement bounds, flat coordinates and zero
  work. The control does not qualify arbitrary B-rep pairs.

- Rectangular face closure recognizes exactly tiled subdivided linear sides.
  Exact closure, no gaps, no overlap/retrace and four complete original bounding
  sides are mandatory. No rectangle fit or tolerance expansion is used.
- Trim-loop injectivity now admits clamped original rational B-spline curves
  whose scalar poles are ordered and whose every nonempty original knot block
  has strict progress. Positive rational basis monotonicity and continuous
  knots give an original-definition proof, without rounded decomposition.
  Flat spans and discontinuous knots refuse. Two-curve lens proofs retain their
  previous Bezier-only guard; local progress alone cannot admit an open chord
  contact inside a general spline lens.
- The existing profile arithmetic module was missing its `fiber` re-export;
  that compilation linkage was restored without replacing profile source work.

## Native control and regression evidence

The shared `contact_graft_fixture` builds the actual G2 proposal, binds contact
roots to a cuboid's existing edges, constructs both end curves and the open
skin, then removes the selected original corner arcs and grafts that skin.
The result has 10 vertices, 15 edges, 7 faces and one closed body. Every new
transition edge has two actual owners. Native trim and continuous closure pass;
whole-interval G2 is freshly confirmed on the two LONGITUDINAL contact seams of
the grafted result at source position tolerance 1e-7, normal 1e-5 and tensor
curvature 1e-4. End-face joints and untouched cube edges are not claimed G2.
Graft work is 1,416 cells and 4,794 exact work. Exact joins pass, but all exact
boundary lift identities do NOT pass; metric closure is a separate admission.

48 B-rep integration regressions, 14 NURBS trim/G2/shape regressions and the
subdivided-rectangle unit regression passed (63 tests). Invalid retention seeds,
insufficient work, scalar plateaus, disconnected knots, overlapping side tiles
and gaps refuse. The native geometry-bridge check passed. This is not WASM or
browser/worker/runtime acceptance.

## Independent changed-geometry STEP evidence

Run `contact_graft_acceptance` with an output directory, then
`scripts/verify-cad-step-occt.py <directory> --no-preview` in the OCP environment.
The final exported STEP hash was checked against the independent report after
final source/metadata edits.

OpenCascade 8.0.1.1.0 reports a valid single solid, 7 faces, 15 unique edges and
10 unique vertices. Bounds match [0,2,0]..[10,8,10] within 1.0002883499282689e-7 mm
(the independent bounds threshold is 1e-6; native source tolerance remains
1e-7). Measured volume is 599.4410714171789 mm3.

The independent DESIGN section is quintic, with plane-contact G2 jets:
`r=1.5`, `h=0.35*r*sqrt(2)`,
`X=[r,r-h,r-2h,0,0,0]`, `Z=[0,0,0,r-2h,r-h,r]`.
Analytic Bernstein product integration of `X*dZ` gives removed section area
0.09315476380274575 mm2; the design volume `600 - 6*area` is
599.4410714171835 mm3. Independent volume error is 4.547473508864641e-12 mm3.
This target uses the design polynomial, not native measured mass or exported
skin controls. Generated artifacts are under
`output/qualification/contact-graft-2026-10-16/`.

## Remaining full objective

This is a qualified CLOSED CONTROL CANDIDATE, not completion of general fillets.
Target/retention planning remains explicit; repeated target faces, complex
corners and general nonrectangular boundary-stratum closure can remain unproven.
Fresh native global embedding/self-intersection, whole-result wall thickness
and volume admission are still required. The manifest intentionally records
`nativeProven:false`. Independent STEP validity is not a native global proof.

The G2 builder's r controls its spine/setback construction; the quintic is not
a circular constant-curvature radius-r arc. No whole-radius certificate is
claimed. Unequal-radius complex corners, smooth end transitions, three-part
STEP/history acceptance, WASM/worker/UI, full P0 and remaining P2/P3 stay open.
