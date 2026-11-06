# Common-denominator central corner candidate — 2026-11-03

Native Rust work only. This is NOT a qualified closed-body, whole-body G2,
absolute-curvature, thickness, embedding, STEP or WASM/UI acceptance.

## Geometry

The explicit quartic profile remains an original-source candidate. The central
single-surface blend has four witnessed internal G1/G2 failures. It must not be
sewn as an accepted replacement. Its three curved exterior joins also remain
unproven; three flat exterior joins are proven.

`CentralLoftCandidate::propose_common_denominator_patches` constructs a NEW
network from the original saturated U cells. It multiplies the two original row
denominators in Bernstein form, so every V control in a U row has the same
weight. Each patch retains an independent binary homogeneous gauge. The new
interior differs from the old rational V blend. Source definitions are unchanged.
Literal source seam roots are copied, not recomputed by multiplying/dividing
small endpoint weights. Literal constant-coordinate endpoint two-jets are also
preserved in the product net; no approximate collinearity is promoted to identity. No inherited G2, radius or closed-body receipt is granted.

For the unequal-radius fixture (0.1/0.2/0.35 mm, handle 1/256 mm), five degree-10
by degree-5 patches are generated. All four shared original root columns match
literally and all four whole-curve position checks pass at 1e-7 mm. Generic
shape comparisons exhaust 6000 cells at each seam and remain Unresolved.

`inspect_original_patch_pair` is a separate bounded original-source G2 inspector
for two clamped cells with literal V-constant weight rows and a shared polynomial
seam. It forms endpoint jets from ORIGINAL weight ratios and stored controls;
there is no rounded re-gauged source surrogate. The shared V jets and paired
normal/tensor differences are checked on a full adaptive cover. Budget exhaustion
or unsupported source structure cannot return WithinTolerance. It grants no
absolute-curvature or body certificate.

## Checks so far

- region_replacement: 42 passed (including two network proposal/guard tests).
- surface_g2_pcurves: 44 passed (including three original patch-pair tests).
- bezier_curvature_bound predicate: 7 passed.
- Independent rational-cylinder fixtures: 3 complete smooth joins proven,
  3 curvature-only defects rejected, including gauges differing by 2^40.
  Normal upper bound <6e-15 and tensor upper bound <2.8e-14/mm for each
  positive local cylinder seam. These fixtures do not qualify the corner.
- Original quartic-profile absolute-cap qualification was previously positive
  at 10/mm for all three radii and both requested profile tolerances; this is
  NOT a cap certificate for the central network or a sewn body.
- Local original flat-pole quotient proofs succeed in their recorded bands:
  4.365676283969965e-11 and 1.5683215485198546e-11 normalized U.
  These extremely local bands do not cover the rest of an exterior seam.

## Required before sewing

1. Prove or correct the actual network G2 on all four seams and all six external
   contacts. The original patch-pair inspector has found actual tensor-jump violations in
   the initial network (lower bounds about 11976.7, 1966.0, 1561.8, 116.6/mm).
   All four normal lower bounds were below 1e-5; this does not grant G2.
   The amended constant-coordinate two-jet proposal is also rejected on all
   four joins: lower tensor bounds 11976.660958676293, 1965.9510468544472,
   1561.8382719155622, 6039.337146264327/mm. Small normal witness lower bounds
   do NOT certify whole-seam normals. Joint geometric reconstruction is needed.
2. Check the actual central patch fields for regularity, absolute principal
   curvature <=10/mm, embedding and allowed material/volume. Local pole proofs
   alone cannot grant these checks.
3. Split original source/contact coedges at owned network boundaries; preserve
   root and face identities and prove the new pcurves against original supports.
4. Assemble trimmed source faces, transition strips and network patches into a
   closed oriented body, then repeat radius/thickness/intersection checks.

The existing single-surface sewing gate is unchanged and correctly refuses this
fixture. Diagnostic example exit status 0 only means its report completed.

## Reproduce

Use the repository toolchain and an isolated target directory:

```sh
cargo test --offline --manifest-path crates/Cargo.toml -p brep-core --lib region_replacement:: --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102
cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core --lib surface_g2_pcurves:: --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102
cargo run --offline --manifest-path crates/Cargo.toml -p nurbs-core --example patch_pair_qualification --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102
cargo run --offline --manifest-path crates/Cargo.toml -p brep-core --example trihedral_corner_curvature --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102 -- --quartic-profile 0.00390625
```

The last command is diagnostic and exits successfully even when sewing is
refused. Read `stage=sewing` and the explicit `admitted=false` result.
