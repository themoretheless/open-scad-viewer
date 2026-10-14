# Affine closed miter cavity orientation qualification — 2026-10-04

Inspected the Rust shell nesting, shell relation, boundary embedding and volume
audit paths. Volume certification requires fresh embedded-boundary evidence,
complete shell relation/parent roles, and independently correct material
orientation. A positive boundary is insufficient for positive solid volume.

Extended the existing affine closed hollow miter test in
`crates/brep-core/src/volume_validity.rs`:

- Outer/inner periodic circle profiles, square closed path, constant affine axis
  scale [2,1,1] and center displacement [.125,0,0].
- Original model retains positive volume certificate, outer outward and inner
  inward, immediate cavity ownership and exact boundary witness classification.
- Clone model, reverse all inner-shell face uses, rebuild topology identities.
  Boundary remains proven, nesting roles remain consistent, inner orientation
  becomes outward, and volume is unproven. The audit preserves its input.
- Existing removal of a fresh cross-shell disjoint pair still prevents the
  exact-witness nesting fallback from assigning parents/roles.

Verification:

- `cargo test --locked --manifest-path crates/Cargo.toml -p brep-core --lib volume_validity`:
  7/7 tests pass in 65.90 seconds. Log: `sweep-affine-cavity-orientation-native.log`.
- `cargo test --locked --manifest-path crates/Cargo.toml -p brep-core --lib shell_`:
  32/32 tests pass in 21.96 seconds. Log: `sweep-shell-nesting-orientation-native.log`.
  Includes shell nesting, shell relation, shell orientation, material islands,
  partial budgets, rational spheres, swapped/rotated shells, touching refusal,
  and related shell ownership regressions. Some cases overlap the first run;
  these counts are not asserted as 39 distinct tests.
- Scoped diff check passes. Only Rust test coverage changed; no production
  algorithm/ABI change and no new WASM rebuild required for this test addition.

This qualifies orientation refusal and nested material-role behavior for the
selected affine closed hollow sweep. It does not prove all authored frame/guide
or moving-law combinations, arbitrary nesting, or universal embedding.
