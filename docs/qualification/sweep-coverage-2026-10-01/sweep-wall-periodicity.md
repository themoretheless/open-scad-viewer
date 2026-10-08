# Retained wall regularity profile correspondence — 2026-10-04

Rust `Sweep::certify_wall_regularity` checked degree, knots, weights, control
count and dimensions against the authored profile, but omitted `periodic`.
The level error and wall geometry audits already required equal periodicity.
Added that same premise to wall regularity.

Regression uses a valid degree-2 periodic profile with uniform knots and repeated
tail controls. The unmodified retained sweep must have certified wall regularity.
Changing just one section to nonperiodic still passes `Curve::validate`, but must
be refused for changed correspondence. Before the fix this new test fails on the
required refusal: `sweep-wall-periodicity-mutation-before.log`.

An earlier attempted mutation of a clamped linear profile was rejected by the
curve validator already. It was replaced with the valid periodic-to-nonperiodic
mutation above; that first passing attempt is not treated as evidence of the gap.

Post-change verification started:

- native full progressive_miter module tests:
  `sweep-wall-periodicity-native-final.log`;
- geometry WASM rebuild/packaging:
  `sweep-wall-periodicity-wasm.log`.

Completed outcomes:

- Native progressive_miter module: 62/62 tests pass, including the valid
  periodic-to-nonperiodic mutation regression.
- WASM rebuild/optimization/packaging exited successfully. Optimized bytes:
  10731689. Generated kernel and public geometry-kernel.wasm SHA256 both
  `34a4e5be7bd2b9fb9b4156adb42da4c30400e444d50331103594b12d8b27e0e6`,
  matching generated identity.ts.
- Fresh public-WASM regression: five suites / 52 tests pass in 52.38 seconds,
  unchanged timeouts, evidence `sweep-wall-periodicity-public-final.log`.
  Suites: nurbsProgressiveMiter, sweepBoundaryCertificate, miterProfileSmoothness,
  miterStationSmoothness, miterSmoothStationWalls. The added public test verifies
  that valid periodic profile identity is retained in every returned section,
  regularity remains certified, and the authored input is unchanged.
- Direct Vue typecheck passes (`sweep-wall-periodicity-types.log`); scoped diff
  check passes.

This qualifies the periodicity correspondence fix through native and fresh
public WASM. It does not establish global embedding or all-mode regularity.
