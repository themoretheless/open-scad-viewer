# Sweep: exact Gram cofactors, original geometry and fresh global audit

2026-11-02. This is a bounded Rust extension and qualification milestone, not
completion of arbitrary algebraic correspondence or all sweep combinations.
`result.json` is the final authority only when its `passed` field is true.

## Final audit status

**Audit completed; qualification not passed.** 477 tests passed and five failed
out of 482 executed tests. All 32 native protocol tests and the no-codec check
passed. Both independent STEP cases passed. Final source guards and owned-main
hashes match. No limits, deadlines or tolerances were increased, and no failing
expectations were changed into passing tests.

| Phase | Passed | Failed |
| --- | ---: | ---: |
| Topology | 19 | 0 |
| Predicates | 154 | 0 |
| G1/G2 continuity | 66 | 0 |
| Complete world traces | 39 | 0 |
| BRep/sweep | 167 | 5 |
| Native protocol | 32 | 0 |

The unresolved positive BRep fixtures are:

- `rational_hollow_cap_quarter_turn_frame_retains_exact_material_and_full_volume`:
  `actual-affine-filled-volume-unproved`, with the inner `ideal-volume-work-limit`
  exhausting the existing 100000-cell allowance. This is a work refusal, not a
  wall-clock timeout.
- `closed_owned_hollow_authored_arc`, `closed_owned_hollow_fixed_arc`,
  `closed_owned_hollow_guided_arc`, `closed_owned_hollow_guided_parameter`:
  `retained-periodic-volume-unproved`. Retained face-pair/contact and orientation
  obligations remain unproved. These are not classified as invalid geometry
  merely because the sufficient proof failed.

These five failures are retained in `r4-brep-sweep.log`. Their causal relationship
to other integrated baseline changes has not been established by a clean A/B
control; no claim is made that they are harmless or pre-existing. The completed
native audit is recorded separately from the failed BRep gate; the gate remains
failed. The new original quintic body's separate global cap-injectivity refusal
is described below and is NOT counted as one of those five failed expectations.

## Implemented proof

The private original-cofactor factory now constructs a rational symmetric Gram
candidate from half-exponents of positive even original monomials. For every
original coefficient it selects a matching monomial pair, preferring diagonals
and then the closest exponent pair. This deterministic proposal is not a
numerical fit or complete SOS search. Exact reconstruction must prove
`p = vᵀGv` or `-p = vᵀGv` coefficient by coefficient. Rational symmetric
elimination then requires every pivot to be positive, or zero with a completely
zero remaining row. A negative pivot, unsupported support, coefficient mismatch,
zero polynomial, exact arithmetic failure or exhausted caller ledger grants no
certificate. Every search, allocation, reconstruction and elimination uses the
existing PredicateContext; its one-million-work and expansion ceilings remain
unchanged. There are no new dependencies, tolerances or acceptance flags.

The cofactor `s⁴+s³t+s²t²+st³+t⁴`, arising from `(s⁵-t⁵)/(s-t)`, is now
certified by a 3×3 positive rational Gram matrix (basis `[t²,st,s²]`, diagonal
`[1,1,1]`, adjacent entries `1/2`, remaining entries zero; exact LDL pivots
`[1,3/4,2/3]`) although both previous direct
square-completion and monomial AM–GM constructors refuse it. A real original
polynomial strip `(q⁵,q²+q,v)` is continued through the critical X projection.
The physical strip is regular at q=0 through Y; global cofactor sign and the
existing monotone-graph/divisibility theorem prevent a branch switch there.
One-ULP indefinite, nonzero off-diagonal without diagonal support, zero
polynomial and work-minus-one regressions preserve refusals.

## Original B-spline geometry and scope

The public fixture is `C(q)=10(q⁵,q²+q/4)`, q=2u−1, extruded in z. Its positive
integer-weight target has the exact Möbius clock u/(2−u). Authored Cartesian
poles are dyadic; checks independently evaluate both original parameterizations.
The core and native API independently require regularity, preserve originals,
refuse work-minus-one and one-ULP pole damage, and distinguish a G1-only
curvature perturbation from G2. The companion `(q⁵,q⁴)` source has real rank
loss and refuses even with forged success flags.

The explicit low-level original strip/cofactor test exercises the new Gram
route. The public API may legitimately use its established earlier proper-map
path; no claim is made that the new fallback is necessarily selected by that
public Möbius example. The preceding arbitrary nonlinear Motzkin pullback
examples still do not establish that their cofactors arise from a complete
physical sweep/body source.

A second native test closes the same quintic profile by its chord and extrudes
it over z∈[0,2]. Its exact profile area is 400/7 and volume is 800/7. The
regular curve is simple because X=10q⁵ is strictly monotone. The chord is
strictly above the interior curve: its height difference is
`10(1-q²+(q⁵-q)/4)>0` for −1<q<1. Physical caps and side/chord joins are sharp:
whole-boundary G1/G2 must not be promoted from local smoothness.

