# Fixed proof coordinates for paired caps

Base: `879a0948b` (includes the independently tested cap temporal-jet fix).

Open paired rational caps now use the existing fixed invertible proof-coordinate
proposal. All original coefficient jets, positive weights, full cap-difference
time terms and whole-volume metric/pair covers remain mandatory. A point jet
only proposes coordinates. Source shear extraction remains disabled for paired
caps. The exact determinant and failed factor trials spend the same ledgers;
no work, metric visit, or tolerance ceilings were raised.

The new moving-cap regression uses a genuine quarter-turn coefficient law and
unequal caps with nonzero temporal carrier derivative. Full embedding,
regularity and parameter-topology claims pass at **79 cells / 10 exact work**.
Measured-budget replay passes; each one-less budget refuses, as does a motion
singular at its midpoint. Existing independent unequal rational carriers pass
at 113 cells / 12 exact work, including both budget boundaries.

A dispatch fix sends fully clamped supported borders to their existing complete
normalized tensor owner, instead of a periodic/nonclamped predicate returning
MissingProof prematurely. The unchanged rounded-reversal regression now proves
Different. No expectation was relaxed.

Native evidence (nightly-2026-09-16, locked sources, two build/test threads,
test opt-level=1 and test/dev debug info disabled):

- Cargo `-p nurbs-core --lib curve_surface_agreement -- --nocapture`: **39 passed**.
- Cargo `-p nurbs-core --lib analysis::affine_volume -- --nocapture`: **27 passed**, 1.62 seconds.
- Same freshly built NURBS unit-test executable run without filter,
  `--test-threads=2`: **2223 passed / 0 failed / 1 ignored**, 81.89 seconds.
- Cargo `-p brep-core --lib rational_hollow_cap_quarter_turn_frame -- --nocapture`:
  **0 passed / 1 failed**, 116.53 seconds. Exact actual boundary agreement passes
  (241560 work), but the complete actual filled-volume atlas still exhausts
  99800 source cells from its unchanged 100000 aggregate cell budget.
  It returns `actual-affine-filled-volume-unproved`, no Solid. This remains OPEN.

Thus this packet proves the new native coordinate path and restores the full
NURBS suite. It is NOT full BRep qualification, full original-body/G1/G2
completion, WASM/STEP/browser qualification, or a green publication gate.
The next step is source-paid subdivision/metric refinement of the actual volume
atlas, preserving complete pair coverage and refusal boundaries. Logs and tested
source hashes are retained without rewriting prior packets.
