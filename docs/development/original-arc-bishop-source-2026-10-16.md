# Original geometric arc-clock Bishop source: development checkpoint

This is unfinished successor work to qualified `1c58ccd30`; do not borrow its
3361-test/14-WASM qualification for changed source. The full original loft/sweep
volume/G1/G2 goal remains active. The primary dirty checkout is untouched.

## Implemented in Rust

- Exact original Cartesian C0 and forward unit-direction C0 predicates, including
  nonclamped active endpoints. Position and tangent are independent premises.
  Raw full-multiplicity predicate inputs never use a saturating C0 shortcut;
  native Curve still requires continuous B-spline storage.
- Rational formal-jet fractions reduce by one exact common homogeneous factor;
  numerator/denominator binary rescaling preserves the value. Literal original
  Bezier endpoint homogeneous differences avoid a rounded derivative proxy and
  avoid artificial overflow/underflow of unreduced formal de Boor fractions.
- New `certify_original_arc_rmf_transport` proves original connected positions,
  common forward tangents and complete positive-speed Bishop/holonomy covers.
  The existing public Cartesian-parameter API remains strict. All failed exact
  trials and integration/correction cells share their original allowance.
- Spatial Sweep owners select the same clock policy through approximation,
  boundary error, endpoint material and original filled-volume proof.
- Arc source frame jets require the exact geometric tangent jets, not periodic
  raw speed. Parameter-mode speed requirements remain. Complete original
  boundary translation is checked in its selected station units; scalar and
  affine station laws keep their Cartesian periodic jets.
- Arc inverse lengths refine original integral enclosures under the same budget
  to remove the fixed coarse inverse-phase uncertainty floor. Refinement widths
  are proposals, not geometric admission conditions. A tiny cell at a raw speed
  jump retains its rigorous one-sided hull integral.
- Atlas metric SPD can use outward leading principal minors with ONE constant
  metric/coordinate scale over its entire hull. A rounded inverse is not proof.
- Whole-interval invertibility has a Neumann residual test:
  `||I-R A S||2 <= ||I-R Ac||F + ||R||2 ||A S-Ac||F < 1`.
  Gram bounds enclose `||R||2`. Every matrix in the original Jacobian box is then
  invertible, including averaged Jacobians. This is not pointwise determinant
  admission. Fixed-dimension interval tests and bounded metric proposals are
  charged by their enclosing atlas visits.

Shared native fixture: `crates/nurbs-core/tests/support/arc_bishop_source.rs`.
Its original degree-nine rational spatial loop is geometrically unchanged by
weights `w_i * 2^i`, i.e. positive Mobius time `phi(t)=2t/(1+t)`. Raw endpoint
speeds differ by factor four. Native/JSON/WASM/STEP must reuse this Rust source.

## Evidence so far

- Exact predicates:67 passed before the later interval-atlas additions.
- Arc clock/connected-source/budget native tests:3 passed.
- Independent Bernstein/RK4/Simpson original holonomy and first/second station
  derivative oracle:1 passed. Geometry, length and holonomy are unchanged by the
  exact clock transformation; raw parameter mode still refuses.
- Existing complete original-boundary smoothness regressions:3 passed.
- Whole-metric interval SPD regression:1 passed.

## Latest intrinsic original-body proof

A fresh native original-volume example now passes for the SAME exact Mobius
re-clocked spatial source and unchanged material, geometry and tolerance.
`osv-arc-bishop-intrinsic-pair-cover-v2-tests-20261016.log`:1 passed,43.65s.
The earlier whole-world-Jacobian attempts remain honest work-limit refusals.
The final tiny charging/diagnostic-cleanup edits still require a fresh run.

