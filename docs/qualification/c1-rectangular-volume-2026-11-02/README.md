# Complete C1 rectangular carrier-volume certification

Implementation is Rust-owned. This packet records a focused qualified milestone;
the full BRep suite is pending and the larger original loft/sweep/G1/G2 goal
remains OPEN. Prior failing-regression packets are retained without rewriting.

## Mathematical contract

Gale and Nikaido, Theorem 4, Mathematische Annalen 159 (1965), 81-93:
https://doi.org/10.1007/BF01360282
Primary text: https://fig.if.usp.br/~marchett/fluidos/jacobian-matrix-global-univalence-mapping_gale-nikaido.pdf

The new path requires an open parameter-box volume, a single positive rational
pose piece and both original rational carriers with sufficient literal C1 knot
multiplicities. Sorted valid knots and positive original weights are validated
before those premises are used. Other cases retain the global metric/pair path.

ONE fixed output transform and ONE positive diagonal parameter scale apply to
the ENTIRE cube. Every source subcell must prove all seven principal Jacobian
minors strictly positive with outward arithmetic. Source points only reject
proposals. The original boxes cover the transformed rectangular parameter
domain. The theorem gives global injectivity; the positive full minor gives
regularity, and compactness gives a homeomorphism onto the image. Restriction
to validated common material trims preserves their topology. World orientation,
model boundary binding, source approximation and geometric G1/G2 stay separate
mandatory constructor obligations. No smoothness request is downgraded.

## Source enclosure and resource ownership

The original rational cap-difference owner now restricts coefficient numerators
H'W-HW' before quotient evaluation, intersecting with its independent previous
jet enclosure. This retains algebraic cancellation for unequal original weights.
Original poles/weights/knots are retained. Degree-limited optional product nets
have a charged coefficient census; higher degrees keep the previous enclosure.
Every failed source query and extra restriction consumes the same bounded ledger.

Proof-directed subdivision reuses only fully verified child boxes under that
same fixed predicate/transform/scale, keeping every unresolved sibling pending.
No partial cover grants global flags. The existing 1024 visit ceiling, 100000
aggregate constructor cell budget, work limits and tolerances were not raised.

## Verified native evidence

Toolchain: nightly-2026-09-16, locked Cargo sources, test opt-level=1,
test/dev debug info disabled, two build/test threads.

- Exact captured BRep volume fixture: **1 passed**, 3.70 seconds. Global
  injectivity/regularity/topology all certified at **6671 cells / 276 exact work**.
  Measured budgets replay; each one-less budget refuses.
- Full `nurbs-core --lib`: **2233 passed / 0 failed / 1 ignored**, 56.79 seconds.
  Includes the exact formerly failing fixture, all seven-minor enforcement,
  a non-SPD P-matrix, a smooth time-fold despite nonsingular affine frames,
  a fold hidden between initial probes, C0-knot rejection, quotient derivative
  sign proof, and original curve/frame/cap/closed-seam adversaries.
- BRep `rational_hollow_cap_quarter_turn_frame`: **1 passed**, 246.33 seconds.
  The FULL constructor returns a certified source-bound body with a real hole,
  independent rational cap weights and a nonplanar end cap. Actual admission:
  **253606 work / 6875 cells**. Source-to-boundary error upper bound:
  **0.031261117520197695**, unchanged target 0.05. Cap proposal bound:
  **0.000203281392892496**, unchanged candidate tolerance 0.001.
  Measured constructor work/cell budgets replay; each one-less budget and the
  strict 1e-8 cap tolerance refuse. Inputs are unchanged. Cap-wall joins are
  explicitly **C0**; the source profile classifications remain owned.

The full remaining BRep run excludes ONLY that already completed exact test;
its eventual report plus the focused log will cover the complete test inventory.
`source-before-full-brep.json` inventories workspace Rust/Cargo/JSON inputs;
external dependency revisions/checksums remain pinned in Cargo.lock.
No WASM, browser, STEP or CI completion is claimed here.
