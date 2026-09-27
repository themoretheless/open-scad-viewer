# Cooperative semantic-edge extraction

The owned Rust analysis job now yields during semantic-edge extraction as well
as BVH construction. Cancelling it drops both phases' scratch without requiring
a new Worker. The production compiler already uses this job whenever it has a
cancellation or yield callback; no second host implementation is introduced.

## Work and ownership

The edge future yields every 4,096 work units while initializing representatives,
following/compressing merge chains, welding positions, building triangle edge
occurrences, sorting, classifying groups and emitting indices. Radix histogram,
prefix and scatter loops yield internally, including the 65,536-bucket path.
Degenerate triangles and long non-manifold groups count toward the budget.

The host returns to its event loop using the existing 50 ms compiler scheduling
interval. The work budget is not a wall-clock deadline. Allocations, render
preparation, volume/area calculations, Boolean operations and selection surface
grouping remain synchronous; the coordinator still provides hard preemption.

The existing two-job/750,000-source-triangle admission limits, monotonically
increasing IDs and `finally` cleanup are unchanged. No result is published until
both BVH and edges complete; copied result buffers survive WASM memory growth.

The synchronous traversal is preserved to avoid charging existing callers for
suspension. Numeric helpers and group classification are shared. Differential
tests compare ordering and diagnostics, including both radix widths, welding,
non-finite coordinates, signed zero, degeneracy, creases and non-manifold groups.

## Verification

The [scoped report](../qualification/cooperative-edges-2026-09-27.json) records
source/artifact hashes and test results. Reproduce the native comparison with:

```sh
rustc --edition 2024 -O benchmarks/cooperative-edges-native.rs -o /tmp/edges-parity
/tmp/edges-parity
```

The native differential checks 18 inputs/modes up to 750,000 triangles. A bridge
test first counts BVH-only polls, then cancels complete analysis jobs after an
additional 1, 20 and 80 edge steps and verifies registry admission is released.
The established edge fixtures also compare the resumable implementation.

For the real browser check after building the application:

```sh
npm run test:cooperative-edges
```

The prior artifact completed this same sphere's entire BVH-only adapter in 179
checkpoints. The BVH source is unchanged; checkpoint 240 therefore exercises the
new edge stage. Four real cancel messages must succeed in one Worker, followed
by a correct cube and a byte-identical full sphere. The harness yields every
eight checkpoints to make message delivery deterministic; measured cancellation
latency is not a production scheduling guarantee. The parser tests also cancel
at this later checkpoint and verify a subsequent build.

Single native timing samples are diagnostic only. Cooperative extraction has
suspension overhead; this change does not claim a throughput improvement or
completion of cooperative cancellation throughout compilation.

## Local results

- 10 polygon edge tests and 4 bridge lifecycle tests passed.
- 271 TypeScript tests passed across 11 files; application/MCP and strict harness
  typechecks passed. Production packaging verified 108 artifacts.
- Chromium cancelled all four jobs at checkpoint 240 in the same Worker; the
  subsequent cube and sphere matched their synchronous result buffers.
- All 35 input/output hashes in the synchronous analysis benchmark matched the
  preceding WASM artifact.

Warm alternating samples (two warmups, five samples) compare the whole owned
analysis call; cooperative includes both BVH and edges. They include ABI polling
but deliberately omit event-loop sleeps. Reproduce with
`node --import tsx benchmarks/cooperative-analysis.mts`.

| Sphere triangles | Synchronous median | Cooperative median | Checkpoints |
| ---: | ---: | ---: | ---: |
| 960 | 0.553 ms | 1.080 ms | 24 |
| 16,128 | 11.978 ms | 15.991 ms | 406 |
| 65,024 | 46.867 ms | 60.371 ms | 1,489 |

These local figures describe total suspension overhead, not the incremental cost
of this edge change or application-wide latency. The synchronous API remains
available for callers that do not need cancellation.
