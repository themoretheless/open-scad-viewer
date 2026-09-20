# Shared CAD worker and truss analysis

`computeTrussAnalysis` now uses the same warm worker as main-scene operations,
CAD changes and pair inspection. No second geometry realm is created solely for
analysis, and neither the worker client nor its protocol imports a runtime
geometry kernel on the main thread. Workbench load/support controls are still
pending; the new API is not yet exposed as a structural-analysis UI.

## Lifecycle

The previous unversioned response channel had no request ID, lost native error
codes and had no deadline or message-deserialization failure handling. It is now
split into a typed protocol, an injectable lifecycle client, an operation runtime
and the browser entry point. The existing convenience APIs retain their call
signatures, with the new `computeTrussAnalysis(model, {signal, timeoutMs})` API.

- Version, request ID and operation kind bind each response to its request.
- Successful truss responses require bounded matching array dimensions, finite
  numbers and admitted residuals. Sparse arrays are not accepted as numbers.
- Solver error names/codes survive transport, including `TRUSS_SINGULAR`.
  Recoverable operation failures keep the warm worker.
- Superseding calls, abort signals, timeout, crashes and malformed responses
  terminate the noncooperative worker. A later call creates a fresh realm.
- Settled callbacks cannot resolve or reject a later job. Old IDs are ignored;
  future IDs or a wrong operation/version are protocol errors.
- Cleanup removes handlers, abort listeners and deadlines on every terminal path.
- Default deadline: 30 seconds for truss, 120 seconds for existing CAD operations.
  Explicit client deadlines must be finite and between 1 and 120000 ms.
- There is no extra host-side copy of CAD meshes. `postMessage` performs the
  normal structured clone; the client retains only scalar response dimensions.
  Invalid deadlines/pre-aborted requests do not supersede valid active work.

Async kernel initialization belongs to the worker runtime. An overlapping
request during initialization receives `CAD_BUSY`; it cannot start a second
operation against partially initialized state. Cancellation still terminates
the realm rather than pretending to interrupt synchronous Rust in place.

## Verification

Focused client, runtime, real-worker and CAD/modeling tests: 61 passed.
Full Vitest: 3,360 passed and the same nine qualification-binding failures across
351 files in 109.22 seconds. The failures remain in `engineManifest` (1),
`g0ToolchainFingerprints` (6), `g1GithubActionsQualification` (1) and
`qualificationPlanArtifact` (1); archives were not rewritten. Full log:
`/private/tmp/osv-truss-worker-full-tests.log`.
After final host-validator/listener cleanup, the 61 focused tests were rerun and
passed. Vue/MCP/standalone-harness type checks pass. Final Vite build and
`verify-dist` pass: 92 artifacts, 5,946,072 asset bytes plus 9,729,072 raw WASM
bytes, totaling 15,675,144 bytes. Geometry WASM itself is unchanged this step.
The isolated Node worker loads the actual browser entry and real embedded WASM;
it exercises analytical truss extension, singular failure/recovery, 3 mm pair
clearance, box resizing and main-scene deletion in one reused worker. A separate
worker records entry into an infinite loop using shared memory, then is actually
terminated by the deadline; a subsequent real CAD worker solves correctly.

The production browser harness builds only its host adapter, then loads the
actual worker emitted into `dist/assets`. It tests analytic member forces on
4/43/125-node tripod fixtures, error preservation, worker reuse and abort/restart.
The finite solver work itself is not a timing-based cancellation proof; the
entered infinite-loop test above supplies that evidence.

Reproduce after `node_modules/.bin/vite build`:

```sh
CHROMIUM_EXECUTABLE='/Applications/Google Chrome Dev.app/Contents/MacOS/Google Chrome Dev' \
  node tools/browser-qualification/truss-worker.mjs
```

Two isolated Chrome 156.0.8063.3 headless runs on macOS measured full worker round
trips with 20 warmups and 31 samples per fixture. No tests/builds ran concurrently.
Correctness checks and fixture construction are outside timing.

| Nodes / free DOFs | p50 A / B, ms | p95 A / B, ms |
| --- | ---: | ---: |
| 4 / 3 | 0.10 / 0.10 | 0.10 / 0.20 |
| 43 / 120 | 0.90 / 0.90 | 1.30 / 1.40 |
| 125 / 366 | 12.60 / 13.20 | 14.20 / 15.20 |

Clock resolution limits the smallest values. Initial worker startup plus first
solve took 149.4 ms in both runs. During the warm workload, a main-thread
`requestAnimationFrame` loop advanced 39/42 times, with a maximum observed gap of
16.8 ms. This is a responsiveness check on an otherwise empty page, not full-scene
FPS, rendering/GPU performance, an arithmetic speedup or a cross-browser claim.

Reports: `/private/tmp/osv-truss-worker-final-{a,b}/report.json`.
Both runs used worker hash
`2648f02a02459270a327ce5180450962aceff6c19fbf694033c16b633113440a`
and geometry WASM hash
`329d16db9f7d27e20c9ec457d6da04508121ad517dac5961aa01eb98b9079580`.
The worker's algorithm did not change after these measurements; the host result
validator was subsequently tightened to reject empty-model success packets,
and abort cleanup now retains its original signal if the caller mutates options.

## Remaining Work

The worker still uses the embedded packed kernel. The existing optional network
compiler is installed in `src/main.ts`, not this worker, and its browser guard
currently excludes worker realms. Streaming raw WASM might reduce unpacking but
also adds a separate 7.7 MB request. Test controlled cold-cache, cached and slow
network cases before changing that tradeoff; the warm solve is unaffected.

Next integration work must establish explicit support/load and moment-resultant
semantics before exposing scenario controls. The old branch's false singular
success and moment-distribution logic must not be restored through the new API.

## Published WASM CI Follow-up

CI run 35485757963 on preceding commit `7ff1ee7c` completed both Node jobs:
3,334 passed, 9 failed, 5 skipped each. The nine failures are the same archive
bindings listed above; the four actual-WASM truss tests pass on both Node 20.19
and 22. The oracle two-process capture passes in 4599/5735 ms respectively,
confirming the earlier outer-deadline fix on a real CI run above five seconds.
Node job durations were 227.68/267.52 seconds. Logs are retained at
`/private/tmp/osv-ci-35485757963-node{20,22}.log`.
These jobs predate this worker integration and are not evidence for its CI status.
