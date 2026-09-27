# Cooperative BVH construction

The compiler can now observe cancellation during Rust BVH construction. It
keeps the initialized Worker and drops only the unfinished analysis job.
The existing coordinator can still terminate a Worker that cannot yield.

## Ownership and scheduling

`abi_solid_analysis_start` takes an owned render snapshot. The registry allows
two concurrent jobs and checks a 750,000 source-triangle limit before making
that snapshot. Job IDs increase monotonically and are never reused.

`abi_solid_analysis_step` polls a pinned future. BVH preparation, bounds scans,
quickselect partitions, node traversal and output ordering yield every 4,096
work units. A pending step returns zero; completion removes the job and returns
an ordinary array-result handle. This is a work budget, not a wall-clock bound:
allocations and the remaining synchronous stages can take longer.

The TypeScript adapter awaits the compiler checkpoint between steps. The
production checkpoint polls cancellation and returns to the event loop when
its 50 ms scheduling interval is due. Cancellation drops the future and all
owned scratch through `abi_solid_analysis_cancel` in `finally`. Completion
copies arrays into independent host buffers before freeing the result lease.
No WASM memory view or source-solid borrow survives an await.

The compiler uses this path whenever it has cancellation or yield callbacks.
Other callers retain the synchronous API. Its recursive traversal is preserved:
an initial shared asynchronous traversal slowed a native synchronous benchmark,
so only the cooperative path uses resumable traversal. Both share input records
and comparison rules. Frozen-reference differential checks cover both paths.

## Validation

See the scoped [qualification report](../qualification/cooperative-bvh-2026-09-27.json).
Reproduce the native byte comparison with:

```sh
rustc --edition 2024 -O benchmarks/cooperative-bvh-native.rs -o /tmp/cooperative-bvh-parity
/tmp/cooperative-bvh-parity
```

The frozen test fixture records the builder before this change. The differential
runner checks all bounds bits, nodes and triangle order across 122 cases,
including invalid/degenerate input and 750,000 triangles. Its timings are single
samples for diagnostics, not a throughput qualification.

After building the application, run `npm run test:cooperative-bvh` for the real
Chromium Worker check. It sends four cancellation messages after 80 checkpoints
inside the production analysis adapter, then completes a cube and a sphere in
the same Worker. Full results must match the synchronous buffers byte for byte.
The harness intentionally yields more often than the production scheduler to
make the message boundary deterministic; its cancellation latency is not an
application-wide latency guarantee. `npm run test:build-recovery` separately
checks the production Worker and editor recovery controls.

Local verification passed 12 Rust tests, 238 TypeScript tests, both application
typechecks, strict harness typechecking, and production packaging (108 artifacts).
The four browser cancellations occurred at checkpoint 80; the same Worker then
completed both fixtures with byte parity. All six production recovery scenarios
and the editor cancel/rebuild check also passed.

The separate synchronous WASM benchmark preserved all 35 input/output hashes.
Sphere-128 BVH medians were 3.592 ms before and 3.577 ms after; combined analysis
was 11.187 ms and 11.157 ms. These five-sample local runs show no material change
on that fixture, not a general speedup. Cooperative native construction took
421.7 ms versus 321.6 ms synchronous in one 750,000-triangle sample; suspension
has a cost and should be evaluated separately from responsiveness.

## Remaining work

Primitive/Boolean evaluation, volume/area metrics, render snapshot preparation,
semantic-edge extraction and selection surface grouping remain synchronous.
There is a checkpoint before edge extraction, but no checkpoint inside it.
Large allocations also remain synchronous. This closes the BVH portion of R1;
it does not complete cooperative cancellation throughout compilation or qualify
CUDA, MLX, tensor backends, or the entire release.

## Subsequent change

[Cooperative semantic-edge extraction](cooperative-edges-2026-09-27.md) removes
the synchronous edge stage described above. The qualification linked here
remains evidence for the earlier BVH-only artifact.
