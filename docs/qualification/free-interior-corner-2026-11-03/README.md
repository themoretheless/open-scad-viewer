# Two free interior rows for the central corner — 2026-11-03

Native Rust proposal work, not a qualified closed CAD body. Absolute principal
curvature <=10/mm, every external/internal G2 seam, material/volume, embedding,
source trimming, sewing, STEP and WASM/UI acceptance remain mandatory.

## Construction

The existing common-denominator degree-10 by degree-5 network is coupled first.
Its Cartesian V polynomial is then elevated to degree 7. The first and last
THREE V rows stay fixed after elevation; two interior rows are free. A linear
least-squares thin-plate proposal minimizes the sum of squared second control
polygon differences in V with these six boundary rows fixed. A relaxation in
[0,1] blends the original elevated and proposed interior poles.

This is control-polygon fairing, not a surface-curvature optimization or proof.
Polynomial degree elevation preserves boundary two-jets mathematically; actual
rounded geometry still needs fresh original-source inspection. Literal constant
components and collapsed pole columns are retained. No fitted source surrogate
or sampled boundary is introduced.

Construction, initial coupling, fairing and final coupling receive explicit
shares of a single operation allowance. Excessive work refuses the candidate.
The positional guard applies to the boundary two-jet rows. It accumulates an
outward interval L1 displacement for each actual control point and compares it
with the same total budget. Uniform budget/3 allocation to individual axes
needlessly rejected an otherwise admissible one-axis displacement; the total
control-displacement allowance has not been increased.
Free interior rows can move during final coupling because they do not determine
external boundary two-jets; the whole actual control hull is then checked against
original stock planes. Stock containment does NOT certify allowed volume or
embedding. Sequential positional guards are NOT an original-source G0 receipt;
actual source agreement must be checked before admission.

The final actual polynomial-seam G2 inspector supports all V rows, not a hardcoded
six-row subset. Every diagnostic curvature point uses the actual new surface:
a lower bound above 10/mm rejects it, while successful points never admit a patch.

## Tests and experiment

The dedicated fixtures check full original G2 on both external boundaries while
the interior changes, literal collapsed pole ownership, unchanged input geometry,
stock/NaN/work guards, and free interior coupling without spending the boundary
position budget. The actual corner probe tests five relaxation values with the
same radii 0.1/0.2/0.35 mm and handle 1/256 mm. See recorded results below.

The guards are tested in both directions: a one-axis displacement between
budget/3 and budget is allowed and the resulting actual whole-seam G2 is checked;
a two-axis displacement with each component below budget but total above budget
is rejected. These checks concern the actual control-point movement, not a claim
of original B-rep source agreement.

## Recorded result

**49 focused regressions passed.** All five degree-7 proposals now construct
inside the original stock control-hull bounds, with no tolerance increase.
This is control-hull containment, not acceptance of a closed body's volume.

For every relaxation, internal seams 1 and 2 are proven over the complete actual
original seam with regularity and normal/tensor upper bounds within 1e-5 and
1e-4/mm. Seams 0 and 3 remain Unresolved at 6000 cells each. They have no
whole-seam certificate and cannot be skipped.

| Relaxation | Largest checked point curvature lower bound, /mm | Whole-seam G2 |
| --- | --- | --- |
| 0 | 239.1823406092673 | 2/4 proven |
| 0.25 | 238.37524089079128 | 2/4 proven |
| 0.5 | 237.54369464216458 | 2/4 proven |
| 0.75 | 236.68775046826238 | 2/4 proven |
| 1 | 235.80746728604535 | 2/4 proven |

All five shapes are REJECTED by original point curvature witnesses above the
unchanged 10/mm cap (patches 0, 1 and 4). A smaller witness lower bound does not
bound the actual global maximum. This one-direction control-polygon fairing
cannot be treated as completion: the next shape correction must account for
both parameter directions, preserve the boundary two-jets and owned seams,
and repeat full curvature/material/volume/embedding qualification before sewing.

## Reproduce

```sh
cargo test --offline --manifest-path crates/Cargo.toml -p brep-core --lib region_replacement:: --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102
cargo run --offline --manifest-path crates/Cargo.toml -p brep-core --example trihedral_corner_curvature --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102 -- --quartic-profile 0.00390625
```

The example exits 0 when diagnostic reporting completes even if all candidates
are rejected. Inspect `stage=fair_join`, `stage=fair_cap_witness` and the sewing
refusal; this exit code is not a production acceptance check.
