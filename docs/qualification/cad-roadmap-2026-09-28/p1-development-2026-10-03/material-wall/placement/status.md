# Placed annular material wall qualification

This follow-up addresses the specific coordinate rotation and translation recorded in `../placed-limitation.json`. It does not certify arbitrary Euler rotations or whole-body minimum thickness.

Two failures had separate causes:

1. Rational planar cap pcurves were lifted through an independently rounded translated plane. Eligible exact coordinate caps now use the transformed original edge coordinates as their plane-chart coordinates. The original edge curves, vertices, surface control-point set and topology stay unchanged. An exact source-lift guard prevents placement from repairing a preexisting mismatch. General operation charts retain their established UV domains.
2. The ruled blend/wall joined-chart proof tried only two world-axis projections. A bounded family of signed coordinate projections now passes the same full original-chart seam, weighted order and dominance checks. Structural failures are rejected before interval subdivision. Every admitted certificate retains its projection and complete 512-cell report.

The specific placed annular wall now qualifies a 15 mm interval over 36 selected face pairs. Exhausted curved-volume work must still refuse a material interval. Automatic search candidates remain samples; convergence comes from complete selected-union lower bounds and a certified interior material chord.

The final focused placed-wall test passes, including 15 mm convergence and refusal with only one face pair admitted (88.93 s). The complete-cap/source-preservation regression passes. The mismatched-source regression passed separately before widening the rational-boundary degree guard; that guard still excludes the perturbed source. All 46 NURBS Boolean tests pass on the final guard. The bridge suite passed 353 tests / 1 ignored before widening that degree guard.

The wider B-rep run used an earlier over-restrictive degree-two cap guard and failed its placed-wall test. It is superseded for that case by the final focused test; this is not a claim of one green full-suite run on the final source. Packaged WASM delivery and final runtime qualification remain pending.
