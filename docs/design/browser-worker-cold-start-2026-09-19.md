# Browser worker cold initialization

## Reproduction and correction

CI 35475827899 reproduced a browser-worker assertion test receiving
`GeometryEngineUnavailableError` instead of its actual parser assertion. A local
test delaying the real kernel facade warmup by 350 ms reproduced that same failure
under the unchanged 250 ms provider-readiness deadline.

The display worker now warms `defaultGeometryKernel` in its existing initializing
phase before asking the engine to build. Initialization has its own 5,000 ms
Promise-race deadline, cleared on settlement. Cancellation and stale-job state
are checked immediately after warmup, before any parser call. Compiling progress
is emitted only after that checkpoint. Initialization errors are reported in the
initializing phase rather than mislabeled as compilation failures.

This does not raise the 250 ms provider deadline, retry a failed build, change
engine admission, or alter the exact-solid branch. The timer bounds asynchronous
waiting while the worker event loop is responsive; it cannot interrupt synchronous
WASM/JS execution or stop an already running native compile. The coordinator's
existing cancellation grace/worker termination remains the hard cancellation
boundary. A timeout does not claim that native compilation has been terminated.

## Verification

- Controlled real-kernel startup at both 0 and 350 ms reaches the actual parser
  assertion with exactly one correlated terminal event.
- A never-settling warmup fails at the initialization deadline. A late rejection
  produces no second terminal event or unhandled rejection, and no timer remains.
- Cancellation during initialization prevents the parser from being called.
  Mid-parse cancellation tests now explicitly await parser entry before sending
  cancellation, so they continue to test compilation rather than startup.
- 42 lifecycle, assertion, real-worker boundary and coordinator tests pass.
- UI type checking, Vite production build and dist verification pass: 89 artifacts,
  5,781,665 asset bytes plus 9,697,456 raw WASM bytes.
- Real Chrome production scenario passes source build, scene replacement,
  cancellation/recovery, generator checks and exact-solid geometry hashes.
  Report: `tmp/performance/browser-cold-start/report.json`. One streaming compile
  succeeded. This is functional verification, not a before/after speedup claim.

The in-process HeadlessGeometryService initialization path, unrelated CI timeouts,
and manifest/artifact qualification mismatches remain separate open work.
