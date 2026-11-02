# Algebraic parameters and exact image clocks — 2026-10-30

This bounded Rust extension has two distinct outputs: portable algebraic event
certificates and a proved planar image-clock normalization used by the atlas.
It does not claim arbitrary algebraic surface subdivision.

## Source-bound algebraic events

`minkowski3d::curved` exports `certify_algebraic_seam_parameters`,
`verify_algebraic_seam_parameters`, `certify_algebraic_knot_parameters`,
`verify_algebraic_knot_parameters`, `AtlasAlgebraicRoot` and
`AtlasAlgebraicParameters`. Seam queries accept a three-dimensional authored
Curve and an exact rational spatial point. Knot queries accept a two- or
three-dimensional authored Curve, coordinate axis and exact level. Ratios and
polynomials use canonical integer strings. Parameters are normalized to [0,1]
on one original knot span. A crossing of unpartitioned curve spans still refuses.

The witness retains a polynomial and a rational interval selecting an actual
root. It never promotes the interval midpoint to an exact parameter. The source
binding includes every original F64 bit of the curve, its degree, knot vector,
weights and periodic flag, as well as the exact query and homogeneous data.

The producer uses the existing Rust algebraic root isolation engine through a
narrow crate-internal facade. The verifier does not trust the producer's search:
it checks one selected root, exact vanishing of every source equation at that
root, positive denominator, disjoint ordered intervals, endpoint roots and the
complete interior-root census. Source side signs are proved using an interval
containing no other zero of the first nonzero source equation. Rational
singletons use [0,0] side markers. Even-multiplicity irrational events remain.
A boundary lying wholly on the knot level is a nonisolated set and refuses.

## Actual atlas support for irrational clocks

When an unmatched edge contains a known rational spatial vertex whose original
parameter is irrational, a planar chart can sometimes eliminate the clock.
Every homogeneous control is checked against its exact corner plane, every
boundary against its corner chord, and the entire projected disk is proved
homeomorphic to that triangle using strict interior Jacobian and a checked
Jordan boundary. Only then is its parameterization replaced by the exact linear
corner map. Neighbor subdivision proceeds with rational coordinates of the SAME
surface image. An arbitrary curved/nonplanar chart is never flattened.

The same proved image normalization is a fallback for nonlinear UV knot
pullbacks. Example: x=(u+u²)/2, y=v(1+u/2). The x=1/2 preimage has parameter
(sqrt(5)-1)/2, but the entire UV image is exactly its corner triangle. The
previous exact tensor-cell clipping is therefore applicable without introducing
algebraic control points or altering the UV region.

## Remaining boundary

General nonplanar restrictions at irrational parameters and genuinely curved
algebraic knot-line pullbacks still require algebraic control nets or verified
semialgebraic source domains. Rational Net controls have not been silently
replaced by root midpoints. These cases remain unproved/refused. Degree, bit,
root-isolation and atlas work limits remain finite. Legacy v4 is unchanged.
WASM/dist, browser delivery and a standalone checker process replay are not
part of this run.

## Validation

**94 native Minkowski tests passed, 0 failed**. After adding the final forgery
assertion (a false rational root strictly inside the genuine isolating interval),
the four focused algebraic tests were rebuilt and all passed. Production Rust
code was unchanged between these two runs. The 94-test wall time (60.25 s)
is observational, not a performance benchmark. Formatting and scoped whitespace
checks pass.

```sh
CARGO_TARGET_DIR=/private/tmp/osv-atlas-20261028-target \
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 \
cargo test --manifest-path crates/Cargo.toml -p brep-core --lib minkowski3d:: -- --nocapture
```

See `native-tests.log`, `final-forgeries-tests.log`, `result.json` and
`source-files.json`. The source ledger is a scoped hash binding, not a complete
isolated workspace snapshot. Unrelated concurrent changes were preserved.
