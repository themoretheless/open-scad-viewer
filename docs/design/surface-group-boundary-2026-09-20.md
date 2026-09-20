# Rust surface grouping boundary

The grouping implementation from `themoretheless-rust-surface-grouping` is
isolated in `geometry-bridge/src/mesh_surface_groups.rs`. It preserves current
host sparse-mesh behavior: validate every position, but only weld referenced
positions when vertices outnumber indices. Rendering does not call it yet.

## Conformance

- Seven native tests cover welding, winding, degeneracy, sparse inputs,
  nonfinite unused positions, layout, angle and triangle limits.
- `scripts/check-surface-group-parity.mts` compares exact IDs and errors with
  the current host implementation across 838 deterministic cases, including
  angular threshold neighbors. The native example consumes the same JSON
  fixtures; signed zero and nonfinite values are covered by native tests.
- Nineteen tests across `surfaceGroupsKernel`, `solidAnalysis`, and
  `meshSurfaceGroups` pass against the rebuilt WASM. These cover display meshes,
  subarray uploads, empty results, refusal/recovery, memory growth and transfers.
- Application typecheck and standalone strict benchmark typecheck pass.

## Measurements

Run `node --import tsx benchmarks/surface-group-boundary.mts` after building
the geometry kernel. Node 22.23.2, macOS arm64. Five warmups, fifteen alternating
TS/Rust samples per case, exact ID checks outside timing. No simultaneous local
build/test workload. Allocation and GC remain part of the measurement.

The Rust interval includes host uploads, ABI response decoding, result copy and
lease release. Both sides recompute groups; neither uses the content cache.

| Connected planar strip triangles | TS p50 A/B, ms | WASM p50 A/B, ms |
| --- | ---: | ---: |
| 128 | 0.0997 / 0.0907 | 0.0855 / 0.0865 |
| 8192 | 2.1423 / 2.1387 | 1.9515 / 1.9646 |
| 65536 | 27.8971 / 27.6899 | 16.4012 / 16.4249 |

The large strip improves about 41%; the medium strip about 8-9%. The small
case is noisy. These fixtures do not establish performance for arbitrary
topologies, cache hits, cold startup, or complete scene publication.

Reports: `/private/tmp/osv-surface-group-boundary-a.json` and
`/private/tmp/osv-surface-group-boundary-b.json`, including all samples and hashes.
Optimized WASM is 7,680,174 bytes, +10,580 versus the preceding local artifact;
SHA-256 `bb075a40cc24aa3127343c0ab7679fd068cef8d5b6cf638bb2b2f54845a39379`.
Historical qualification artifacts were not updated.

## Packaging Check

Production Vite build and `verify-dist` pass: 91 artifacts, 5,786,696 asset
bytes plus 9,708,036 raw WASM bytes, 15,494,732 bytes total. The tracked
`public/wasm/geometry-kernel.wasm` is included with this integration, not just
the ignored generated embedded payload.

The Chrome production worker packaging check passes cold initialization, warm
operations, refusal and recovery for SVG and G-code workers. Report:
`tmp/performance/surface-groups-worker-packaging/report.json`.
This is packaging compatibility evidence, not a replacement for qualification
or a full application interaction test.

## Remaining Integration

Do not copy the old branch's export-then-render grouping sequence: it computes
groups twice. Group the final display coordinates once in the producer, carry
an explicit distinction between inferred patches and authored CAD faces, and
retain the host fallback/cache for other mesh producers. Compare full
publication before switching that path. This commit exposes the measured raw
buffer boundary but does not switch selection behavior or claim a full Rust
migration.
