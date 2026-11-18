# Compensated product endpoint jets — 2026-11-03

This is native geometry progress, NOT closed-body acceptance.

## Change

The degree-10 common-denominator central network now reauthors its endpoint
first and second control points from the original degree-5 source-cell row jets.
Compensated two-component arithmetic is used for the proposal, especially the
cancellation in rational second derivatives. It is NOT proof arithmetic and
never grants G2, source identity, material or curvature certificates.

After P1 has rounded to its actual stored binary64 value, P2 is solved using
that stored P1's actual first derivative. Reusing the ideal unrounded target
first derivative magnifies rounding error through the tiny endpoint weight.
Source surfaces, weights, boundary roots, knot clocks and tolerances are not
changed. Two endpoint 2-jets are only restored when their unknown control points
do not overlap (product degree >=5). Every changed result needs fresh admission.

## Actual unequal-radius corner result

Fixture: radii 0.1/0.2/0.35 mm, stock 10 mm, central handle 1/256 mm.
Position/normal/tensor tolerances remain 1e-7 mm, 1e-5, 1e-4/mm; requested
absolute principal-curvature cap remains 10/mm.

Five patches are built. The original-source paired inspector still proves
violations at ALL FOUR joins:

| Seam | Normal witness lower | Tensor-jump witness lower, /mm |
| --- | --- | --- |
| 0 | 3.7991027265746443e-7 | 7592.41356301249 |
| 1 | 1.0480371212236539e-7 | 1463.3315415287686 |
| 2 | 7.494606993254378e-8 | 0.19715022005024527 |
| 3 | 1.2316887758963006e-9 | 1.7504371964104608 |

These are original-source counterexample LOWER bounds. Their reduction from
previous snapshots is not an upper bound, whole-seam G2 proof or cap proof.
The original single-surface sewing gate is unchanged and correctly refuses
its still-unproven exterior G2. No network is published as a closed body.

## Next geometry work

Reconcile the actual original half-rim second-derivative clocks and displacement
fields together. Merely changing denominator gauges and repairing product
rounding is insufficient. Preserve full original boundary images and nominal
radius qualification while rebuilding the neighboring transitions. Then repeat
all four internal and six exterior G2 checks and whole-patch absolute curvature
before trimming and body assembly. No tolerance widening or skipped seam is
permitted.

## Evidence

`native-fixture.txt` records the actual diagnostic run; exit status 0 means the
report completed, not that geometry was admitted. `source-sha256.json` identifies
the source snapshot. `tests.txt` records the focused regression result.

Focused `region_replacement::` regressions: **44 passed**. The new tests include
compensated cancellation and the stored-P1 dependency, plus whole-curve
original-boundary agreement for both ends of a rational degree-3 -> degree-6
product fixture at 1e-7 mm, with unchanged roots and original source. This is
not a proof for all external boundaries of the actual corner network.

Reproduce with the repository toolchain:

```sh
cargo test --offline --manifest-path crates/Cargo.toml -p brep-core --lib region_replacement:: --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102
cargo run --offline --manifest-path crates/Cargo.toml -p brep-core --example trihedral_corner_curvature --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102 -- --quartic-profile 0.00390625
```
