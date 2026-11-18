# Exact rational product clocks and joint boundary guarantees

Qualification completed on the frozen 4665-file source snapshot. Native Rust only; no WASM/UI publication in this run.

## Proved class

A positive geometric Bernstein weight sequence `w_i = w_0 r^i` represents
exactly the same Euclidean control polygon as its polynomial view composed
with `phi(t) = r t / (1-t+r t)`, on the complete normalized interval.
Positive tensor-geometric weights give independent such clocks in U and V.
All product identities are checked as exact rationals from the original
binary64 weights. A one-ULP mismatch is unsupported by this route.

Only strict coordinate monotonicity and whole-chart injectivity transfer.
The polynomial view's coordinate projection, metric, contraction and rank
metadata are cleared. Actual boundary agreement, regularity, trim simplicity,
contacts, shell orientation and smoothness retain their own proof owners.
This view does not assert equality at a common original parameter.

The new contact proof supports rational Bezier strips strictly on one side
of an exact planar carrier except for two natural rows. Each zero row must
be a complete shared edge, strictly outside the other original carrier hull,
or restricted by exact hull intersection to one actual shared vertex.
Unowned line/area contact, mixed signs and unsupported contact unions refuse.
This path runs only behind the existing joint exact-domain prerequisites.

## Geometry under qualification

The authored profile is `C(q) = 10(q^5, q^2+q/4)` with
`q = (3u-2)/(2-u)`. Its rational degree-five weights are `[32,16,8,4,2,1]`.
The isolated X critical point occurs at the non-dyadic original parameter
`u=2/3`, with nonzero exact Y derivative `45/8`. Three profile spans at
stations z=0,1,2 give eight faces and three actual internal station seams.
The two planar caps use literal XY charts and original rational edge trims.

The test requires continuous whole-boundary error zero AND complete global
volume certificates AND independently extracted station G1/G2, including
STEP import/transport. Sharp cap/profile corners preclude whole-body G1/G2.
The independent exact oracle binds every carrier and cap trim to the body
hash. The swept material volume is exactly `800/7`.

## Limits and isolation

Existing budgets and tolerances are unchanged. New attempts spend the
caller's remaining shared work; the restricted contact trial is capped at
256 within the former per-pair limit of 1000. Failed attempts count too.
Degree 32 and the existing arithmetic/decomposition caps remain unchanged.

Qualification uses `/private/tmp/osv-rational-product-20261103`, cloned from
the preceding frozen quintic snapshot, plus the selected rational overlay.
Concurrent unrelated main edits are preserved and excluded from this audit.
Build optimization is level 1 with debug assertions and overflow checks on;
build jobs and test threads are both 2. No GUI automation is used.

## Scope still open

Arbitrary rational weight nets, general algebraic maps and all combinations
of moving frames, motion laws, caps and periodic seams are not proven by this
class. A retained-boundary bound does not prove an ideal sweep motion law.
Local station G1/G2 never supplies a full-shell smoothness certificate.

## Final results

| Gate | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| nurbs-full | 2271 | 0 | 1 |
| brep-full | 2317 | 0 | 5 |
| algebraic-maps | 41 | 0 | 0 |
| native-matrix | 47 | 0 | 0 |

Counts are executions per gate. Existing ignored measurement cases remain named in `audit.json`; no new exclusions were introduced. The focused three new joint tests are also included in the full BRep gate.

Original, imported and transported boundaries all prove continuous error zero together with complete regularity, chart injection, trim, contact, nesting and orientation. All three actual station seams independently prove G1/G2 before and after STEP. Three low-global-budget controls retain the zero continuous bound but refuse Solid even when caller success fields are forged. Separate Rust tests preserve rank-loss, folded-map, weight-drift and missing topology refusals, plus exact-budget replay and one-short failure. Sharp full-shell smoothness remains false.

The exact rational oracle reconstructs the denominator and both coordinate numerators on the full interval, including the non-dyadic critical parameter 2/3. Independent OCCT reports a valid 1-solid/8-face/15-edge/9-vertex body, zero interference pairs, and absolute volume error 3.297131456747593e-09 under the unchanged 1e-7 tolerance. Exact-parameter boundary point error is 1.3322676295501878e-15; closest-point diagnostics do not control acceptance.

Frozen source, selected owned main files, probe inputs and every gate log were hash-verified after qualification. `audited-source.tar.gz` includes the baseline literal compile-time inputs plus this overlay. `source.json`, `owned.json`, `inputs.json`, `audit.json` and `evidence.tar.gz` record reproducibility. Concurrent unrelated main edits are outside this source qualification.
