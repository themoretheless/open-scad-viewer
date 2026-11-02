# Exact trim partition over surface knot cells — 2026-10-30

This is a bounded native Rust extension, not completion of the arbitrary
algebraic geometry goal. Legacy v4 atlas code is unchanged.

The existing UV disk proof runs first. If its complete rational control hull
lies in one authored surface knot cell, composition uses that cell directly,
including nonlinear curved disks. Previously the whole surface domain was
requested as one Bernstein cell and any internal knot refused.

For an affine/projective degree-one UV triangle crossing internal surface
knots, exact rational convex polygon clipping partitions it against every
nonempty tensor knot cell. Each polygon is triangulated with positive oriented
area. The original surface cell is exactly extracted and composed for each
triangle. The exact sum of oriented UV areas must equal the original triangle
area. Cell interiors are disjoint; every clipped polygon is contained in the
original triangle. These facts establish local coverage without filling holes
already excluded by the original region proof. No sampling or F64 clipping is
used. Geometry and verification remain in Rust.

Limits: at most 128 tensor cells, at most 512 output triangles per input disk,
existing work/bit/degree guards. Curved UV maps crossing knot boundaries refuse:
no chord substitution or approximate root has been introduced. Adjacent faces
may still require a globally conforming partition; the final atlas incidence,
star and separation checks remain mandatory. No claim of a new accepted full
closed-body class is made from local trim tests alone.

The two new regressions check a triangle crossing an internal knot (three exact
output triangles, outside-domain refusal), and a holed region whose surface is
split through the hole (the hole point belongs to none of the output triangles).

Arbitrary implicit algebraic branches, genuine poles, irreducible degrees beyond
the arithmetic caps and exhausted work budgets remain open. WASM/dist and browser
qualification have not been run for this change.

Validation: focused trim tests **7 passed**, complete `minkowski3d::` tests
**85 passed, 0 failed**. Test wall time 68.81 s is observational, not a benchmark.
Formatting and scoped `git diff --check` pass.

```sh
CARGO_TARGET_DIR=/private/tmp/osv-atlas-20261028-target \
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 \
cargo test --manifest-path crates/Cargo.toml -p brep-core --lib minkowski3d:: -- --nocapture
```

`source-files.json` binds seven atlas files; it is not a complete isolated
workspace snapshot. See `native-tests.log` and `result.json` for evidence.
