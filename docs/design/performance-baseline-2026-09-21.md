# Performance Baseline: 2026-09-21

## Environment

- Apple M4 Max, macOS 25.6.0, arm64
- Node v22.23.2, V8 12.4
- Source: `8b267aecb4ea52a1aeb58f1a145e3828c48a79f5`
- Rust release profile, locked dependencies

## NURBS Process

Command: `npm run bench:nurbs-process -- <report> 5`

The benchmark starts a fresh disposable process for each request. The first
iteration is cold; p50 below is over five samples and includes startup,
validation, evaluation, transfer, and child shutdown.

| Fixture | p50 | min | max |
|---|---:|---:|---:|
| union | 348.532 ms | 345.350 ms | 456.925 ms |
| intersection | 349.023 ms | 344.193 ms | 423.471 ms |
| difference | 348.856 ms | 346.096 ms | 418.679 ms |
| large STL | 418.812 ms | 409.059 ms | 482.496 ms |

All repeated response hashes were identical. This measures the process
boundary, not isolated native kernel throughput.

## Warm Solid Analysis

Command: `npm run bench:analysis -- --samples 3 --warmups 1`

The dominant measured phase is display preparation (`mesh_render::render`),
not BVH or semantic-edge extraction:

| Fixture | render p50 | combined p50 |
|---|---:|---:|
| sphere-128 | 6.976 ms | 11.936 ms |
| three-spheres-128 | 21.544 ms | 36.312 ms |
| cylinder-128 | 0.231 ms | 0.320 ms |

The render path must preserve face order, crease classification, property
vertex merge records, and byte-parity tests. Any optimization should compare
the existing reference implementation and deterministic output signatures.

## Render A/B Candidate

The first candidate replaces iterator overhead in the hot corner loop with
explicit three-component accumulation and output writes. With five samples
and two warmups, the same retained-handle benchmark measured:

| Fixture | Before render | Candidate render | Change | Before combined | Candidate combined |
|---|---:|---:|---:|---:|---:|
| sphere-128 | 6.976 ms | 6.621 ms | -5.1% | 11.936 ms | 11.439 ms |
| three-spheres-128 | 21.544 ms | 20.551 ms | -4.6% | 36.312 ms | 34.536 ms |
| cylinder-128 | 0.231 ms | 0.180 ms | -22.1% | 0.320 ms | 0.262 ms |

The candidate passed the geometry-bridge byte-parity tests and the full
workspace clippy gate. The generated geometry WASM artifact was rebuilt from
the candidate before publication.

The follow-up reserves merge vectors to the bounded incident-triangle shape,
avoiding repeated reallocations. A second five-sample run measured:

| Fixture | Candidate render | Reserved render | Candidate combined | Reserved combined |
|---|---:|---:|---:|---:|
| sphere-128 | 6.621 ms | 6.254 ms | 11.439 ms | 11.048 ms |
| three-spheres-128 | 20.551 ms | 19.348 ms | 34.536 ms | 33.907 ms |
| cylinder-128 | 0.180 ms | 0.154 ms | 0.262 ms | 0.236 ms |

## Native Validation

Command: `cargo run --release --locked --manifest-path crates/Cargo.toml -p
brep-core --example bench_validation -- --profile quick`

Median wall times were 14.762 ms for 12 teeth, 40.342 ms for 32 teeth, and
50.343 ms for 60 teeth over eight observations. These are native validation
measurements and should not be compared directly with the disposable NURBS
process timings.

## Next A/B Boundary

Profile `mesh_render::render` in isolation. Compare a candidate that reuses
per-source adjacency/normal work against the current CSR implementation.
Keep the current implementation as the byte-parity oracle and retain only a
repeatable win on `sphere-128` and `three-spheres-128` without changing
`cylinder-128` or any render regression tests.
