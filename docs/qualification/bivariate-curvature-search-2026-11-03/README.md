# Bivariate bending and sampled curvature proposal — 2026-11-03

Native Rust construction progress. NOT a qualified closed body, full G2 network,
whole-patch curvature certificate, source B-rep agreement, volume, embedding,
STEP or WASM/UI acceptance.

## Two-dimensional rational bending proposal

Only control points at least three rows from EVERY boundary are unknown. All
boundary two-jet poles, shared seam root columns, weights and parameter clocks
are copied literally from the input degree-10 by degree-7 network. This preserves
those input boundary jets, not an unverified original-B-rep contact certificate.

A 7x7 parameter grid forms rational tensor coefficients for Suu, Suv and Svv,
including actual rational denominator derivatives and original chart widths.
The quadratic proposal minimizes sampled parametric bending (mixed derivatives
weighted by sqrt(2)), with a small bias toward original free poles. Sixty-four
projected coordinate sweeps keep candidates inside original stock control bounds.
The iteration count does not assert convergence, geometric curvature or volume.

Independent source-evaluator tests compare all three derivative coefficient
families with native original surface derivatives. A rational bump fixture is
reduced while every boundary two-jet pole and input geometry stays unchanged;
full original G2 is independently inspected along both external V boundaries.

## Approximate curvature search

A second proposal searches free pole coordinates using approximate principal
curvature on a 5x5 grid. It has an explicit shared allowance of 50000 source
surface evaluations across all five patches, 2/4/6 sweeps, and a total construction
allowance of 1000000 units. Each step halves a bounded initial displacement;
weights, clocks and every boundary two-jet pole remain literal. Work exhaustion,
invalid bounds or absence of a regular sampled candidate refuses the search.

The approximate Gaussian/mean-curvature score is NEVER admission evidence.
Independent outward interval shape-tensor bounds on actual resulting surfaces
can reject candidates, while successful points cannot qualify whole domains.
Stock control containment does not establish volume or absence of intersections.

## Endpoint witnesses

The original patch-pair G2 inspector now checks actual seam endpoints when its
initial whole-interval cover fails. This uses the same shared cell allowance.
Point bounds only reject; a whole-cover proof remains necessary for acceptance.
A degree-16 V regression has an endpoint curvature defect above tolerance but a
midpoint jump below tolerance: it is rejected within six cells. An exhausted
allowance remains Unresolved. Previously successful first-cover proofs incur no
extra endpoint work.

## Read results

`native-fixture.txt` and `result.json` record actual unchanged radii
0.1/0.2/0.35 mm, stock 10 mm, handle 1/256 mm, positional tolerance 1e-7 mm,
normal tolerance 1e-5, tensor tolerance 1e-4/mm and requested cap 10/mm.
The candidate is rejected until ALL required gates pass. Program exit code 0
means diagnostic reporting completed, not that sewing or the operation succeeded.

## Verified results

54 B-rep region regressions and 45 original-surface G2 regressions passed.
All construction/search variants remain rejected by independent curvature point
witnesses above 10/mm. On the SAME 5x5 grid, the largest observed witness LOWER
bounds are:

| Search sweeps | Largest point witness lower, /mm |
| --- | --- |
| 0 (bending proposal baseline) | 380.94154668012897 |
| 2 | 172.19915434051325 |
| 4 | 160.33413227646298 |
| 6 | 159.5140264264739 |

These numbers do not bound the true global maximum. The denser grid reveals
additional defects that the earlier 3x3 grid did not test. No successful sample
set is interpreted as a whole-domain certificate.

After six sweeps, patch witness lowers are 117.29567203032896,
16.095077638664854, 5.674620493027951, 12.520642847639111 and
159.5140264264739/mm. Patches 0/1/3/4 are rejected; patch 2 remains unqualified.

Full original internal seam G2 remains proven for seams 1 and 2. Endpoint-aware
inspection still leaves seams 0 and 3 Unresolved at 6000 cells each. Fixed boundary
jets prevent the interior search from repairing these two outstanding proofs.
All six original exterior contacts, full curvature/material/volume/embedding,
source-face trimming and closed-body assembly remain unqualified for this network.

Further work must change the actual jointly built transition geometry or its
parameter correspondence and verify the same unchanged constraints. More search
iterations or lower sampled scores alone cannot complete the requested result.

## Reproduce

```sh
cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core --lib surface_g2_pcurves:: --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102
cargo test --offline --manifest-path crates/Cargo.toml -p brep-core --lib region_replacement:: --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102
cargo run --offline --manifest-path crates/Cargo.toml -p brep-core --example trihedral_corner_curvature --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102 -- --quartic-profile 0.00390625
```