The new sufficient class is an original arc-clock Bishop frame, constant unit
scale, a literal fixed carrier in the exact initial transverse plane, no guide,
authored affine/frame laws or contact shift. A fresh whole-carrier contraction
proves Q injective and rank two; original coefficient predicates prove the plane.
With source length L, carrier radius rho, tangent rate K and full angular rate
Omega, every local physical chart uses ONE exact orthonormal endpoint frame.
The outward inequality `2*(Omega*d)^2 + (K*d+rho*Omega/L)^2 < 1` makes all
averaged Jacobians nonsingular on a convex disk x temporal lift. The same frame
covers both pieces of a wrapped periodic chart. Distant original sections lie
within rho of their original source-path point and are strictly separated by
outward world boxes. An exhaustive time-pair cover, including every diagonal,
then proves original embedding, regularity and relative trim topology.
The carrier premise covers both complete UV factors, not just sampled profiles.

Direction-sensitive skew-ODE Duhamel bounds improve the original Bishop normal
and tube enclosures. Closed endpoint strips also use the exact return of the
corrected terminal normal to the initial one and a charged angular-speed bound.
The independent first/second-jet oracle and adversarial complete-cover tests
pass3 cases in `osv-arc-bishop-closure-anchor-tests-20261016.log`.

The first actual BRep/G2 run refused later in the wall error owner after its
100000-cell allowance; its original volume proof passed with101246cells,
34154exact work and111 original path boxes. It returned no model.
A new original arc-station chord remainder uses complete original velocity and
acceleration jets, original length bounds and inverse-length residuals. It
requires a single original span OR a freshly proved positional/forward-tangent
join theorem, so a sharp corner cannot receive quadratic error credit.
Two focused remainder tests and one exact nonaxial carrier-plane test pass.
The latter checks one binary64 step off the source plane and zero/one-short
exact-work budgets. The complete global example now also tests zero/one-short
cell and exact-work budgets, and an exact twice-traversed regular source loop.

The next body run (session63082) uses4096 Bishop integration steps; changing
proof resolution does not change the original/retained construction, geometry,
65 stations or0.05mm tolerance. Its log is
`osv-arc-bishop-intrinsic-chord-g2-body-tests-20261016.log`.
Full NURBS development tests also run separately (session84595,
`osv-arc-bishop-full-nurbs-development-tests-20261016.log`).
Those development runs are terminal: the body reached a complete wall bound
but refused its unchanged0.05mm error budget (upper0.09336208554842408mm,
58924source cells). Full NURBS reported1602passed/18failed; most failures exposed
one-short-budget fallback handling, with extra refinement work also exhausting
an existing spatial case. They are not successful qualification evidence.
The subsequent focused76 error-owner regressions passed. Current source fixes
budget-stopped refinement/pair queries and only proposes extra chord work when
the coarse position term alone exceeds the requested tolerance. Adaptive
original derivative subdivision tightens that chord estimate without changing
geometry or tolerance. Current body run78937:
`osv-arc-bishop-refined-chord-g2-body-tests-20261016.log`.
No current full qualification/WASM/STEP claim.
Temporary `OSV_ARC_ATLAS_DIAG` instrumentation has been removed.

Logs use `/private/tmp/osv-arc-bishop-*20261016.log`.
Build target: `/private/tmp/osv-arc-bishop-source-target-20261016`.
Pinned toolchain/lock unchanged: nightly-2026-09-16.

## Required next work

1. Finish the full original global atlas and actual G2 BRep for this SAME source;
   retain parameter-mode refusal and zero/one-short no-model cases.
2. Add native JSON/STEP/WASM qualification using the shared original fixture.
3. Fresh full native/architecture checks, private WASM rebuild, guarded reports
   and independent STEP. Previous source guards do not qualify this successor.
4. Continue arbitrary frames/nonplanar physical end-cap atlases/closed clocks
   beyond these sufficient classes; keep explicit sharp C0 and unresolved refusal.
5. Public fingerprints/browser/Rush/UI/full semantic G1/main/CI remain separate.

Publication is not authorized for this successor. The pending human question
names ONLY `codex/source-fibered-caps` through `1c58ccd30` to the existing GitHub
repository. Do not treat elapsed time or goal continuation as its answer.