The source fixture authors caps in literal world XY charts, keeping original
edge XY polynomials as pcurves. This avoids rounding thirds when normalizing
the control-hull bounding rectangle. It changes source UV representation,
not physical cap geometry; the production prism constructor is unchanged.
Native whole-boundary correspondence, regularity, global geometry and
smoothness are separately recorded for original and imported atlases.

STEP restores an oriented UV atlas that is not an index-identical common-UV
correspondence to the source. The common-UV source-versus-import query must
refuse with `boundary-correspondence-mismatch`. Native identity comparisons
within each original/imported atlas are separate certificates, not a proof of
cross-atlas continuous error. Independent OCCT checks physical export geometry.

## General audit and regression repair

The integrated source snapshot had a pre-existing world-trace regression:
a rounded nonbinary knot reversal returned `Indeterminate(MissingProof)` before
the complete normalized clamped-border owner could establish `Different`.
The verifier now gives that complete original-border owner priority over the
sufficient independent trace shortcut. Periodic/nonclamped inputs still use the
existing fallback. No failed trace gains equality credit; the original regression
and the complete world-trace suite must pass unchanged.

The native matrix also exposed a pre-existing valid periodic torus refusal:
the vertex-link graph had merged both ends of each closed edge into one node.
It now uses `(edge ID, endpoint index)` nodes and actual incoming/outgoing ends.
For a nonclosed edge, the vertex uniquely determines its endpoint, so this is
an isomorphic relabeling of the old link. A closed edge retains two distinct
ends. Edge incidence/direction, fan connectivity and cycle-degree checks remain
mandatory; pinched fans, excess uses and equal-direction uses still refuse.
All 19 topology tests, including original periodic/link negative regressions,
are included in the final gate.

`qualify-r4.py` checks the selected source snapshot before every phase and final
completion, and binds owned files to main. It runs topology, predicates, all continuity
checks, complete world traces, BRep sweep tests, native protocol suites and
no-codec compilation. The final test profile uses optimization level 1 with debug assertions and
integer overflow checks explicitly enabled. This changes compilation, not work,
cell, degree, deadline or tolerance limits; no performance comparison against
the earlier unoptimized runs is claimed. The old unoptimized BRep run was
stopped after its source became obsolete due to the topology correction; its
partial results are retained and excluded from final passing counts.
Main has concurrent unrelated edits; the complete live
dirty main and WASM are not claimed qualified. Source and failed attempts are
retained separately, with budgets and actual test commands in the receipts.
An initial snapshot omitted nine external `include_str!` fixture files required
to compile BRep unit tests. They were copied unchanged and source-bound; no tests
were excluded or ceilings changed to resolve that harness error.

A fresh replay of the earlier moving-matrix nonplanar closed carrier constructs
and restores STEP, recomputes joint continuous/global/G1/G2 certificates and
preserves zero-budget, closure-gap and interior-fold refusals. Its analytic
reference volume is computed by a separate standalone Rust formula. OCCT imports
the fresh STEP independently; the reference is not native mass integration.

## Independent STEP evidence and oracle limitation

`verify-quintic-occt.py` checks validity, unique topology, analytic signed volume,
28 analytic parameter-point values, 15 material classifications and OCCT
level-5 V/V through F/F interference search. It imports only the emitted STEP,
not our geometry evaluator. The absolute geometric/volume threshold is 1e-7,
unchanged from the prior independent moving-carrier check.

The first closest-point diagnostic returned distance 0.073 despite matching
explicit STEP parameter witnesses, valid topology, analytic volume and no
interferences. It cannot certify the global closest distance in this case. This failed oracle attempt is retained.
The final geometry acceptance uses direct original-to-STEP parameter evaluation
of the known degree-five wall, with the closest-point diagnostic still reported
but not used as a global optimizer certificate. This is numerical independent
evidence, not an exact Rust global embedding theorem.

The new integration test initially needed a Rust borrow repair when accessing
loop coedges and edge data through the Model dereference wrapper. It now borrows
disjoint fields of `model.0` directly. Failed compilation and attempted intermediate
qualification receipts are retained; final acceptance uses the single r4 source.

## Located global gap on the new original body

With exact XY cap charts, original and imported self-atlas comparisons have
`continuousBound=true`, zero error, independently proven regularity and exact
edge/surface binding. Neither becomes a global Solid certificate. A fresh
Rust per-face diagnostic locates `loop-simplicity-unproven` on cap faces 2 and 3;
wall face UV loops are proved. Later intersection/nesting/orientation stages
remain gated/unproved, not geometric counterexamples. OCCT validity, analytic
volume, material classifications and absence of detected interferences support
that this is a native proof-coverage gap on a valid fixture. They do not replace
that proof or turn its `certified=false` into true.

## Remaining boundaries

No complete real-algebraic CAD/positivity algorithm, arbitrary inverse maps,
arbitrary singular/ambiguous branch graphs, missing intrinsic source properties,
or complete sweep mode matrix is claimed. New local Gram sign evidence does
not independently certify caps, source-body embedding, nesting, shell orientation
or full retained-boundary G1/G2. Exact STEP atlas equivalence and every currently
unproved global stage must keep their refusal. No WASM or UI actions, focus
interaction, publication, commits or cleanup are part of this milestone.
