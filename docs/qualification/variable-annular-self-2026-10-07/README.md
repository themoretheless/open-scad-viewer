# Variable annular body: complete native self-intersection qualification

Date: 2026-10-07. This report covers the four native control bodies from
`partial_annular_variable_arc(20, 5, 6, radii, direction*[0.3, 0.7, 0.4], 1e-7)`.
The two radius orders are `[0.5, 1.25]` and `[1.25, 0.5]`; directions are +1 and -1.

## Result

Every body has 31 qualified faces and all 465 face pairs classified:
338 disjoint pairs, 127 contacts restricted to owned shared boundaries,
zero unresolved pairs, zero unvisited pairs, and no continuation cursor.
The joint native audit reports `absenceProven=true` and `proven=true` for all four.
Exact boundary agreement, exact joins and simple trims also pass. The model's
serialized original geometry is unchanged before and after each joint audit.
The complete results, bounds and budgets are in `final.json.gz`.

This closes the reported self-intersection gap for these four control bodies.
It does not qualify arbitrary NURBS fillets, thickness, G1/G2 at poles,
independent STEP measurements, WASM packaging or browser interaction.
The older 255 unresolved pairs on a different control model are not covered here.

## Diagnostic correction

The earlier per-face JSON used only the regular projection result and omitted
existing owned-pole quotient certificates for faces 0 and 10. Both were already
qualified. The true initial state was 30/31 faces, with face 5 unproven.
The serializer now uses `Face::absence_proven()` and records quotient reasons.
Historical evidence is retained; earlier global failure was still correct.

## Proofs added

1. **Middle face.** Original coefficient interval bounds enclose the Jacobian
   of `(atan2(y,x), sqrt(x*x+y*y)-z)` on the whole chart. A strictly positive
   X enclosure establishes a continuous angle branch. The row-oriented global
   interval matrix has positive diagonal bounds and positive determinant bound.
   Internal knots that break C1 are refused by the single-chart verifier.
   This applies the global univalence condition from
   [Gale and Nikaido, 1965](https://link.springer.com/article/10.1007/BF01360282).
   It is a separate certificate, not a claimed contraction result.
2. **Six cap/wall vertex pairs.** Original owned straight edges and bounded
   tilted anchors propose separating planes. Exact orientation signs on all
   original enclosure controls and positive Bezier basis support strata prove
   that contact is restricted to the owned vertex. Rounded proposals have no
   authority without these checks.
3. **Middle blend/wall pair [5,7].** Matching original natural-boundary controls
   and weights establish a continuous union of two original Bezier charts.
   The wall parameter is reversed and original parameter widths are enclosed.
   One global interval Jacobian encloses every segment integral, including
   segments crossing the seam. Positive principal minors make every enclosed
   integral matrix nonsingular, proving injectivity of the union. This proof
   allows the piecewise smooth seam; it does not claim seam G1 continuity.
4. **Collapsed exit/wall pair [10,12].** The exact source frame uses tangent
   `t=A-P` and normal `n=h-(h.t)/(t.t)*t`. Literal original wall identities
   establish `n.t=0` symbolically before factoring the derivative. Whole-chart
   weighted dominance proves the glued union, with the existing owned pole
   quotient. No tolerance-based zero assignment is used.

The middle joined determinant lower bound is at least
0.4146669230344915. Exit joined dominance margins are
0.9252649196219059 and 0.5609971422803074 for the two radius orders.
All contact certificates enter pair classification only after the existing
joint gate proves exact lifts, exact joins, simple trims, positive winding
and every face's injectivity. Coincident geometry without owned topology
is refused.

## Progress evidence

The intermediate reports use the two positive-direction bodies; each row has
the same counts in both radius orders. `final` covers both directions.

| Report | Qualified faces | Visited pairs | Disjoint | Shared boundary | Unresolved |
| --- | ---: | ---: | ---: | ---: | ---: |
| probe | 30 | 465 | 328 | 44 | 93 |
| p-matrix | 31 | 465 | 328 | 44 | 93 |
| joint-128 | 31 | 465 | 328 | 119 | 18 |
| joint-before-tilt | 31 | 465 | 338 | 119 | 8 |
| joint | 31 | 465 | 338 | 125 | 2 |
| joint-polar | 31 | 465 | 338 | 126 | 1 |
| final | 31 | 465 | 338 | 127 | 0 |

The original shorter pair prefix visited 163/465 pairs and left 302 unvisited.
Increasing complete pair coverage first exposed 93 unresolved pairs. The joint
proofs then removed those uncertainties; an unproven pair was never relabelled
as an actual defect or as safe without a certificate.

Final budgets: tolerance 1e-8; face spans 10000; exact work 1000000;
trim pairs/cells 10000 each; trim domain cells 1000000; pair count 1000;
geometry cells 1000000; UV domain cells 8000000; per-pair geometry cells 4096
and UV domain cells 100000. New joined certificates use 16 subdivisions,
512 cells for two charts. Earlier `joint-128` used 128 geometry cells and
2000 domain cells per pair. `probe` and `p-matrix` use plain pair inspection
without the joint contact admission gate.

## Verification

Final focused suites passed: surface injectivity 13, quotient injectivity 7,
hull contacts 13, boundary embedding 6, self-intersection 3: 42 tests.
The separate face-injectivity suite passed 9 tests. Logs are archived here.
Tests include wrong angular branches, folds, C0 internal knots, insufficient
coverage, source seam perturbation, reversed walls, invalid exact source order,
missing topology ownership, interior overlap and microscopic gaps.

Reproduce from the isolated checkout:

```sh
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-self-qualification \
  -- /tmp/variable-annular-self.json embedding
```

All reports and logs use deterministic gzip with mtime=0.
`manifest.json` records SHA-256 hashes of compressed evidence files.
