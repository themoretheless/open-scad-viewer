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

Full new original-volume and BRep examples still require a successful proof.
Earlier runs refused honestly at the explicit original atlas budget (300000,
then1000000), with no model. Raising Bishop resolution1024->4096 changed proof
accuracy, not geometry or the0.05mm tolerance. Diagnostics exposed inverse-clock
precision and expensive unsuitable fixed-metric probes. The latest whole-box
Neumann test and global example are being verified; inspect current logs/sessions.
Temporary `OSV_ARC_ATLAS_DIAG` instrumentation is test-only and must be removed
before freezing qualification source.

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
