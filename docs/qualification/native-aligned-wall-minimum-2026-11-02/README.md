# Native closed-body and aligned-wall qualification

The opt-in exact-source quadratic-core G2 control body has 27 faces, 82 edges, 57 vertices and zero open edges. All 351 distinct face pairs are proven compatible with global embedding. The 19 internal G2 joins and whole-support curvature bound 9.867331764661289 mm^-1 remain below 10 mm^-1.

## Added qualification

Rust enumerates all 378 unordered face products, including all 27 self products. Exact source coefficient boxes prove 44 distance bounds; whole-support oriented unit-normal enclosures exclude aligned entering/exiting chords in the other 334 products. No unresolved products remain at 2 mm, with endpoint-normal sin^2 tolerance 1e-6.

An automatically proposed axis chord is independently admitted using exact source UV inversion, strict interior trim membership, outward entering/exiting normals, and exclusion of all 25 other boundary faces from the complete closed segment. Its exact length is 2 mm, attaining the complete lower bound for this criterion. The small-body regression reverses imported face order so a longer 4 mm candidate precedes the minimum; a 2.1 mm requested bound is rejected by the verified 2 mm witness.

This is a minimum for face-normal-aligned interior material chords. It is not unrestricted reach, medial-axis thickness, edge-normal-cone thickness or arbitrary boundary distance. Witness proposals currently cover opposite imported coordinate planes; failed search does not prove absence. Full support boxes conservatively include trimmed-out regions.

## Verification

58 focused tests passed: 32 native transition regressions, 2 G2-profile tests, 3 source-law tests, 6 support-frame tests and 15 B-rep tests. The expanded example completed within one unchanged 10,000,000 coefficient budget (9,824,019 used). Gzipped logs and source hashes accompany this receipt.

Command: `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo run --offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0 -p brep-core --example expanded_contact_runout -- --exact-source-brep-split --quadratic-core-g2`.

## Open scope

Arbitrary rational NURBS source trimming and general unequal-radius nodes remain unqualified. Four end panels still fail constant R1; the source-derived radius law is qualified only on the explicit transverse core band. Full rolling-ball fillet qualification, unrestricted thickness, packaged WASM/worker/UI, STEP and history acceptance remain open. This native opt-in result is not production application admission.
