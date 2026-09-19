# Disposable MCP worker cold-start phase

## Reproduction

The direct worker previously attached its request listener and sent `started`
before its first geometry-kernel compilation. The first provider readiness
probe therefore included cold initialization in its 250 ms deadline. In a
disposable worker, returning unavailable ends the job before late warmup can
restore service.

A real-worker fixture delays only the first cached `CadGeometryKernel.warm`
by 350 ms, then loads the unmodified Rust kernel and imports the production
worker entrypoint. Before the change, `cube(2)` deterministically failed with
`GeometryEngineUnavailableError: ... readiness check exceeded 250 ms`.
The fixture and delay are test-only; production has no delay/environment knob.

## Change

`directGeometry.worker.ts` now awaits the parser's existing kernel warmup before
attaching its request listener. Node queues the request while startup runs;
the existing `started` handshake is sent only after initialization completes.
The existing supervisor startup timer therefore covers module imports and
kernel warmup together. No new protocol event or timeout setting is introduced.

- Default startup deadline remains 5000 ms.
- Default complete-job deadline remains 30000 ms, including queue/startup time.
- Provider readiness still uses 250 ms; actual probe failure/quarantine rules
  are unchanged.
- Cancellation and startup expiration retain terminate-and-join ownership in
  the supervisor; a hung warmup cannot keep the worker alive indefinitely.
- Cold initialization exceptions now fail worker startup instead of being
  reported later through a provider readiness probe. They are not swallowed.

This does not share native handles or instances between disposable workers,
does not enable host I/O, and does not change browser or direct in-process
GeometryBuildEngine initialization. Unsupported/invalid source may now incur
cold startup before its worker-side planning refusal; source admission limits
in the parent remain unchanged. This is lifecycle correctness, not a claimed
reduction in compilation time.

## Verification

The fixture now passes both a real build (volume 8, area 24) and a capability
probe in separate disposable workers. Further real-worker cases prove that a
60000 ms artificial warmup is terminated/joined by a 1000 ms test startup limit,
and that cancellation before `started` also joins without quarantining the
supervisor or retaining an admitted job.

The combined supervisor, MCP stdio, unchanged no-I/O isolation and provider
readiness suites pass 52 tests. UI and MCP typechecks passed. Fake-timer
readiness tests continue to enforce the unchanged 250 ms policy.

The original remote Node22 CI failure is evidence for the failure class, not
proof that this local fix resolves every CI failure. It addresses disposable
MCP cold starts only. Direct in-process test contention and frozen-evidence
drift remain separate open items; the changed code has not run in remote CI.
