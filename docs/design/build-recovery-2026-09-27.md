# Geometry build cancellation and recovery

The editor can cancel an active build while retaining its last published scene.
Cancellation clears scheduled auto-build work before terminating the Worker, so
ending a parameter gesture cannot immediately restart the cancelled request.
Editing or pressing Render starts a new request normally.

`BuildCoordinator` owns silence detection and recovery. App configures 30 seconds
without a valid current-job Worker message. Repeated host requests do not count
as liveness. A timeout replaces the Worker and retries the original job once,
including its source digest, document revision, quality and selection-surface
option. Host elapsed time includes both attempts. A second timeout produces
`WORKER_TIMEOUT`. New jobs have independent retry allowances.

Silence and supersession timers capture tokens and Worker generations. Cleared
callbacks and old Worker messages cannot affect replacement work. Deserialization
errors settle immediately. If full geometry has already published, failures in
leftover preview work retire that work without replacing the full result.

## Verification

Run the production browser check after rebuilding the app:

```sh
npm run build
npm run test:build-recovery
```

The harness uses the production geometry Worker, the shared coordinator and a
fresh Chromium profile. It checks heavy-build supersession with the production
300 ms grace, forced hard preemption, user cancellation, bounded timeout retry,
new-revision recovery and disposal. The App smoke exercises its actual Cancel
build and Render buttons, checks retained scene state, and verifies cube volumes
before cancellation and after rebuilding.

Timeout cases deliberately wedge the event loop of a wrapper around the real
Worker and configure a 5-second silence deadline. They test host recovery from
silence; they do not establish which kernel calls can be interrupted. Reports
include events, publications, source/artifact hashes and browser version.
Failures preserve a diagnostic report and screenshot under the output directory.

Focused regression suite:

```sh
npx vitest run tests/buildCoordinator.test.ts tests/geometryWorker.test.ts \
  tests/geometryWorkerProtocol.test.ts tests/geometryWorkerRealBoundary.test.ts \
  tests/openscadParser.test.ts tests/autoBuildScheduler.test.ts \
  tests/buildPromotionPolicy.test.ts tests/geometryWorkerAssertion.test.ts
```

## Verified result — 2026-09-27

- 151 tests passed across the eight files above, including 42 coordinator tests.
- Vue and MCP TypeScript checks passed.
- `npm run build` passed; distribution verification checked 108 artifacts.
  Geometry and language WASM outputs remained byte-identical after rebuilding.
- Chromium 151.0.7922.34 passed all six coordinator scenarios and the App smoke.
  Warm supersession kept one Worker with no hard restart. Timeout injection
  retried once; repeated silence failed once and a new revision succeeded.
- The actual Source drawer retained one mesh after cancellation. The cancelled
  job published no success, and the next cube built with volume 27 after the
  initial cube with volume 8. No page errors were observed.
  This checks Source build state and geometry; it does not qualify GPU rendering
  or the separate Solid workspace.

[Browser report with artifact/source hashes](../qualification/build-recovery-2026-09-27.json).
The build still emits existing Rust warnings for `wgsl_export` naming and an
unused doc comment in `geometry-bridge/src/abi.rs`, plus Vite's packed-kernel
chunk-size warning. Distribution size checks passed; no budgets were increased.

## Remaining boundary

Individual synchronous Rust/WASM kernel, BVH and topology calls still cannot
receive cancellation messages while executing. Supersession can retain the warm
Worker when the evaluator reaches a checkpoint; otherwise the host terminates
it. Cooperative cancellation *inside* those kernel calls remains open. This work
does not close the general G0/G1 or B-rep release qualification matrices.

## Follow-up

[Cooperative BVH construction](cooperative-bvh-2026-09-27.md) subsequently added
resumable Rust analysis jobs. The evidence above describes the earlier artifact;
Boolean, render and topology calls still require the existing hard-preemption fallback.
