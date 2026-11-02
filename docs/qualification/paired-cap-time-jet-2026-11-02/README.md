# Paired cap temporal jet in proof coordinates

Base: `c77ed95ced93e8646c81b6c42e85a21fa227c2b4`.

A fixed exact column-factor coordinate change must act on the interpolated
carrier temporal derivative as well as its value and UV derivatives. Otherwise
the transformed enclosure omits part of `A(C1-C0)`. The patch transforms both.
The new regression independently differentiates
`F=(u+.5(v+t/4),v+t/4,2t)` and requires the full time column
`(1/8,1/4,2)` at four times, with narrow outward enclosures.

Native commands used `nightly-2026-09-16`, `--locked`, two build/test threads,
`CARGO_PROFILE_TEST_OPT_LEVEL=1`, and test/dev debug info disabled.

- `cargo test --locked --manifest-path crates/Cargo.toml -p nurbs-core --lib paired_cap_time_jet -- --nocapture`: 1 passed.
- Same command with filter `analysis::affine_volume`: 26 passed, including folded-map and budget refusal tests.
- Same command without filter: **2221 passed, 1 failed, 1 ignored**, 149.76 seconds.
  Failure: `analysis::curve_surface_agreement::tests::rounded_complement_is_not_an_exact_nurbs_basis_reversal`
  expects `Different`, receives `Indeterminate(MissingProof)`. This full-suite failure remains unresolved;
  the packet is not a full qualification or permission to publish.

No budgets, tolerances, admission predicates, public paired-cap proof path,
WASM, or TypeScript were changed. Enabling proof coordinates for general paired
caps and qualifying the moving quarter-turn complete volume remain open.
Full original-body/global G1/G2 scope remains open. Logs are immutable gzip files;
the source hashes identify the exact tested patch and lockfile.
